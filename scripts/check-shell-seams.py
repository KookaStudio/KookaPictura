#!/usr/bin/env python3
"""Verify the main-window shell seams, the moc owners, and their fixed budgets.

The shell's panel translation units, the MenuRegistrar translation units, and
the four collaborator translation units must not include `frame.h` or name
`PicturaMainWindow`; no `frame_menus*.cpp` may remain; the collaborator headers
must not include `frame.h`; and `frame.h` / `frame_includes.h` must stay inside
their fixed ceilings (760 lines and 60 include lines). There is no allowlist.

Two ownership rules ride along. Every QObject header with a same-named `.cpp`
must have that `.cpp` include its generated `moc_<header>.cpp`, and a QObject
header without one must have a consumer that includes its moc, so a header edit
never recompiles a central moc unit (build-performance `bp_moc_includes`). The Qt
Test fixture header must not include `frame.h`, so the suites' shared header does
not pull the frame closure into every suite.

Run from the repository root: python3 scripts/check-shell-seams.py [--self-test]
"""

from __future__ import annotations

import argparse
import re
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CPP = Path("crates/pictura-app/cpp")
FRAME_H_MAX_LINES = 760
FRAME_INCLUDES_MAX_INCLUDES = 60
FRAME_INCLUDE = re.compile(r'^\s*#\s*include\s*"frame\.h"', re.MULTILINE)
INCLUDE_LINE = re.compile(r"^\s*#\s*include\b", re.MULTILINE)
FRAME_NAME = "PicturaMainWindow"

TEST_FIXTURE = Path("tests/qt_test_support.h")
QOBJECT = re.compile(r"\bQ_OBJECT\b|\bQ_GADGET\b")
MOC_INCLUDE = "moc_{}.cpp"

COLLABORATOR_HEADERS = (
    "document_registry.h",
    "panel_host.h",
    "import_save_coordinator.h",
    "menu_registrar.h",
)
COLLABORATOR_TUS = (
    "document_registry*.cpp",
    "panel_host*.cpp",
    "import_save_*.cpp",
    "menu_registrar*.cpp",
)
MENU_FILES = ("menu_*.cpp", "menu_registrar*")


def _rel(path: Path, root: Path) -> str:
    try:
        return path.relative_to(root).as_posix()
    except ValueError:
        return path.as_posix()


def _scan(paths: list[Path], root: Path) -> list[str]:
    errors = []
    for path in paths:
        text = path.read_text(encoding="utf-8", errors="replace")
        if FRAME_INCLUDE.search(text):
            errors.append(f'{_rel(path, root)}: includes "frame.h"')
        if FRAME_NAME in text:
            errors.append(f"{_rel(path, root)}: names {FRAME_NAME}")
    return errors


def check_panel_tus(root: Path = ROOT) -> list[str]:
    panels = root / CPP / "panels"
    files = sorted(
        path
        for path in panels.rglob("*")
        if path.is_file() and path.suffix in (".cpp", ".h") and "_test" not in path.name
    )
    return _scan(files, root)


def check_menu_tus(root: Path = ROOT) -> list[str]:
    cpp = root / CPP
    files = sorted(
        path
        for pattern in MENU_FILES
        for path in cpp.glob(pattern)
        if path.is_file()
    )
    errors = _scan(files, root)
    errors.extend(
        f"{_rel(path, root)}: legacy frame_menus translation unit remains"
        for path in sorted(cpp.rglob("frame_menus*.cpp"))
    )
    return errors


def check_collaborator_tus(root: Path = ROOT) -> list[str]:
    cpp = root / CPP
    files = sorted(
        path
        for pattern in COLLABORATOR_TUS
        for path in cpp.glob(pattern)
        if path.is_file()
    )
    return _scan(files, root)


def check_collaborator_headers(root: Path = ROOT) -> list[str]:
    cpp = root / CPP
    errors = []
    for name in COLLABORATOR_HEADERS:
        path = cpp / name
        if not path.is_file():
            errors.append(f"{_rel(path, root)}: collaborator header is missing")
            continue
        if FRAME_INCLUDE.search(path.read_text(encoding="utf-8", errors="replace")):
            errors.append(f'{_rel(path, root)}: includes "frame.h"')
    return errors


