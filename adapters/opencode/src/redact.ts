//! Default redaction, bounding and diff-hunk policy.
//!
//! Applied before anything is put into an envelope (MVP-SPEC §7.1, §13):
//!
//! - reasoning/thinking parts are dropped entirely;
//! - common secret shapes are masked with `[REDACTED]`;
//! - each artifact content is bounded to a configurable byte budget without
//!   splitting a UTF-8 code point;
//! - `FileDiff` `before`/`after` snapshots are reduced to bounded unified-diff
//!   hunks by a line-level LCS.
//!
//! Nothing here interprets content: it only removes, masks or summarizes it.
//! The raw, unbounded, unredacted text never leaves this module.

import { stripContextBlocks } from "./context.js";
import type { OpenCodeFileDiff, OpenCodePart } from "./opencode.js";

const REDACTED = "[REDACTED]";

/**
 * Secret shapes masked by default.
 *
 * Kept deliberately conservative: known key prefixes, PEM private-key blocks
 * and `KEY=value`/`KEY: value` assignment lines for common credential names.
 */
const SECRET_PATTERNS: ReadonlyArray<readonly [RegExp, string]> = [
  [
    /-----BEGIN [A-Z0-9 ]*PRIVATE KEY-----[\s\S]*?-----END [A-Z0-9 ]*PRIVATE KEY-----/g,
    REDACTED,
  ],
  [
    /^(\s*(?:TOKEN|API[_-]?KEY|SECRET|PASSWORD|ACCESS[_-]?KEY|AUTHORIZATION)\s*[:=]\s*)\S.*$/gim,
    `$1${REDACTED}`,
  ],
  [/\bsk-[A-Za-z0-9_-]{8,}\b/g, REDACTED],
  [/\bghp_[A-Za-z0-9]{20,}\b/g, REDACTED],
  [/\bgithub_pat_[A-Za-z0-9_]{20,}\b/g, REDACTED],
  [/\bxox[baprs]-[A-Za-z0-9-]{10,}\b/g, REDACTED],
];

/** Masks common secret shapes in `text`. */
export function redactSecrets(text: string): string {
  let redacted = text;
  for (const [pattern, replacement] of SECRET_PATTERNS) {
    redacted = redacted.replace(pattern, replacement);
  }
  return redacted;
}

/**
 * Truncates `value` to at most `maxBytes` UTF-8 bytes.
 *
 * The cut never splits a code point: the result is the longest prefix whose
 * UTF-8 encoding fits, without a trailing lone surrogate.
 */
export function truncateUtf8(value: string, maxBytes: number): string {
  if (maxBytes <= 0) {
    return "";
  }
  const encoder = new TextEncoder();
  if (encoder.encode(value).length <= maxBytes) {
    return value;
  }
  let low = 0;
  let high = value.length;
  while (low < high) {
    const mid = Math.ceil((low + high) / 2);
    if (encoder.encode(value.slice(0, mid)).length <= maxBytes) {
      low = mid;
    } else {
      high = mid - 1;
    }
  }
  let result = value.slice(0, low);
  if (result.length > 0) {
    const last = result.charCodeAt(result.length - 1);
    if (last >= 0xd800 && last <= 0xdbff) {
      result = result.slice(0, -1);
    }
  }
  return result;
}

/** Drops reasoning/thinking parts; other parts pass through unchanged. */
export function omitReasoningParts(parts: OpenCodePart[]): OpenCodePart[] {
  return parts.filter(
    (part) => part.type !== "reasoning" && part.type !== "thinking",
  );
}

/** Joins the `text` of every text part, preserving order. */
export function collectText(parts: OpenCodePart[]): string {
  return parts
    .filter((part) => part.type === "text" && typeof part.text === "string")
    .map((part) => part.text ?? "")
    .join("\n");
}

/** Drops injected context blocks, redacts, then bounds `content`. */
export function boundContent(content: string, maxBytes: number): string {
  return truncateUtf8(redactSecrets(stripContextBlocks(content)), maxBytes);
}

/** Context lines kept around each change when formatting a hunk. */
const DIFF_CONTEXT_LINES = 3;

