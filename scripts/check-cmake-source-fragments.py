#!/usr/bin/env python3
"""Verify the CMake source fragments cover every translation unit exactly once.

The shell and application source lists live in `cmake/sources/*.cmake`. This
check keeps the split honest:

- every fragment has a `list(APPEND ...)` block that appends to the variable
  for its target (shell fragments to `PICTURA_SHELL_SOURCES`, `selftest.cmake`
  to `PICTURA_SELF_TEST_SOURCES`),
- no path appears in two fragments,
- every listed path exists on disk,
- every `.cpp` under `crates/pictura-app/cpp/` is listed, except the Qt Test
  sources and the bridge translation unit that the Rust build script compiles
  and CMake does not (`panels/actions_tool_recording_bridge.cpp`),
- `main.cpp` and `selftest*` sources live only in `selftest.cmake`, and
  `selftest.cmake` holds nothing else,
- the root `CMakeLists.txt` includes every fragment.

Headers are optional; they are listed for IDE grouping but are not required.

Run `python3 scripts/check-cmake-source-fragments.py --self-test` to exercise
the checker on a throwaway tree.
"""

from __future__ import annotations

import argparse
import re
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FRAGMENTS = Path("cmake/sources")
CPP = Path("crates/pictura-app/cpp")
SELFTEST_FRAGMENT = "selftest.cmake"
SHELL_VARIABLE = "PICTURA_SHELL_SOURCES"
SELFTEST_VARIABLE = "PICTURA_SELF_TEST_SOURCES"
RUST_BUILT = "crates/pictura-app/cpp/panels/actions_tool_recording_bridge.cpp"
PATH_RE = re.compile(r"crates/pictura-app/cpp/[^\s)]+")
APPEND_RE = re.compile(r"list\(\s*APPEND\s+(\w+)(.*?)\)", re.S)
INCLUDE_RE = re.compile(r"include\(\s*cmake/sources/([^\s)]+)\s*\)")


def is_selftest_source(path: str) -> bool:
    name = Path(path).name
    return name == "main.cpp" or name.startswith("selftest")


def fragment_appends(fragment: Path) -> list[tuple[str, list[str]]]:
    """(variable, paths) for each list(APPEND ...) block, comments stripped."""
    text = re.sub(r"#[^\n]*", "", fragment.read_text(encoding="utf-8"))
    return [
        (match.group(1), PATH_RE.findall(match.group(2)))
        for match in APPEND_RE.finditer(text)
    ]


def check(root: Path) -> list[str]:
    fragments = root / FRAGMENTS
    if not fragments.is_dir():
        return [f"missing directory: {FRAGMENTS}"]

    errors: list[str] = []
    listed: dict[str, str] = {}
    for fragment in sorted(fragments.glob("*.cmake")):
        expected = (
            SELFTEST_VARIABLE
            if fragment.name == SELFTEST_FRAGMENT
            else SHELL_VARIABLE
        )
        appends = fragment_appends(fragment)
        if not appends:
            errors.append(f"{fragment.name} has no list(APPEND ...) block")
        for variable, paths in appends:
            if variable != expected:
                errors.append(
                    f"{fragment.name} appends to {variable}, expected {expected}"
                )
            for path in paths:
                if path in listed:
                    errors.append(
                        f"duplicate path {path} in {fragment.name} and {listed[path]}"
                    )
                    continue
                listed[path] = fragment.name

    for path, fragment in listed.items():
        if path == RUST_BUILT:
            errors.append(f"{fragment} lists the Rust-built translation unit: {path}")
        if not (root / path).is_file():
            errors.append(f"{fragment} lists a missing file: {path}")
        selftest_source = is_selftest_source(path)
        in_selftest = fragment == SELFTEST_FRAGMENT
        if selftest_source and not in_selftest:
            errors.append(f"{fragment} lists a self-test source: {path}")
        if in_selftest and not selftest_source:
            errors.append(f"{SELFTEST_FRAGMENT} lists a shell source: {path}")

    cpp = root / CPP
    for source in sorted(cpp.rglob("*.cpp")):
        rel = source.relative_to(root).as_posix()
        if "/tests/" in rel or rel == RUST_BUILT:
            continue
        if rel not in listed:
            errors.append(f"translation unit not listed in any fragment: {rel}")

    cmake = root / "CMakeLists.txt"
    if not cmake.is_file():
        errors.append("missing root CMakeLists.txt")
        included: set[str] = set()
    else:
        included = set(INCLUDE_RE.findall(cmake.read_text(encoding="utf-8")))
    for fragment in sorted(fragments.glob("*.cmake")):
        if fragment.name not in included:
            errors.append(
                f"fragment not included by CMakeLists.txt: {fragment.name}"
            )

    return errors


