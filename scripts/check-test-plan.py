#!/usr/bin/env python3
"""Verify every OpenSpec change touched on this branch carries a test plan.

For each `openspec/changes/<name>/` (archive excluded) changed against the base
ref or in the working tree, fails when:
- `test-plan.md` is missing,
- a spec scenario (its `[tag]`, or its title when untagged) has no plan row,
- a row's Coverage cell is neither `manual: <reason>` nor test references,
- a referenced test does not exist: `tst_<suite>::<case>` must be a slot in
  crates/pictura-app/cpp/tests/tst_<suite>.cpp, `<crate>::<fn>` a `fn <fn>`
  under crates/<crate>/.

The Coverage cell is a row's last column. Base ref: $GUARD_BASE, else the merge
base with master/main.

Run from the repository root: python3 scripts/check-test-plan.py [--self-test]
"""
from __future__ import annotations

import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CHANGES = ROOT / "openspec" / "changes"
QT_TESTS = ROOT / "crates" / "pictura-app" / "cpp" / "tests"

SCENARIO = re.compile(r"^####\s+Scenario:\s*(.+?)\s*$")
TAG = re.compile(r"\[([A-Za-z0-9_\-]+)\]\s*$")
REF = re.compile(r"`([A-Za-z0-9_\-]+)::([A-Za-z0-9_]+)`")


def scenario_keys(spec_text: str) -> list[str]:
    keys = []
    for line in spec_text.splitlines():
        m = SCENARIO.match(line)
        if m:
            tag = TAG.search(m.group(1))
            keys.append(tag.group(1) if tag else m.group(1))
    return keys


def plan_rows(plan_text: str) -> list[tuple[str, str]]:
    """(scenario cell, coverage cell) for each table body row."""
    rows = []
    for line in plan_text.splitlines():
        line = line.strip()
        if not line.startswith("|"):
            continue
        cells = [c.strip() for c in line.strip("|").split("|")]
        if len(cells) < 2 or set(cells[0]) <= set("-: ") or cells[0].lower() == "scenario":
            continue
        rows.append((cells[0].strip("`"), cells[-1]))
    return rows


def coverage_errors(coverage: str, exists) -> list[str]:
    if coverage.lower().startswith("manual:"):
        return [] if coverage[len("manual:"):].strip() else ["manual row without a reason"]
    refs = REF.findall(coverage)
    if not refs:
        return [f"coverage is neither `manual: <reason>` nor a test reference: {coverage!r}"]
    return [f"no such test: {a}::{b}" for a, b in refs if not exists(a, b)]


def test_exists(owner: str, name: str) -> bool:
    if owner.startswith("tst_"):
        src = QT_TESTS / f"{owner}.cpp"
        return src.is_file() and re.search(
            rf"\bvoid\s+{name}\s*\(\s*\)", src.read_text(encoding="utf-8")
        ) is not None
    crate = ROOT / "crates" / owner
    pattern = re.compile(rf"\bfn\s+{name}\s*\(")
    return crate.is_dir() and any(
        pattern.search(p.read_text(encoding="utf-8")) for p in crate.rglob("*.rs")
    )


def check_change(change: Path, exists=test_exists) -> list[str]:
    plan = change / "test-plan.md"
    if not plan.is_file():
        return ["missing test-plan.md"]
    rows = plan_rows(plan.read_text(encoding="utf-8"))
    errors = []
    covered = {s for s, _ in rows}
    for spec in sorted(change.glob("specs/**/spec.md")):
        for key in scenario_keys(spec.read_text(encoding="utf-8")):
            if key not in covered:
                errors.append(f"scenario has no plan row: {key}")
    for scenario, coverage in rows:
        errors += [f"{scenario}: {e}" for e in coverage_errors(coverage, exists)]
    return errors


def git(*args: str) -> str:
    return subprocess.run(
        ["git", *args], cwd=ROOT, capture_output=True, text=True, check=False
    ).stdout


def touched_changes() -> list[Path]:
    base = os.environ.get("GUARD_BASE", "")
    if base and not git("rev-parse", "--verify", "-q", f"{base}^{{commit}}").strip():
        # A shallow CI checkout without the base would silently check nothing.
        raise SystemExit(f"check-test-plan: GUARD_BASE {base} is not in this clone; fetch it")
    if not base:
        for ref in ("master", "main", "origin/master", "origin/main"):
            base = git("merge-base", "HEAD", ref).strip()
            if base:
                break
    paths = git("status", "--porcelain", "--untracked-files=all", "--", "openspec/changes")
    paths = [line[3:] for line in paths.splitlines()]
    if base:
        paths += git("diff", "--name-only", base, "--", "openspec/changes").splitlines()
    names = set()
    for p in paths:
        parts = Path(p).parts
        if len(parts) > 3 and parts[2] != "archive":
            names.add(parts[2])
    return [CHANGES / n for n in sorted(names) if (CHANGES / n).is_dir()]


def self_test() -> int:
    spec = "#### Scenario: Tagged one [t_one]\n#### Scenario: Untagged two\n"
    assert scenario_keys(spec) == ["t_one", "Untagged two"]
    plan = (
        "| Scenario | Steps | Coverage |\n|---|---|---|\n"
        "| `t_one` | do | `tst_x::a`, `tst_x::b` |\n"
        "| Untagged two | do | manual: needs a real display |\n"
    )
    assert plan_rows(plan) == [
        ("t_one", "`tst_x::a`, `tst_x::b`"),
        ("Untagged two", "manual: needs a real display"),
    ]
    known = {("tst_x", "a")}
    exists = lambda a, b: (a, b) in known  # noqa: E731
    assert coverage_errors("`tst_x::a`", exists) == []
    assert coverage_errors("`tst_x::b`", exists) == ["no such test: tst_x::b"]
    assert coverage_errors("manual:", exists) == ["manual row without a reason"]
    assert coverage_errors("TODO", exists)[0].startswith("coverage is neither")
    print("check-test-plan: self-test OK")
    return 0


def main() -> int:
    if "--self-test" in sys.argv[1:]:
        return self_test()
    failed = False
    changes = touched_changes()
    for change in changes:
        for e in check_change(change):
            print(f"FAIL: openspec/changes/{change.name}: {e}")
            failed = True
    print(f"check-test-plan: {len(changes)} change(s) checked")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
