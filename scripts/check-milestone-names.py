#!/usr/bin/env python3
"""Fail if a milestone reference appears in a Rust/C++ identifier or string.

Milestones (``m<NN>`` / ``M<NN>``) belong in comments, docs, and specs only.
Comments are skipped; string literals and identifiers are checked, so
``m47_float_close``, ``reopenedM17`` and ``unknownM43Key`` all trip it while a
passing mention in a ``//`` comment does not.

Every finding is local to one file, so the check can scan only the changed crate
sources: `--changed FILE` takes the NUL-separated list from
`scripts/changed_files.py`, and an empty list falls back to the whole tree.
`--full` (the default) always scans everything, which is what CI uses. A
whole-file `[mM]<digit>` prefilter skips the
lexer on the files that cannot hold a milestone; it is a superset of the
milestone pattern, so it changes no result.

Run from the repository root: python3 scripts/check-milestone-names.py [--changed FILE] [--full] [--self-test] [--root DIR]
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from changed_files import scoped_paths  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
MILESTONE = re.compile(r"(?:^|[_-])[mM]\d|[a-z]M\d")
# The milestone patterns all contain `m`/`M` followed by a digit; a file without
# one cannot hold a finding, so it never reaches the lexer.
PREFILTER = re.compile(r"[mM]\d")
SUFFIXES = (".rs", ".cpp", ".h")
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


def scan(root: Path, paths: list[Path]) -> int:
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
        if not PREFILTER.search(text):
            continue
        for line, kind, token in violations(text):
            print(f"{path.relative_to(root)}:{line}: {kind} '{token}'")
            bad += 1
    return bad


def all_paths(root: Path) -> list[Path]:
    return sorted(
        path for path in (root / "crates").rglob("*") if path.suffix in SUFFIXES
    )


def check(root: Path = ROOT, changed: str | None = None) -> int:
    selected = scoped_paths(
        lambda rel: rel.startswith("crates/") and rel.endswith(SUFFIXES), changed
    )
    if selected is None:
        paths = all_paths(root)
    else:
        paths = [root / rel for rel in selected if (root / rel).is_file()]
    bad = scan(root, paths)
    if bad:
        print(f"check-milestone-names: FAILED ({bad} milestone name(s))")
        return 1
    print("check-milestone-names: OK")
    return 0


def _self_test() -> int:
    import tempfile

    assert list(violations("let m47_float_close = 1;")) == [
        (1, "identifier", "m47_float_close")
    ]
    assert list(violations("// m47 in a comment\n")) == []
    assert list(violations('let s = "unknownM43Key";')) == [(1, "string", "unknownM43Key")]
    assert list(violations('let r = r#"reopenedM17"#;')) == [(1, "string", "reopenedM17")]
    assert PREFILTER.search('"unknownM43Key"')
    assert not PREFILTER.search("fn clean_name() {}")

    import contextlib
    import io

    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        (root / "crates").mkdir()
        (root / "crates" / "clean.rs").write_text("fn clean() {}\n", encoding="utf-8")
        (root / "crates" / "bad.rs").write_text("let m47_x = 1;\n", encoding="utf-8")
        changed = root / "changed.txt"
        with contextlib.redirect_stdout(io.StringIO()):
            changed.write_text("crates/clean.rs\n", encoding="utf-8")
            assert check(root, str(changed)) == 0, "a scoped scan flagged an unchanged file"
            changed.write_text("crates/clean.rs\ncrates/bad.rs\n", encoding="utf-8")
            assert check(root, str(changed)) == 1, "a scoped scan missed a changed violation"
            changed.write_text("", encoding="utf-8")
            assert check(root, str(changed)) == 1, "an empty list skipped the full scan"
            assert check(root) == 1, "the full scan missed a violation"
        (root / "crates" / "bad.rs").write_text("// m47 only in a comment\n", encoding="utf-8")
        with contextlib.redirect_stdout(io.StringIO()):
            assert check(root) == 0, "the full scan flagged a comment"
    print("check-milestone-names: self-test OK")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--changed", default=None, help="list of changed paths to scan")
    parser.add_argument("--full", action="store_true", help="scan the whole tree (default)")
    parser.add_argument("--root", default=None, help="scan this directory instead of the repository")
    args = parser.parse_args()
    if args.self_test:
        return _self_test()
    root = Path(args.root) if args.root else ROOT
    return check(root, None if args.full else args.changed)


if __name__ == "__main__":
    sys.exit(main())
