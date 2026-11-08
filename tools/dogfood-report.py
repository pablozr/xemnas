#!/usr/bin/env python3
"""Fill the dogfood diary from the local app database (read-only, stdlib only).

Usage:
  python tools/dogfood-report.py            print the block to stdout
  python tools/dogfood-report.py --write    rewrite the block in the diary
  --db PATH   database (default: $XEMNAS_DATA_DIR or %LOCALAPPDATA%/xemnas, then state/app.db)
  --doc PATH  diary (default: docs/operacao/dogfood-log.md)
  --outbox PATH  capture outbox (default: `outbox/` next to the database's `state/`)

The block between `<!-- dogfood:auto:start -->` and `<!-- dogfood:auto:end -->`
is regenerated from scratch on every run, so running it once or ten times gives
the same file. Only numeric aggregates are written: never questions, choices,
paths, project names or ids.

Metric definitions follow the in-app Diagnostics (storage-sqlite/src/diagnostics.rs):
latency is candidate `created_at` minus capture `received_at`; review time is
decision `confirmed_at` minus candidate `created_at`; noise is dismissed over
decided (accepted, edited_and_accepted, dismissed); losses are failed jobs and
failed/skipped assessments. Rejected counts the files in `outbox/rejected/` per local
day of their `rejected_at` (captures the app refused, never in the database); the line
below the table gives the outbox right now: pending files (waiting for the app to open,
oldest by `observed_at`, else file mtime) and rejected files per reason code. Only
counts and codes are read from the outbox; the quarantine (`unregistered/`) adds
"Aguardando registro: <folder name> (N)" per root. Percentiles use the same nearest-rank rule
(index = round((n - 1) * p)). Days are local-time days; the database stores UTC.
"""

import argparse
import json
import os
import sqlite3
import sys
from collections import defaultdict
from datetime import datetime
from pathlib import Path

START = "<!-- dogfood:auto:start -->"
END = "<!-- dogfood:auto:end -->"
DEFAULT_DOC = Path(__file__).resolve().parent.parent / "docs" / "operacao" / "dogfood-log.md"
DASH = "—"

HEADER = [
    "Dia", "Capturas", "Candidatos", "Confirmados", "Descartados", "Ruído",
    "Auto", "Desfeitos", "Latência p50/p95", "Revisão p50/p95",
    "Injeções", "Tokens", "Omitidos", "MCP", "MCP respondidas", "Rejeitadas", "Perdas",
]


def default_db():
    root = os.environ.get("XEMNAS_DATA_DIR") or os.path.join(
        os.environ.get("LOCALAPPDATA") or os.path.expanduser("~"), "xemnas"
    )
    return Path(root) / "state" / "app.db"


def parse(stamp):
    """RFC3339 text to an aware datetime, or None when unparseable."""
    try:
        return datetime.fromisoformat(stamp.replace("Z", "+00:00"))
    except (ValueError, AttributeError):
        return None


def day_of(stamp, tz):
    moment = parse(stamp)
    if moment is None:
        return None
    if moment.tzinfo is None:
        moment = moment.replace(tzinfo=datetime.now().astimezone().tzinfo)
    return moment.astimezone(tz).date().isoformat()


def percentile(values, fraction):
    ordered = sorted(values)
    return ordered[int((len(ordered) - 1) * fraction + 0.5)]


def human(ms):
    seconds = ms / 1000
    if seconds < 60:
        return f"{seconds:.0f} s"
    if seconds < 3600:
        return f"{seconds / 60:.1f} min"
    return f"{seconds / 3600:.1f} h"


def spread(values):
    if not values:
        return DASH
    return f"{human(percentile(values, 0.5))} / {human(percentile(values, 0.95))}"


def has_table(db, name):
    row = db.execute("SELECT 1 FROM sqlite_master WHERE type='table' AND name=?", (name,))
    return row.fetchone() is not None