/**
 * Above this many LCS cells, the exact line diff is skipped in favour of a
 * bounded prefix/suffix diff, so a huge file cannot blow up time or memory.
 */
const MAX_LCS_CELLS = 4_000_000;

interface LineOp {
  kind: "equal" | "remove" | "add";
  text: string;
}

function splitDiffLines(value: string): string[] {
  if (value.length === 0) {
    return [];
  }
  return (value.endsWith("\n") ? value.slice(0, -1) : value).split("\n");
}

/** Exact line diff via a longest-common-subsequence table. */
function lcsLineOps(before: string[], after: string[]): LineOp[] {
  const columns = after.length + 1;
  const lengths = new Uint32Array((before.length + 1) * columns);
  for (let i = before.length - 1; i >= 0; i--) {
    for (let j = after.length - 1; j >= 0; j--) {
      lengths[i * columns + j] =
        before[i] === after[j]
          ? lengths[(i + 1) * columns + (j + 1)] + 1
          : Math.max(
              lengths[(i + 1) * columns + j],
              lengths[i * columns + (j + 1)],
            );
    }
  }
  const ops: LineOp[] = [];
  let i = 0;
  let j = 0;
  while (i < before.length && j < after.length) {
    if (before[i] === after[j]) {
      ops.push({ kind: "equal", text: before[i] });
      i++;
      j++;
    } else if (lengths[(i + 1) * columns + j] >= lengths[i * columns + (j + 1)]) {
      ops.push({ kind: "remove", text: before[i] });
      i++;
    } else {
      ops.push({ kind: "add", text: after[j] });
      j++;
    }
  }
  while (i < before.length) {
    ops.push({ kind: "remove", text: before[i] });
    i++;
  }
  while (j < after.length) {
    ops.push({ kind: "add", text: after[j] });
    j++;
  }
  return ops;
}

/** Bounded fallback diff: trim the common prefix/suffix and summarize the rest. */
function coarseLineOps(before: string[], after: string[]): LineOp[] {
  let start = 0;
  while (
    start < before.length &&
    start < after.length &&
    before[start] === after[start]
  ) {
    start++;
  }
  let beforeEnd = before.length;
  let afterEnd = after.length;
  while (
    beforeEnd > start &&
    afterEnd > start &&
    before[beforeEnd - 1] === after[afterEnd - 1]
  ) {
    beforeEnd--;
    afterEnd--;
  }
  const ops: LineOp[] = [];
  for (let i = 0; i < start; i++) {
    ops.push({ kind: "equal", text: before[i] });
  }
  for (let i = start; i < beforeEnd; i++) {
    ops.push({ kind: "remove", text: before[i] });
  }
  for (let j = start; j < afterEnd; j++) {
    ops.push({ kind: "add", text: after[j] });
  }
  for (let i = beforeEnd; i < before.length; i++) {
    ops.push({ kind: "equal", text: before[i] });
  }
  return ops;
}

function countHunkLines(
  ops: LineOp[],
  start: number,
  end: number,
): { before: number; after: number } {
  let before = 0;
  let after = 0;
  for (let i = start; i <= end; i++) {
    if (ops[i].kind !== "add") {
      before++;
    }
    if (ops[i].kind !== "remove") {
      after++;
    }
  }
  return { before, after };
}

/** Formats the op list as unified-diff hunks with bounded context. */
function formatHunks(ops: LineOp[]): string {
  const ranges: Array<[number, number]> = [];
  for (let i = 0; i < ops.length; i++) {
    if (ops[i].kind === "equal") {
      continue;
    }
    const start = Math.max(0, i - DIFF_CONTEXT_LINES);
    const end = Math.min(ops.length - 1, i + DIFF_CONTEXT_LINES);
    const last = ranges[ranges.length - 1];
    if (last !== undefined && start <= last[1] + 1) {
      last[1] = Math.max(last[1], end);
    } else {
      ranges.push([start, end]);
    }
  }

  let output = "";
  let beforeLine = 1;
  let afterLine = 1;
  let rangeIndex = 0;
  for (let i = 0; i < ops.length; i++) {
    const range = ranges[rangeIndex];
    if (range !== undefined && range[0] === i) {
      const { before, after } = countHunkLines(ops, range[0], range[1]);
      output += `@@ -${beforeLine},${before} +${afterLine},${after} @@\n`;
    }
    const op = ops[i];
    if (range !== undefined && i >= range[0] && i <= range[1]) {
      output +=
        op.kind === "equal"
          ? ` ${op.text}\n`
          : op.kind === "remove"
            ? `-${op.text}\n`
            : `+${op.text}\n`;
    }
    if (op.kind !== "add") {
      beforeLine++;
    }
    if (op.kind !== "remove") {
      afterLine++;
    }
    if (range !== undefined && i === range[1]) {
      rangeIndex++;
    }
  }
  return output;
}

