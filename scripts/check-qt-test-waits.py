#!/usr/bin/env python3
"""Enforce the Qt Test fixed-wait policy on the C++ sources.

A fixed wait in a Qt Test suite either flushes queued events (`qWait(0)`) or says
why no observable condition can replace it, in a `//` comment that trails the
call or on a comment-only line right above it. This is the same policy the Rust
`qt_test_waits_are_justified` test carried; it lives here so it runs where C++
changes are checked: `scripts/guard.sh` (local verification and the guards CI
workflow) and the Qt-aware CI job, not only in a Rust test module that a
C++-only pull request never executes.

`--changed FILE` (the list from `scripts/changed_files.py`) scans only the Qt
Test sources in the changed set; an empty list or `--full` scans the whole test
tree.

Run from the repository root:
    python3 scripts/check-qt-test-waits.py [--changed FILE] [--full] [--self-test]
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from changed_files import scoped_paths  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
TESTS = "crates/pictura-app/cpp/tests"
SUFFIXES = (".cpp", ".h")
CALL = "qWait("


def wait_violations_in(text: str) -> list[tuple[int, str]]:
    """(line number, line) for each non-zero wait without a reason comment."""
    lines = text.splitlines()
    found: list[tuple[int, str]] = []
    for index, line in enumerate(lines):
        rest = line
        while True:
            at = rest.find(CALL)
            if at < 0:
                break
            args = rest[at + len(CALL):]
            depth = 1
            close = len(args)
            for offset, char in enumerate(args):
                if char == "(":
                    depth += 1
                elif char == ")":
                    depth -= 1
                    if depth == 0:
                        close = offset
                        break
            trailing = "//" in args[close:]
            above = index > 0 and lines[index - 1].lstrip().startswith("//")
            if args[:close].strip() != "0" and not trailing and not above:
                found.append((index + 1, line.strip()))
            rest = args[close:]
    return found


def wanted(rel: str) -> bool:
    return rel.startswith(TESTS + "/") and rel.endswith(SUFFIXES)


def violations(root: Path = ROOT, changed: str | None = None) -> list[str]:
    selected = scoped_paths(wanted, changed)
    if selected is None:
        paths = sorted(path for path in (root / TESTS).glob("*") if path.suffix in SUFFIXES)
    else:
        paths = [root / rel for rel in selected if (root / rel).is_file()]
    found = []
    for path in paths:
        text = path.read_text(encoding="utf-8", errors="replace")
        rel = path.relative_to(root).as_posix()
        found.extend(f"{rel}:{line}: {body}" for line, body in wait_violations_in(text))
    return found


def self_test() -> int:
    assert wait_violations_in("QTest::qWait(0);") == []
    assert wait_violations_in("QTest::qWait(50); // repaint needs a frame") == []
    assert wait_violations_in("// no signal marks the debounce\nQTest::qWait(250);") == []
    assert wait_violations_in("// no observable condition\nQTest::qWait(250);") == []
    assert wait_violations_in(
        "QTest::qWait(200); // asserts the edit does not fire early"
    ) == []
    assert wait_violations_in("QTest::qWait(150);") == [(1, "QTest::qWait(150);")]
    assert [line for line, _ in wait_violations_in("a;\nQTest::qWait(x + 1);\nb;")] == [2]
    assert wait_violations_in("qWait(0); qWait(5);") == [(1, "qWait(0); qWait(5);")]

    import tempfile

    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        tests = root / TESTS
        tests.mkdir(parents=True)
        (tests / "tst_ok.cpp").write_text("QTest::qWait(0);\n", encoding="utf-8")
        (tests / "tst_bad.cpp").write_text("QTest::qWait(20);\n", encoding="utf-8")
        assert [v.split(":")[0] for v in violations(root)] == [
            f"{TESTS}/tst_bad.cpp"
        ]

        changed = root / "changed.txt"
        changed.write_text(f"{TESTS}/tst_ok.cpp\n", encoding="utf-8")
        assert violations(root, str(changed)) == []
        changed.write_text(f"{TESTS}/tst_bad.cpp\n", encoding="utf-8")
        assert [v.split(":")[0] for v in violations(root, str(changed))] == [
            f"{TESTS}/tst_bad.cpp"
        ]
        # A changed path outside the Qt test tree is not this check's business.
        (root / "crates/pictura-app/cpp").mkdir(parents=True, exist_ok=True)
        (root / "crates/pictura-app/cpp/frame.cpp").write_text(
            "qWait(20);\n", encoding="utf-8"
        )
        changed.write_text("crates/pictura-app/cpp/frame.cpp\n", encoding="utf-8")
        assert violations(root, str(changed)) == []
    print("check-qt-test-waits: self-test OK")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--changed", default=None, help="list of changed paths to scan")
    parser.add_argument("--full", action="store_true", help="scan the whole tree (default)")
    args = parser.parse_args()
    if args.self_test:
        return self_test()
    failed = violations(changed=None if args.full else args.changed)
    if failed:
        print("check-qt-test-waits: FAILED (fixed wait without a reason comment)")
        for item in failed:
            print(f"  {item}")
        print("Wait on an observable condition, use qWait(0) to flush events, or name")
        print("why no condition exists in a // comment on or above the line.")
        return 1
    print("check-qt-test-waits: OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