def check_moc_ownership(root: Path = ROOT) -> list[str]:
    cpp = root / CPP
    sources = sorted(cpp.rglob("*.cpp"))
    includes: dict[str, list[Path]] = {}
    for source in sources:
        text = source.read_text(encoding="utf-8", errors="replace")
        for stem in re.findall(r'^\s*#\s*include\s*"(moc_[^"]+)\.cpp"', text, re.MULTILINE):
            includes.setdefault(f"{stem}.cpp", []).append(source)
    errors = []
    for header in sorted(cpp.rglob("*.h")):
        if not QOBJECT.search(header.read_text(encoding="utf-8", errors="replace")):
            continue
        moc = MOC_INCLUDE.format(header.stem)
        same = header.with_suffix(".cpp")
        if same.is_file():
            if moc not in same.read_text(encoding="utf-8", errors="replace"):
                errors.append(f"{_rel(same, root)}: does not include its own {moc}")
        elif not includes.get(moc):
            errors.append(f"{_rel(header, root)}: no translation unit includes {moc}")
    return errors


def check_test_fixture_seam(root: Path = ROOT) -> list[str]:
    fixture = root / CPP / TEST_FIXTURE
    if not fixture.is_file():
        return [f"{_rel(fixture, root)}: file is missing"]
    if FRAME_INCLUDE.search(fixture.read_text(encoding="utf-8", errors="replace")):
        return [
            f'{_rel(fixture, root)}: includes "frame.h"; the suites\' shared header '
            "must not pull the frame closure"
        ]
    return []


def check_budgets(root: Path = ROOT) -> list[str]:
    cpp = root / CPP
    errors = []
    frame_h = cpp / "frame.h"
    frame_includes = cpp / "frame_includes.h"
    if not frame_h.is_file():
        errors.append(f"{_rel(frame_h, root)}: file is missing")
    else:
        lines = len(frame_h.read_text(encoding="utf-8", errors="replace").splitlines())
        if lines > FRAME_H_MAX_LINES:
            errors.append(f"{_rel(frame_h, root)}: {lines} lines exceeds {FRAME_H_MAX_LINES}")
    if not frame_includes.is_file():
        errors.append(f"{_rel(frame_includes, root)}: file is missing")
    else:
        text = frame_includes.read_text(encoding="utf-8", errors="replace")
        includes = len(INCLUDE_LINE.findall(text))
        if includes > FRAME_INCLUDES_MAX_INCLUDES:
            errors.append(
                f"{_rel(frame_includes, root)}: {includes} include lines exceeds "
                f"{FRAME_INCLUDES_MAX_INCLUDES}"
            )
    return errors


