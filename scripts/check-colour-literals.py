#!/usr/bin/env python3
"""Reject theme-like grey colour literals outside the theme layer.

The design system is token-driven: a widget outside `theme.{h,cpp}` must not
hard-code a grey surface, text, border, or control colour. This check flags
opaque equal-channel `QColor(r, g, b)` / `QColor(0xNN, ...)` literals and the
grey `Qt::gray`/`darkGray`/`lightGray` values in `crates/pictura-app/cpp`,
excluding `theme.cpp` and a baseline allow-list of documented data-colour sites
and not-yet-converted debt (`scripts/colour-literal-allowlist.txt`). The
allow-list only shrinks.

Chromatic literals (a chosen colour, a gradient stop, a per-channel palette) and
translucent greys (overlays/dims) are out of scope: the common theme drift is
opaque greys on the grey ramp.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "crates/pictura-app/cpp"
ALLOW = ROOT / "scripts" / "colour-literal-allowlist.txt"

COLOR_RE = re.compile(r"QColor\s*\(([^()]*)\)")
QT_GREY_RE = re.compile(r"Qt::(gray|darkGray|lightGray)\b")


def _num(token: str) -> int | None:
    token = token.strip()
    try:
        return int(token, 16) if token.lower().startswith("0x") else int(token)
    except ValueError:
        return None


def grey_literals(text: str) -> set[str]:
    found: set[str] = set()
    for match in COLOR_RE.finditer(text):
        args = [a.strip() for a in match.group(1).split(",") if a.strip()]
        if len(args) < 3:
            continue
        channels = [_num(a) for a in args[:3]]
        if None in channels:
            continue
        # Translucent greys are overlays/dims (effects or data), not the opaque
        # grey ramp this guard polices.
        if len(args) >= 4 and _num(args[3]) != 255:
            continue
        if channels[0] == channels[1] == channels[2]:
            found.add(match.group(0))
    for match in QT_GREY_RE.finditer(text):
        found.add(match.group(0))
    return found


def load_allowlist() -> set[tuple[str, str]]:
    allowed: set[tuple[str, str]] = set()
    if not ALLOW.exists():
        return allowed
    for line in ALLOW.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        path, sep, rest = line.partition("  ")
        if not sep:
            continue
        literal = rest.split("  ", 1)[0].strip()
        allowed.add((path.strip(), literal))
    return allowed


def is_test_file(rel: str, name: str) -> bool:
    return "/tests/" in rel or name.endswith("_test.cpp") or name.endswith("_test.h") \
        or name.startswith("selftest")


def violations() -> list[tuple[str, str]]:
    allowed = load_allowlist()
    found: list[tuple[str, str]] = []
    for path in sorted(SRC.rglob("*")):
        if path.suffix not in (".cpp", ".h") or path.name == "theme.cpp":
            continue
        rel = path.relative_to(ROOT).as_posix()
        if is_test_file(rel, path.name):
            continue
        text = path.read_text(encoding="utf-8", errors="replace")
        for literal in sorted(grey_literals(text)):
            if (rel, literal) not in allowed:
                found.append((rel, literal))
    return found


def main() -> int:
    failed = violations()
    if failed:
        print("check-colour-literals: FAILED (theme-like grey literals outside the theme layer)")
        for rel, literal in failed:
            print(f"  {rel}: {literal}")
        print("Use a theme token, or add the site to scripts/colour-literal-allowlist.txt with a reason.")
        return 1
    print("check-colour-literals: OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