def _write_tree(
    root: Path,
    shell: list[str],
    selftest: list[str],
    files: list[str],
    includes: list[str] | None = None,
    shell_variable: str = SHELL_VARIABLE,
) -> None:
    fragments = root / FRAGMENTS
    fragments.mkdir(parents=True)
    for name, variable, paths in (
        ("shell.cmake", shell_variable, shell),
        (SELFTEST_FRAGMENT, SELFTEST_VARIABLE, selftest),
    ):
        body = "".join(f"    {path}\n" for path in paths)
        (fragments / name).write_text(
            f"list(APPEND {variable}\n{body})\n", encoding="utf-8"
        )
    if includes is None:
        includes = ["shell.cmake", SELFTEST_FRAGMENT]
    (root / "CMakeLists.txt").write_text(
        "".join(f"include(cmake/sources/{name})\n" for name in includes),
        encoding="utf-8",
    )
    for path in files:
        target = root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text("", encoding="utf-8")


def self_test() -> int:
    a = "crates/pictura-app/cpp/a.cpp"
    b = "crates/pictura-app/cpp/b.h"
    main = "crates/pictura-app/cpp/main.cpp"
    selftest = "crates/pictura-app/cpp/selftest_x.cpp"
    cases = [
        {
            "name": "pass",
            "shell": [a, b],
            "selftest": [main, selftest],
            "files": [
                a,
                b,
                main,
                selftest,
                "crates/pictura-app/cpp/tests/tst_x.cpp",
                RUST_BUILT,
            ],
            "expected": None,
        },
        {
            "name": "duplicate",
            "shell": [a, a],
            "selftest": [main],
            "files": [a, main],
            "expected": "duplicate path",
        },
        {
            "name": "missing",
            "shell": [a, "crates/pictura-app/cpp/missing.cpp"],
            "selftest": [main],
            "files": [a, main],
            "expected": "missing file",
        },
        {
            "name": "unlisted",
            "shell": [a],
            "selftest": [main],
            "files": [a, "crates/pictura-app/cpp/c.cpp", main],
            "expected": "not listed",
        },
        {
            "name": "wrong target",
            "shell": [a, selftest],
            "selftest": [main],
            "files": [a, selftest, main],
            "expected": "self-test source",
        },
        {
            "name": "selftest holds shell",
            "shell": [a],
            "selftest": [main, b],
            "files": [a, main, b],
            "expected": "shell source",
        },
        {
            "name": "commented out",
            "shell": [f"# {a}"],
            "selftest": [main],
            "files": [a, main],
            "expected": "not listed",
        },
        {
            "name": "wrong variable",
            "shell": [a],
            "selftest": [main],
            "files": [a, main],
            "shell_variable": SELFTEST_VARIABLE,
            "expected": "appends to",
        },
        {
            "name": "missing include",
            "shell": [a],
            "selftest": [main],
            "files": [a, main],
            "includes": ["shell.cmake"],
            "expected": "not included",
        },
        {
            "name": "rust built listed",
            "shell": [RUST_BUILT],
            "selftest": [main],
            "files": [RUST_BUILT, main],
            "expected": "Rust-built",
        },
    ]
    failed = 0
    with tempfile.TemporaryDirectory(prefix="pictura-fragments-check.") as tmp:
        for case in cases:
            root = Path(tmp) / case["name"].replace(" ", "-")
            _write_tree(
                root,
                case["shell"],
                case["selftest"],
                case["files"],
                includes=case.get("includes"),
                shell_variable=case.get("shell_variable", SHELL_VARIABLE),
            )
            errors = check(root)
            expected = case["expected"]
            if expected is None:
                if errors:
                    print(
                        f"self-test {case['name']}: expected pass, got {errors}",
                        file=sys.stderr,
                    )
                    failed = 1
            elif not any(expected in error for error in errors):
                print(
                    f"self-test {case['name']}: expected an error containing "
                    f"{expected!r}, got {errors}",
                    file=sys.stderr,
                )
                failed = 1
    if failed:
        return 1
    print("check-cmake-source-fragments: self-test OK")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        return self_test()

    errors = check(ROOT)
    if errors:
        for error in errors:
            print(f"check-cmake-source-fragments: {error}", file=sys.stderr)
        print(
            f"check-cmake-source-fragments: FAILED ({len(errors)} problem(s))",
            file=sys.stderr,
        )
        return 1
    print("check-cmake-source-fragments: OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