def self_test() -> int:
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        cpp = root / CPP
        panels = cpp / "panels"
        panels.mkdir(parents=True)

        (panels / "clean.cpp").write_text('#include "panel_host.h"\n', encoding="utf-8")
        (panels / "frame.cpp").write_text('#include "frame.h"\n', encoding="utf-8")
        (panels / "named.h").write_text("class PicturaMainWindow;\n", encoding="utf-8")
        (panels / "test_support_test.cpp").write_text(
            '#include "frame.h"\n', encoding="utf-8"
        )

        (cpp / "menu_clean.cpp").write_text("int menu();\n", encoding="utf-8")
        (cpp / "menu_bad.cpp").write_text("class PicturaMainWindow;\n", encoding="utf-8")
        (cpp / "menu_registrar_internal.h").write_text(
            '#include "frame.h"\n', encoding="utf-8"
        )

        (cpp / "panel_host.cpp").write_text('#include "frame.h"\n', encoding="utf-8")
        (cpp / "panel_host.h").write_text('#include "frame.h"\n', encoding="utf-8")
        (cpp / "document_registry.h").write_text("class DocumentRegistry;\n", encoding="utf-8")
        (cpp / "import_save_coordinator.h").write_text(
            '#include "document_registry.h"\n', encoding="utf-8"
        )
        (cpp / "menu_registrar.h").write_text("class MenuRegistrar;\n", encoding="utf-8")

        def errors_of(fn) -> str:
            return "\n".join(fn(root))

        panel_errors = errors_of(check_panel_tus)
        assert "panels/frame.cpp" in panel_errors
        assert "panels/named.h" in panel_errors
        assert "test_support_test.cpp" not in panel_errors
        assert "panels/clean.cpp" not in panel_errors

        menu_errors = errors_of(check_menu_tus)
        assert "menu_bad.cpp" in menu_errors
        assert "menu_registrar_internal.h" in menu_errors
        assert "menu_clean.cpp" not in menu_errors
        (cpp / "frame_menus_old.cpp").write_text("int old();\n", encoding="utf-8")
        assert "frame_menus_old.cpp" in errors_of(check_menu_tus)

        collaborator_errors = errors_of(check_collaborator_tus)
        assert "panel_host.cpp" in collaborator_errors
        assert "import_save_coordinator.cpp" not in collaborator_errors

        header_errors = errors_of(check_collaborator_headers)
        assert "panel_host.h" in header_errors
        assert "document_registry.h" not in header_errors

        # A same-named `.cpp` owner that omits its moc, and a header with no
        # owner at all, both fail; the compliant pair and the consumer-owned
        # header pass.
        (cpp / "owned.h").write_text("class Owned {\n    Q_OBJECT\n};\n", encoding="utf-8")
        (cpp / "owned.cpp").write_text('#include "owned.h"\n', encoding="utf-8")
        (cpp / "consumed.h").write_text(
            "class Consumed {\n    Q_OBJECT\n};\n", encoding="utf-8"
        )
        (cpp / "consumed_owner.cpp").write_text(
            '#include "consumed.h"\n#include "moc_consumed.cpp"\n', encoding="utf-8"
        )
        (cpp / "plain.h").write_text("class Plain;\n", encoding="utf-8")
        moc_errors = errors_of(check_moc_ownership)
        assert "owned.cpp" in moc_errors
        assert "moc_owned.cpp" in moc_errors
        assert "consumed" not in moc_errors
        assert "plain.h" not in moc_errors
        (cpp / "owned.cpp").write_text(
            '#include "owned.h"\n#include "moc_owned.cpp"\n', encoding="utf-8"
        )
        assert errors_of(check_moc_ownership) == ""

        fixture = cpp / TEST_FIXTURE
        fixture.parent.mkdir(parents=True, exist_ok=True)
        fixture.write_text("namespace pictura::test {}\n", encoding="utf-8")
        assert errors_of(check_test_fixture_seam) == ""
        fixture.write_text('#include "frame.h"\n', encoding="utf-8")
        assert "qt_test_support.h" in errors_of(check_test_fixture_seam)

        (cpp / "frame.h").write_text("\n" * FRAME_H_MAX_LINES, encoding="utf-8")
        (cpp / "frame_includes.h").write_text(
            '#include "frame.h"\n' * FRAME_INCLUDES_MAX_INCLUDES, encoding="utf-8"
        )
        assert errors_of(check_budgets) == ""

        (cpp / "frame.h").write_text(
            "\n" * (FRAME_H_MAX_LINES + 1), encoding="utf-8"
        )
        assert str(FRAME_H_MAX_LINES) in errors_of(check_budgets)
        (cpp / "frame.h").write_text("\n" * FRAME_H_MAX_LINES, encoding="utf-8")
        (cpp / "frame_includes.h").write_text(
            '#include "frame.h"\n' * (FRAME_INCLUDES_MAX_INCLUDES + 1),
            encoding="utf-8",
        )
        assert str(FRAME_INCLUDES_MAX_INCLUDES) in errors_of(check_budgets)

    print("check-shell-seams: self-test OK")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        return self_test()

    checks = (
        ("panels", check_panel_tus),
        ("menus", check_menu_tus),
        ("collaborator TUs", check_collaborator_tus),
        ("collaborator headers", check_collaborator_headers),
        ("moc owners", check_moc_ownership),
        ("test fixture seam", check_test_fixture_seam),
        ("budgets", check_budgets),
    )
    errors = [error for _, check in checks for error in check()]
    if errors:
        for error in errors:
            print(f"FAIL: {error}", file=sys.stderr)
        print(f"check-shell-seams: FAILED ({len(errors)} problem(s))", file=sys.stderr)
        return 1
    print("check-shell-seams: OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
