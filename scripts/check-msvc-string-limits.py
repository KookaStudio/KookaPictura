#!/usr/bin/env python3
"""Reject C++ string literals longer than MSVC's per-literal limit.

MSVC caps a single string literal at 16,380 single-byte characters, applied
*before* adjacent literals are concatenated (error C2026). GCC and Clang accept
far longer literals, so an oversized literal compiles fine on Linux/macOS and
only fails on the Windows release job. This check runs on Linux and fails at
16,000 bytes -- below MSVC's cap -- so the breakage is caught before a Windows
runner is needed.

Every string-literal token under `crates/pictura-app/cpp` is measured: raw
`R"delim(...)delim"` literals by their body, ordinary literals by their source
bytes (a conservative upper bound, since escape sequences only shrink). Wide
literals (`L`, `u`, `U`) count the same bytes but MSVC's cap is about half (or a
quarter for `char32_t`), so they are held to a correspondingly lower length.
Comments and character literals are skipped, so a quote inside a comment is not
a literal.

Each finding is local to one file, so `--changed FILE` (the list from
`scripts/changed_files.py`) scans only the changed C++ sources; an empty list
falls back to the whole tree, and `--full` (the default, what CI uses) always
scans everything.

Run `python3 scripts/check-msvc-string-limits.py --self-test` to exercise the
lexer and the scoped fallback.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path
from typing import Iterator

sys.path.insert(0, str(Path(__file__).resolve().parent))
from changed_files import scoped_paths  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
SRC = "crates/pictura-app/cpp"
SUFFIXES = (".cpp", ".h")

# MSVC's documented cap is 16,380 bytes; fail below it for a small margin.
LIMIT = 16000

_RAW_RE = re.compile(r'(?<![\w])((?:u8|u|U|L)?)R"([^()\s\\]{0,16})\(')
_STR_RE = re.compile(r'(?<![\w])((?:u8|u|U|L)?)"')

# The MSVC cap counts single-byte characters, so a wide literal holds roughly
# 1/width as many before hitting C2026.
_WIDTH = {"": 1, "u8": 1, "u": 2, "L": 2, "U": 4}


def _bytes(text: str, start: int, end: int) -> int:
    return len(text[start:end].encode("utf-8"))


def literals(text: str) -> Iterator[tuple[int, int, int, str]]:
    """Yield (line, byte_length, width, kind) for each string-literal token."""
    i = 0
    n = len(text)
    line = 1
    while i < n:
        c = text[i]
        if c == "\n":
            line += 1
            i += 1
            continue
        if c == "/" and text.startswith("//", i):
            j = text.find("\n", i)
            i = n if j == -1 else j
            continue
        if c == "/" and text.startswith("/*", i):
            j = text.find("*/", i + 2)
            if j == -1:
                return
            line += text.count("\n", i, j)
            i = j + 2
            continue
        if c == "'":
            # A char literal, or a C++14 digit separator (1'000) -- bail out of
            # the latter at the newline so it cannot swallow later code.
            j = i + 1
            while j < n and text[j] != "'" and text[j] != "\n":
                if text[j] == "\\":
                    j += 1
                j += 1
            if j < n and text[j] == "'":
                line += text.count("\n", i, j)
                i = j + 1
            else:
                i += 1
            continue
        m = _RAW_RE.match(text, i)
        if m:
            start = m.end()
            close = ")" + m.group(2) + '"'
            end = text.find(close, start)
            if end == -1:
                end = n
            yield line, _bytes(text, start, end), _WIDTH[m.group(1)], "raw"
            line += text.count("\n", i, end)
            i = end + len(close)
            continue
        m = _STR_RE.match(text, i)
        if m:
            start_line = line
            start = m.end()
            j = start
            while j < n and text[j] != '"':
                if text[j] == "\\":
                    j += 1
                if j < n and text[j] == "\n":
                    line += 1
                j += 1
            yield start_line, _bytes(text, start, j), _WIDTH[m.group(1)], "string"
            i = j + 1
            continue
        i += 1


# A literal can only exceed the limit if some physical line is long, the file
# holds a raw literal (its body may hold newlines), or a literal is written with
# a backslash-newline continuation. Those tests are cheap enough to run before
# the lexer, and together they are a superset of every possible finding.
#
# The bound counts *characters*, while the limit counts bytes, so it must assume
# the widest UTF-8 encoding a source character can have (4 bytes): a wide literal
# holding 2,001 emoji is 2,024 characters but 8,004 bytes, which is over MSVC's
# 8,000-byte cap for `L` while a 4,000-character bound would miss it.
LONG_LINE = LIMIT // 16


def may_exceed(text: str) -> bool:
    # split('\n') and not splitlines(): splitlines also splits on \v, \f,
    # U+0085 and U+2028, which the C++ lexer does not treat as newlines, so a
    # single long physical line would be measured as short fragments.
    return 'R"' in text or "\\\n" in text or any(
        len(line) >= LONG_LINE for line in text.split("\n")
    )


def scan_file(path: Path, root: Path) -> list[tuple[str, int, int, str]]:
    found: list[tuple[str, int, int, str]] = []
    text = path.read_text(encoding="utf-8", errors="replace")
    if not may_exceed(text):
        return found
    for line, length, width, kind in literals(text):
        if length > LIMIT // width:
            found.append((path.relative_to(root).as_posix(), line, length, kind))
    return found


def violations(root: Path = ROOT, changed: str | None = None) -> list[tuple[str, int, int, str]]:
    selected = scoped_paths(
        lambda rel: rel.startswith(SRC + "/") and rel.endswith(SUFFIXES), changed
    )
    if selected is None:
        paths = sorted(
            path for path in (root / SRC).rglob("*") if path.suffix in SUFFIXES
        )
    else:
        paths = [root / rel for rel in selected if (root / rel).is_file()]
    found: list[tuple[str, int, int, str]] = []
    for path in paths:
        found += scan_file(path, root)
    return found


def _measure(text: str) -> list[tuple[int, int, str]]:
    return [(line, length, kind) for line, length, _w, kind in literals(text)]


def _self_test() -> None:
    # A quote inside a comment or char literal is not a string.
    assert _measure('/* "x" */ const char* s = "real";') == [(1, 4, "string")]
    assert _measure("// \"x\"\nchar c = 'a';\n\"y\";") == [(3, 1, "string")]
    # A C++14 digit separator must not swallow the following literal.
    assert _measure("int a = 1'000;\nconst char* s = \"AB\";") == [(2, 2, "string")]
    assert _measure("int b = 1'000'000;\n\"Z\";") == [(2, 1, "string")]
    # Raw literals: empty and custom delimiters; internal quotes are body.
    assert _measure('R"(raw body)"') == [(1, 8, "raw")]
    assert _measure('R"delim(a)body)delim"') == [(1, 6, "raw")]
    assert _measure('R"(a "b" c)"') == [(1, 7, "raw")]
    # Escapes count as source bytes (upper bound); UTF-8 counts bytes, not chars.
    assert _measure(r'"a\"b"') == [(1, 4, "string")]
    assert _measure('"\u25be"') == [(1, 3, "string")]
    # Encoding prefixes are recognised and carry a width.
    assert _measure('u8"x"') == [(1, 1, "string")]
    assert _measure('L"abc"') == [(1, 3, "string")]
    assert _measure('uR"(ab)"') == [(1, 2, "raw")]
    # Unterminated comment yields nothing rather than raising.
    assert _measure('/* unterminated "x"') == []
    # A token over 16,000 bytes is reported by violations-equivalent logic.
    big = '"' + "a" * 16001 + '"'
    assert any(length > LIMIT // width for _l, length, width, _k in literals(big))
    # The prefilter bounds characters, so a wide literal must not slip under it:
    # 2,001 emoji are 2,024 characters but 8,004 bytes, over the L cap of 8,000.
    wide = 'const wchar_t* s = L"' + "\U0001F600" * 2001 + '";'
    assert may_exceed(wide), "the prefilter missed a wide literal over the byte limit"
    assert any(length > LIMIT // width for _l, length, width, _k in literals(wide))
    # split("\n") not splitlines(): U+2028 is not a newline to the lexer, so a
    # single long physical line carrying it must still be measured whole.
    assert may_exceed("x" * 10 + "\u2028" + "y" * (LONG_LINE - 5))
    assert not may_exceed("short\u2028" * 10)

    import tempfile

    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        src = root / SRC
        src.mkdir(parents=True)
        (src / "clean.cpp").write_text('const char* s = "ok";\n', encoding="utf-8")
        (src / "big.cpp").write_text('const char* s = "' + "a" * 16001 + '";\n', encoding="utf-8")
        (src / "notes.txt").write_text('"' + "a" * 16001 + '"\n', encoding="utf-8")
        changed = root / "changed.txt"
        changed.write_text(f"{SRC}/clean.cpp\n", encoding="utf-8")
        assert violations(root, str(changed)) == [], "a scoped scan flagged an unchanged file"
        changed.write_text(f"{SRC}/clean.cpp\n{SRC}/big.cpp\n", encoding="utf-8")
        assert [v[0] for v in violations(root, str(changed))] == [f"{SRC}/big.cpp"]
        changed.write_text("", encoding="utf-8")
        assert [v[0] for v in violations(root, str(changed))] == [f"{SRC}/big.cpp"]
        assert [v[0] for v in violations(root)] == [f"{SRC}/big.cpp"]
    print("check-msvc-string-limits: self-test OK")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--changed", default=None, help="list of changed paths to scan")
    parser.add_argument("--full", action="store_true", help="scan the whole tree (default)")
    parser.add_argument("--root", default=None, help="scan this directory instead of the repository")
    args = parser.parse_args()
    if args.self_test:
        _self_test()
        return 0
    root = Path(args.root) if args.root else ROOT
    failed = violations(root, None if args.full else args.changed)
    if failed:
        print(f"check-msvc-string-limits: FAILED (literal over {LIMIT} bytes; MSVC caps at 16380)")
        for rel, line, length, kind in failed:
            print(f"  {rel}:{line}: {length} bytes ({kind})")
        print("Split the literal into adjacent pieces, each under the limit.")
        return 1
    print("check-msvc-string-limits: OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
