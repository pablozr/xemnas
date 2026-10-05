"""Check local Markdown destinations, without dependencies or network access.

Scope: tracked docs/**/*.md, README.md and AGENTS.md; inline/image links and
reference definitions. Fenced code and inline code are ignored. Not a CommonMark
parser: HTML, autolinks, nested link labels and multiline destinations are outside
the contract. Fragments and queries are stripped; anchors are not validated.
Targets must exist and belong to the tracked file set (directories need a tracked
descendant), so ignored/untracked artifacts cannot mask a broken clean clone.
"""

import argparse
from pathlib import Path
import re
import subprocess
from urllib.parse import unquote, urlsplit


def destinations(text):
    fence = None
    for number, line in enumerate(text.splitlines(), 1):
        marker = re.match(r"^\s{0,3}(`{3,}|~{3,})", line)
        if marker:
            token = marker.group(1)
            if fence is None:
                fence = token
            elif token[0] == fence[0] and len(token) >= len(fence):
                fence = None
            continue
        if fence:
            continue
        line = re.sub(r"(`+).*?\1", "", line)
        patterns = (r"!?\[[^\]\n]*\]\(\s*(<[^>]+>|[^\s)]+)",
                    r"^\s{0,3}\[[^\]]+\]:\s*(<[^>]+>|\S+)")
        for pattern in patterns:
            for match in re.finditer(pattern, line):
                yield number, match.group(1).strip("<>")


def check_file(root, source, tracked):
    errors = []
    count = 0
    for line, destination in destinations((root / source).read_text(encoding="utf-8")):
        url = urlsplit(destination)
        if url.scheme or url.netloc or not url.path:
            continue
        count += 1
        target = (root / source).parent.joinpath(unquote(url.path)).resolve()
        try:
            relative = target.relative_to(root.resolve()).as_posix()
        except ValueError:
            relative = None
        versioned = relative in tracked or (
            target.is_dir() and relative is not None
            and any(item.startswith(relative.rstrip("/") + "/") for item in tracked)
        )
        if not target.exists() or not versioned:
            errors.append(f"{source}:{line}: {destination} (missing or untracked target)")
    return count, errors


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--files", nargs="+", help="repository-relative Markdown sources")
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    tracked = set(subprocess.check_output(
        ["git", "ls-files", "-z"], cwd=root
    ).decode("utf-8").split("\0")) - {""}
    sources = sorted(args.files or [name for name in tracked if
                     name in ("README.md", "AGENTS.md") or
                     (name.startswith("docs/") and name.endswith(".md"))])
    errors = []
    count = 0
    for source in sources:
        if source not in tracked:
            errors.append(f"{source}: source is not tracked")
            continue
        checked, failures = check_file(root, source, tracked)
        count += checked
        errors.extend(failures)
    for error in errors:
        print(error)
    print(f"Checked {len(sources)} Markdown files, {count} local links; {len(errors)} errors.")
    return bool(errors)


if __name__ == "__main__":
    raise SystemExit(main())
