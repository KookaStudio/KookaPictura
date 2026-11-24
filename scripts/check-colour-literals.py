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

Each finding is local to one file, so `--changed FILE` (the list from
`scripts/changed_files.py`) scans only the changed C++ sources; an empty list
falls back to the whole tree, and `--full` (the default, what CI uses) always
scans everything.

Run from the repository root: python3 scripts/check-colour-literals.py [--changed FILE] [--full] [--self-test] [--root DIR]
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from changed_files import scoped_paths  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
SRC = "crates/pictura-app/cpp"
SUFFIXES = (".cpp", ".h")

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


def load_allowlist(root: Path = ROOT) -> set[tuple[str, str]]:
    allowed: set[tuple[str, str]] = set()
    allow = root / "scripts" / "colour-literal-allowlist.txt"
    if not allow.exists():
        return allowed
    for line in allow.read_text(encoding="utf-8").splitlines():
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


def scan_paths(
    paths: list[Path], root: Path, allowed: set[tuple[str, str]]
) -> list[tuple[str, str]]:
    found: list[tuple[str, str]] = []
    for path in paths:
        rel = path.relative_to(root).as_posix()
        text = path.read_text(encoding="utf-8", errors="replace")
        for literal in sorted(grey_literals(text)):
            if (rel, literal) not in allowed:
                found.append((rel, literal))
    return found


# The allowlist exempts sites by path, so a change to it can affect any file the
# changed set does not name: that falls back to the whole-tree scan.
ALLOWLIST_CHANGED = lambda rel: rel == "scripts/colour-literal-allowlist.txt"  # noqa: E731


def wanted(rel: str) -> bool:
    if not (rel.startswith(SRC + "/") and rel.endswith(SUFFIXES)):
        return False
    name = rel.rsplit("/", 1)[-1]
    return name != "theme.cpp" and not is_test_file(rel, name)


def violations(root: Path = ROOT, changed: str | None = None) -> list[tuple[str, str]]:
    allowed = load_allowlist(root)
    selected = scoped_paths(wanted, changed, ALLOWLIST_CHANGED)
    if selected is None:
        paths = sorted(
            path
            for path in (root / SRC).rglob("*")
            if path.suffix in SUFFIXES
            and wanted(f"{SRC}/{path.relative_to(root / SRC).as_posix()}")
        )
    else:
        paths = [root / rel for rel in selected if (root / rel).is_file()]
    return scan_paths(paths, root, allowed)


def _self_test() -> int:
    import tempfile

    assert grey_literals("QColor(40, 40, 40)") == {"QColor(40, 40, 40)"}
    assert grey_literals("QColor(40, 40, 40, 255)") == {"QColor(40, 40, 40, 255)"}
    assert grey_literals("QColor(40, 40, 40, 128)") == set()
    assert grey_literals("QColor(10, 20, 30)") == set()
    assert grey_literals("Qt::darkGray") == {"Qt::darkGray"}

    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        src = root / SRC
        (src / "tests").mkdir(parents=True)
        (root / "scripts").mkdir()
        (root / "scripts" / "colour-literal-allowlist.txt").write_text(
            f"{SRC}/allowed.cpp  QColor(50, 50, 50)\n", encoding="utf-8"
        )
        (src / "clean.cpp").write_text("QColor(1, 2, 3);\n", encoding="utf-8")
        (src / "bad.cpp").write_text("QColor(40, 40, 40);\n", encoding="utf-8")
        (src / "allowed.cpp").write_text("QColor(50, 50, 50);\n", encoding="utf-8")
        (src / "theme.cpp").write_text("QColor(40, 40, 40);\n", encoding="utf-8")
        (src / "tests" / "tst_x.cpp").write_text("QColor(40, 40, 40);\n", encoding="utf-8")

        changed = root / "changed.txt"
        changed.write_text(f"{SRC}/clean.cpp\n", encoding="utf-8")
        assert violations(root, str(changed)) == [], "a scoped scan flagged an unchanged file"
        changed.write_text(f"{SRC}/clean.cpp\n{SRC}/bad.cpp\n", encoding="utf-8")
        assert [v[0] for v in violations(root, str(changed))] == [f"{SRC}/bad.cpp"]
        assert violations(root) == [(f"{SRC}/bad.cpp", "QColor(40, 40, 40)")]
        changed.write_text("", encoding="utf-8")
        assert violations(root, str(changed)) == [(f"{SRC}/bad.cpp", "QColor(40, 40, 40)")]
        changed.write_text("scripts/colour-literal-allowlist.txt\n", encoding="utf-8")
        assert violations(root, str(changed)) == [(f"{SRC}/bad.cpp", "QColor(40, 40, 40)")], \
            "an allowlist change did not widen the scan"
    print("check-colour-literals: self-test OK")
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
    failed = violations(root, None if args.full else args.changed)
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
