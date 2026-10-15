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

Run `python3 scripts/check-msvc-string-limits.py --self-test` to exercise the
lexer on the tricky cases.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path
from typing import Iterator

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "crates/pictura-app/cpp"

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


def violations() -> list[tuple[str, int, int, str]]:
    found: list[tuple[str, int, int, str]] = []
    for path in sorted(SRC.rglob("*")):
        if path.suffix not in (".cpp", ".h"):
            continue
        text = path.read_text(encoding="utf-8", errors="replace")
        for line, length, width, kind in literals(text):
            if length > LIMIT // width:
                found.append((path.relative_to(ROOT).as_posix(), line, length, kind))
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
    print("check-msvc-string-limits: self-test OK")


def main() -> int:
    failed = violations()
    if failed:
        print(f"check-msvc-string-limits: FAILED (literal over {LIMIT} bytes; MSVC caps at 16380)")
        for rel, line, length, kind in failed:
            print(f"  {rel}:{line}: {length} bytes ({kind})")
        print("Split the literal into adjacent pieces, each under the limit.")
        return 1
    print("check-msvc-string-limits: OK")
    return 0


if __name__ == "__main__":
    if "--self-test" in sys.argv[1:]:
        _self_test()
        sys.exit(0)
    sys.exit(main())
