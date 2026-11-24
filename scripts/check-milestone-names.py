#!/usr/bin/env python3
"""Fail if a milestone reference appears in a Rust/C++ identifier or string.

Milestones (``m<NN>`` / ``M<NN>``) belong in comments, docs, and specs only.
Comments are skipped; string literals and identifiers are checked, so
``m47_float_close``, ``reopenedM17`` and ``unknownM43Key`` all trip it while a
passing mention in a ``//`` comment does not.
"""

from __future__ import annotations

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
MILESTONE = re.compile(r"(?:^|[_-])[mM]\d|[a-z]M\d")
# Generated tables whose strings are product names, not milestones: Canon's
# EOS M50, M200 and M6 Mark II normalize to model keys like "M50" (#1574).
EXEMPT = {"crates/pictura-codec/src/dng/camera_matrices.rs"}


def violations(text: str):
    i, n, line = 0, len(text), 1
    while i < n:
        c = text[i]
        if c == "\n":
            line += 1
            i += 1
            continue
        if c == "/" and i + 1 < n and text[i + 1] == "/":
            while i < n and text[i] != "\n":
                i += 1
            continue
        if c == "/" and i + 1 < n and text[i + 1] == "*":
            i += 2
            while i + 1 < n and not (text[i] == "*" and text[i + 1] == "/"):
                if text[i] == "\n":
                    line += 1
                i += 1
            i += 2
            continue
        if c in "rRbB":
            j = i
            while j < n and text[j] in "rRbB":
                j += 1
            hashes = 0
            while j < n and text[j] == "#":
                hashes += 1
                j += 1
            if j < n and text[j] == '"':
                start = line
                j += 1
                close = '"' + "#" * hashes
                end = text.find(close, j)
                if end == -1:
                    break
                content = text[j:end]
                line += content.count("\n")
                if MILESTONE.search(content):
                    yield start, "string", content[:60]
                i = end + len(close)
                continue
        if c == '"':
            start = line
            i += 1
            buf: list[str] = []
            while i < n and text[i] != '"':
                if text[i] == "\\" and i + 1 < n:
                    buf.append(text[i + 1])
                    i += 2
                    continue
                if text[i] == "\n":
                    line += 1
                buf.append(text[i])
                i += 1
            content = "".join(buf)
            if MILESTONE.search(content):
                yield start, "string", content[:60]
            i += 1
            continue
        if c.isalpha() or c == "_":
            j = i
            while j < n and (text[j].isalnum() or text[j] == "_"):
                j += 1
            ident = text[i:j]
            if MILESTONE.search(ident):
                yield line, "identifier", ident
            i = j
            continue
        i += 1


def main() -> int:
    bad = 0
    for path in sorted((ROOT / "crates").rglob("*")):
        if path.suffix not in (".rs", ".cpp", ".h"):
            continue
        if path.relative_to(ROOT).as_posix() in EXEMPT:
            continue
        try:
            text = path.read_text()
        except OSError:
            continue
        for line, kind, token in violations(text):
            print(f"{path.relative_to(ROOT)}:{line}: {kind} '{token}'")
            bad += 1
    if bad:
        print(f"check-milestone-names: FAILED ({bad} milestone name(s))")
        return 1
    print("check-milestone-names: OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
