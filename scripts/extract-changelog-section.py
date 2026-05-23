#!/usr/bin/env python3
"""Extract a single version section from a Keep-a-Changelog file.

Usage: extract-changelog-section.py <version> <changelog-path>

Writes the body of the requested section (everything between the matching
header and the next header at the same level) to stdout. Exits non-zero if
the section is not found or the file is missing.
"""

from __future__ import annotations

import sys
from pathlib import Path

SECTION_PREFIX = "## ["


def extract(version: str, changelog: Path) -> str:
    if not changelog.exists():
        raise FileNotFoundError(f"changelog not found: {changelog}")

    header = f"{SECTION_PREFIX}{version}]"
    lines = changelog.read_text(encoding="utf-8").splitlines(keepends=True)
    out: list[str] = []
    in_section = False
    found = False

    for line in lines:
        stripped = line.lstrip()
        if stripped.startswith(header):
            in_section = True
            found = True
            continue
        if in_section and stripped.startswith(SECTION_PREFIX):
            break
        if in_section:
            out.append(line)

    if not found:
        raise LookupError(f"section [{version}] not present in {changelog}")

    while out and not out[0].strip():
        out.pop(0)
    while out and not out[-1].strip():
        out.pop()

    return "".join(out)


def main(argv: list[str]) -> int:
    if len(argv) != 3:
        sys.stderr.write(
            "usage: extract-changelog-section.py <version> <changelog-path>\n"
        )
        return 2
    version, path = argv[1], Path(argv[2])
    try:
        body = extract(version, path)
    except (FileNotFoundError, LookupError) as exc:
        sys.stderr.write(f"{exc}\n")
        return 1
    sys.stdout.write(body)
    if not body.endswith("\n"):
        sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