def collect(db, tz):
    """Per-day aggregates keyed by local date."""
    days = defaultdict(lambda: defaultdict(int))
    lists = defaultdict(lambda: defaultdict(list))

    def count(sql, key, amount_column=False):
        for row in db.execute(sql):
            day = day_of(row[0], tz)
            if day:
                days[day][key] += row[1] if amount_column else 1

    count("SELECT received_at FROM capture_receipts", "captures")
    count("SELECT created_at FROM decision_candidates", "created")
    count("SELECT updated_at FROM decision_candidates "
          "WHERE status IN ('accepted','edited_and_accepted')", "accepted")
    count("SELECT updated_at FROM decision_candidates WHERE status = 'dismissed'", "dismissed")
    count("SELECT created_at FROM auto_reviews WHERE verdict IN ('accepted','discarded')", "auto")
    count("SELECT undone_at FROM auto_reviews WHERE undone_at IS NOT NULL", "undone")
    count("SELECT created_at FROM context_injections", "injections")
    count("SELECT created_at, tokens FROM context_injections", "tokens", True)
    count("SELECT created_at, omitted FROM context_injections", "omitted", True)
    count("SELECT updated_at FROM jobs WHERE state = 'failed'", "lost")
    count("SELECT finished_at FROM assessments WHERE outcome IN ('failed','skipped')", "lost")

    latency = (
        "SELECT dc.created_at, r.received_at FROM decision_candidates dc "
        "JOIN capture_receipts r ON r.capture_id = dc.capture_id"
    )
    review = (
        "SELECT d.confirmed_at, dc.created_at FROM engineering_decisions d "
        "JOIN decision_candidates dc ON dc.id = d.candidate_id"
    )
    for key, sql in (("latency", latency), ("review", review)):
        for end, start in db.execute(sql):
            end_at, start_at = parse(end), parse(start)
            day = day_of(end, tz)
            if end_at and start_at and day:
                lists[day][key].append(int((end_at - start_at).total_seconds() * 1000))

    if has_table(db, "agent_queries"):
        for stamp, outcome in db.execute("SELECT created_at, outcome FROM agent_queries"):
            day = day_of(stamp, tz)
            if day:
                days[day]["mcp"] += 1
                days[day]["mcp_answered"] += outcome == "answered"
    return days, lists


def outbox_files(outbox, folder):
    """The `*.json` files of an outbox folder, or [] when it does not exist."""
    try:
        return sorted(Path(outbox, folder).glob("*.json"))
    except OSError:
        return []


def read_json(path):
    try:
        data = json.loads(path.read_text(encoding="utf-8-sig"))
    except (OSError, ValueError):
        return {}
    return data if isinstance(data, dict) else {}


def count_rejected(outbox, tz, days):
    """Rejected files per local day of `rejected_at`; unreadable ones have no day."""
    for path in outbox_files(outbox, "rejected"):
        stamp = read_json(path).get("rejected_at")
        day = day_of(stamp, tz) if isinstance(stamp, str) else None
        if day:
            days[day]["rejected"] += 1


def oldest_pending(files, tz):
    """Local date of the oldest pending capture: `observed_at`, else the file mtime."""
    dates = []
    for path in files:
        stamp = read_json(path).get("observed_at")
        day = day_of(stamp, tz) if isinstance(stamp, str) else None
        if day is None:
            try:
                day = datetime.fromtimestamp(path.stat().st_mtime, tz).date().isoformat()
            except OSError:
                continue
        dates.append(day)
    return min(dates) if dates else None


def outbox_line(outbox, tz):
    """One summary line of the outbox right now, or None when there is no outbox."""
    if not Path(outbox).is_dir():
        return None
    pending = outbox_files(outbox, "pending")
    codes = defaultdict(int)
    for path in outbox_files(outbox, "rejected"):
        code = read_json(path).get("code")
        codes[code if isinstance(code, str) and code else "desconhecido"] += 1
    oldest = oldest_pending(pending, tz)
    since = f" (mais antiga de {oldest})" if oldest else ""
    reasons = ", ".join(f"{code}={n}" for code, n in sorted(codes.items())) or "nenhuma"
    rejected = sum(codes.values())
    return (f"Outbox agora: {len(pending)} pendentes{since} e {rejected} rejeitadas "
            f"por motivo: {reasons}.{quarantine_text(outbox)}")


def quarantine_text(outbox):
    """` Aguardando registro: <pasta> (N), ...` from `unregistered/index.json`, or ''.

    Only the last path component of each root and a count are written, never the
    whole path; the app keeps the captures intact until the folder is registered."""
    index = read_json(Path(outbox, "unregistered", "index.json"))
    counts = defaultdict(int)
    for entry in index.values():
        root = entry.get("root") if isinstance(entry, dict) else None
        if isinstance(root, str) and root:
            counts[root.replace(chr(92), "/").rstrip("/").rsplit("/", 1)[-1] or root] += 1
    if not counts:
        return ""
    ranked = sorted(counts.items(), key=lambda item: (-item[1], item[0]))
    return " Aguardando registro: " + ", ".join(f"{name} ({n})" for name, n in ranked) + "."