/**
 * Builds unified-diff-style hunks from an OpenCode `FileDiff` `before`/`after`
 * pair. Returns `""` when there is no textual line change.
 */
export function buildDiffHunks(before: string, after: string): string {
  const beforeLines = splitDiffLines(before);
  const afterLines = splitDiffLines(after);
  if (beforeLines.length === 0 && afterLines.length === 0) {
    return "";
  }
  const ops =
    beforeLines.length * afterLines.length > MAX_LCS_CELLS
      ? coarseLineOps(beforeLines, afterLines)
      : lcsLineOps(beforeLines, afterLines);
  if (!ops.some((op) => op.kind !== "equal")) {
    return "";
  }
  return formatHunks(ops);
}

/** `file` relative to `directory`, with forward slashes. */
export function relativePath(file: string, directory: string): string {
  const normalized = file.replace(/\\/g, "/");
  const base = directory.replace(/\\/g, "/").replace(/\/+$/, "");
  if (base.length > 0 && normalized.toLowerCase().startsWith(`${base.toLowerCase()}/`)) {
    return normalized.slice(base.length + 1);
  }
  return normalized.replace(/^\/+/, "");
}

/**
 * Reduces the unified patches of edit tools to bounded `diff --git` blocks.
 *
 * Keeps the hunks (from the first `@@`) under a `diff --git a/<file> b/<file>`
 * header, so the app reads file paths the same way as for session diffs.
 * Redaction and truncation are the same as [`reduceDiffs`].
 */
export function reducePatches(
  patches: Array<{ file: string; diff: string }>,
  directory: string,
  maxBytes: number,
): Array<{ file: string; content: string }> {
  const reduced: Array<{ file: string; content: string }> = [];
  let remaining = maxBytes;
  for (const patch of patches) {
    if (remaining <= 0) {
      break;
    }
    const start = patch.diff.indexOf("@@");
    if (start < 0) {
      continue;
    }
    const file = relativePath(patch.file, directory);
    if (file.length === 0) {
      // Without a file name the hunk cannot be tied to a component.
      continue;
    }
    const header = `diff --git a/${file} b/${file}\n`;
    const block = boundContent(`${header}${patch.diff.slice(start)}`, remaining);
    if (block.length === 0) {
      continue;
    }
    reduced.push({ file, content: block });
    remaining -= Buffer.byteLength(block, "utf8");
  }
  return reduced;
}

/**
 * Reduces diffs to bounded hunks.
 *
 * Each changed file becomes one `diff --git` block containing its generated
 * hunks, redacted and truncated. Files are added in order until `maxBytes` of
 * total diff content is reached, so an oversized diff is reduced rather than
 * dropped.
 */
export function reduceDiffs(
  diffs: OpenCodeFileDiff[],
  maxBytes: number,
): Array<{ file: string; content: string }> {
  const reduced: Array<{ file: string; content: string }> = [];
  let remaining = maxBytes;
  for (const diff of diffs) {
    if (remaining <= 0) {
      break;
    }
    const hunks = buildDiffHunks(diff.before, diff.after);
    if (hunks.length === 0) {
      continue;
    }
    const header = `diff --git a/${diff.file} b/${diff.file}\n`;
    const block = boundContent(`${header}${hunks}`, remaining);
    if (block.length === 0) {
      continue;
    }
    reduced.push({ file: diff.file, content: block });
    remaining -= Buffer.byteLength(block, "utf8");
  }
  return reduced;
}