def render(db, tz, now, outbox=None):
    days, lists = collect(db, tz)
    if outbox is not None:
        count_rejected(outbox, tz, days)
    has_mcp = has_table(db, "agent_queries")
    # A day exists only when something happened in it.
    active = sorted(day for day, data in days.items() if any(data.values()))
    rows = []
    totals = defaultdict(int)
    all_latency, all_review = [], []
    for day in active:
        data, series = days[day], lists[day]
        decided = data["accepted"] + data["dismissed"]
        noise = f"{data['dismissed'] / decided:.0%}" if decided else DASH
        rows.append([
            day, data["captures"], data["created"], data["accepted"], data["dismissed"], noise,
            data["auto"], data["undone"], spread(series["latency"]), spread(series["review"]),
            data["injections"], data["tokens"], data["omitted"],
            data["mcp"] if has_mcp else DASH, data["mcp_answered"] if has_mcp else DASH,
            data["rejected"], data["lost"],
        ])
        for key, value in data.items():
            totals[key] += value
        all_latency += series["latency"]
        all_review += series["review"]

    decided = totals["accepted"] + totals["dismissed"]
    noise = f"{totals['dismissed'] / decided:.0%}" if decided else DASH
    total_row = [
        f"**Total ({len(active)} dia{'s' if len(active) != 1 else ''})**", totals["captures"], totals["created"],
        totals["accepted"], totals["dismissed"], noise, totals["auto"], totals["undone"],
        spread(all_latency), spread(all_review), totals["injections"], totals["tokens"],
        totals["omitted"], totals["mcp"] if has_mcp else DASH,
        totals["mcp_answered"] if has_mcp else DASH, totals["rejected"], totals["lost"],
    ]
    lines = [START, "", "| " + " | ".join(HEADER) + " |", "|" + " --- |" * len(HEADER)]
    for row in rows + ([total_row] if rows else []):
        lines.append("| " + " | ".join(str(cell) for cell in row) + " |")
    if not rows:
        lines.append("| " + " | ".join([DASH] * len(HEADER)) + " |")
    summary = outbox_line(outbox, tz) if outbox is not None else None
    if summary:
        lines += ["", summary]
    lines += ["", f"Atualizado em {now.strftime('%Y-%m-%d %H:%M')} (hora local).", "", END]
    return "\n".join(lines)


def open_readonly(path):
    return sqlite3.connect(f"file:{Path(path).resolve().as_posix()}?mode=ro", uri=True)


def replace_block(text, block):
    start, end = text.find(START), text.find(END)
    if start < 0 or end < start:
        raise ValueError(f"marcadores {START} e {END} não encontrados no documento")
    return text[:start] + block + text[end + len(END):]


def write_doc(doc, block):
    raw = Path(doc).read_bytes().decode("utf-8-sig")
    newline = "\r\n" if "\r\n" in raw else "\n"
    text = replace_block(raw.replace("\r\n", "\n"), block)
    Path(doc).write_bytes(text.replace("\n", newline).encode("utf-8"))


def main(argv=None):
    parser = argparse.ArgumentParser(description="Preenche o diário de dogfood.")
    parser.add_argument("--write", action="store_true", help="atualiza o documento")
    parser.add_argument("--db", type=Path, default=None)
    parser.add_argument("--doc", type=Path, default=DEFAULT_DOC)
    parser.add_argument("--outbox", type=Path, default=None)
    args = parser.parse_args(argv)
    sys.stdout.reconfigure(encoding="utf-8")
    sys.stderr.reconfigure(encoding="utf-8")

    db_path = args.db or default_db()
    if not db_path.is_file():
        print(f"Banco não encontrado: {db_path}", file=sys.stderr)
        return 2
    if args.write:
        try:
            replace_block(args.doc.read_text(encoding="utf-8-sig"), "")
        except (OSError, ValueError) as error:
            print(f"Documento inválido: {error}", file=sys.stderr)
            return 2

    outbox = args.outbox or db_path.parent.parent / "outbox"
    now = datetime.now().astimezone()
    try:
        db = open_readonly(db_path)
        try:
            block = render(db, now.tzinfo, now, outbox)
        finally:
            db.close()
    except sqlite3.Error as error:
        print(f"Não foi possível ler o banco: {error}", file=sys.stderr)
        return 2

    if args.write:
        write_doc(args.doc, block)
        print(f"Atualizado: {args.doc}")
    else:
        print(block)
    return 0


if __name__ == "__main__":
    sys.exit(main())
