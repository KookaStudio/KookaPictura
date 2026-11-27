#!/usr/bin/env python3
"""Fail when the gate manifest drifts from the tree or its placement contract."""
from __future__ import annotations

from contextlib import redirect_stderr, redirect_stdout
from copy import deepcopy
from io import StringIO
from pathlib import Path
import sys
import tempfile

from gate_manifest import (
    CLI_PLACEMENTS,
    MANDATORY_CI,
    ROOT,
    ManifestError,
    announce_deferred,
    execute,
    load,
    script_paths,
    select,
    validate,
)


def discovered(root: Path) -> list[str]:
    scripts = root / "scripts"
    return sorted(
        path.relative_to(root).as_posix()
        for path in scripts.glob("check-*")
        if path.suffix in {".py", ".sh"}
    )


def coverage_errors(rows: list[dict], root: Path) -> list[str]:
    expected = discovered(root)
    actual = script_paths(rows)
    missing = [path for path in expected if path not in actual]
    stale = [path for path in actual if path not in expected and path.startswith("scripts/check-")]
    errors = []
    if missing:
        errors.append(f"unlisted check scripts: {missing}")
    if stale:
        errors.append(f"stale check scripts: {stale}")
    return errors


def tree_errors(root: Path = ROOT) -> list[str]:
    try:
        rows = load(root / "scripts" / "checks.toml")
    except (OSError, ValueError) as error:
        return [str(error)]
    return [*validate(rows, root), *coverage_errors(rows, root)]


def sample_row(check_id: str = "planted", **changes) -> dict:
    row = {
        "id": check_id,
        "scope": "guards",
        "cost": "cheap",
        "phase": "parallel",
        "command": "python3 scripts/check-gate-manifest.py --self-test",
        "script": "scripts/check-gate-manifest.py",
        "local_fast": True,
        "local_full": True,
        "ci_guards": True,
    }
    row.update(changes)
    return row


def expect_error(rows: list[dict], needle: str) -> None:
    errors = validate(rows)
    assert any(needle in error for error in errors), (needle, errors)


def unlisted_script_is_rejected(rows: list[dict]) -> None:
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        (root / "scripts").mkdir()
        (root / "scripts" / "check-planted.py").write_text("", encoding="utf-8")
        errors = coverage_errors(rows, root)
        assert any("check-planted.py" in error for error in errors), errors


def stale_and_malformed_entries_are_rejected(rows: list[dict]) -> None:
    stale = deepcopy(rows)
    stale[0]["scripts"] = ["scripts/check-absent.py"]
    expect_error(stale, "missing script scripts/check-absent.py")

    duplicate = deepcopy(rows)
    duplicate.append(deepcopy(duplicate[0]))
    expect_error(duplicate, "duplicate id")
    expect_error(duplicate, "duplicate script path")

    malformed = deepcopy(rows)
    malformed[0]["local_fast"] = "yes"
    malformed[0]["scope"] = []
    malformed[0]["unexpected"] = True
    expect_error(malformed, "local_fast must be boolean")
    expect_error(malformed, "invalid scope")
    expect_error(malformed, "unknown fields")

    unsorted = deepcopy(rows)
    unsorted.reverse()
    expect_error(unsorted, "not sorted")


def mandatory_ci_cannot_shrink(rows: list[dict]) -> None:
    for check_id in sorted(MANDATORY_CI):
        reduced = [row for row in rows if row["id"] != check_id]
        expect_error(reduced, check_id)
    disabled = deepcopy(rows)
    for row in disabled:
        if row["id"] == "icon-provenance":
            row["ci_guards"] = False
            row["scope"] = "manual"
            row["local_fast"] = False
            row["local_full"] = False
    expect_error(disabled, "icon-provenance")


def local_only_placement_is_rejected(rows: list[dict]) -> None:
    changed = deepcopy(rows)
    for row in changed:
        if row["id"] == "fmt":
            row["ci_build"] = False
    expect_error(changed, "local-only")

    escape = deepcopy(rows)
    for row in escape:
        if row["id"] == "fmt":
            row["id"] = "planted"
    expect_error(escape, "unknown build-CI equivalent")


def ci_entry_requires_local_full(rows: list[dict]) -> None:
    changed = deepcopy(rows)
    for row in changed:
        if row["id"] == "guard":
            row["local_full"] = False
            row["local_fast"] = False
            row["docs_only"] = False
    expect_error(changed, "ci_guards requires local_full")


def new_entry_selects_both_runners() -> None:
    assert {"local-fast", "local-full", "docs-only", "ci-guards"} <= set(CLI_PLACEMENTS)
    row = sample_row()
    assert [item["id"] for item in select([row], "local_full", full_local=True)] == ["planted"]
    assert [item["id"] for item in select([row], "ci_guards")] == ["planted"]


def full_local_requires_explicit_flag() -> None:
    row = sample_row(local_fast=False)
    assert select([row], "local_fast") == []
    notice = StringIO()
    with redirect_stderr(notice):
        announce_deferred([row])
    assert "deferred planted: enable PICTURA_FULL_GATES=1" in notice.getvalue()
    try:
        select([row], "local_full")
    except ManifestError as error:
        assert "PICTURA_FULL_GATES=1" in str(error)
    else:
        raise AssertionError("local-full selection accepted no explicit flag")
    assert [item["id"] for item in select([row], "local_full", full_local=True)] == ["planted"]
    assert [item["id"] for item in select([row], "ci_guards")] == ["planted"]


def docs_only_selection_skips_builds(rows: list[dict]) -> None:
    selected = {row["id"] for row in select(rows, "docs_only")}
    assert {"guard", "archive", "test-plan", "selftest-codes", "openspec"} <= selected
    assert not {"fmt", "clippy", "build-tree", "tests"} & selected


def local_full_contains_fast(rows: list[dict]) -> None:
    fast = {row["id"] for row in select(rows, "local_fast")}
    full = {row["id"] for row in select(rows, "local_full", full_local=True)}
    assert fast <= full
    assert {row["id"] for row in rows if row["ci_guards"]} <= full


def mandatory_local_checks_remain(rows: list[dict]) -> None:
    for placement in ("local_fast", "local_full"):
        selected = {row["id"] for row in select(rows, placement, full_local=True)}
        assert {"worktree-hook", "archive", "build-lock"} <= selected


def serial_runner_stops_on_failure() -> None:
    checks = [
        sample_row("first", command="printf first"),
        sample_row("second", command="printf second; exit 7"),
        sample_row("third", command="printf third"),
    ]
    output = StringIO()
    with redirect_stdout(output):
        rc = execute(
            checks,
            root=ROOT,
            placement="ci_guards",
            jobs=1,
            stop_on_failure=True,
        )
    assert rc == 1
    assert "first=ok" in output.getvalue()
    assert "second=FAILED (rc=7)" in output.getvalue()
    assert "  second" in output.getvalue()
    assert "third" not in output.getvalue()


def parallel_runner_collects_all_results() -> None:
    passing = [
        sample_row("first", command="printf first"),
        sample_row("second", command="printf second"),
    ]
    output = StringIO()
    with redirect_stdout(output):
        rc = execute(passing, root=ROOT, placement="local_fast", jobs=2)
    assert rc == 0
    assert "first=ok" in output.getvalue()
    assert "second=ok" in output.getvalue()
    assert output.getvalue().index("first=ok") < output.getvalue().index("second=ok")

    checks = [
        sample_row("pass", command="printf passed"),
        sample_row("fail", command="printf failed; exit 3"),
    ]
    output = StringIO()
    with redirect_stdout(output):
        rc = execute(checks, root=ROOT, placement="local_fast", jobs=2)
    assert rc == 1
    assert "pass=ok" in output.getvalue()
    assert "fail=FAILED (rc=3)" in output.getvalue()
    assert "  failed" in output.getvalue()


def runners_read_the_manifest() -> None:
    fast = (ROOT / "scripts" / "verify-fast.sh").read_text(encoding="utf-8")
    guards = (ROOT / ".github" / "workflows" / "guards.yml").read_text(encoding="utf-8")
    assert 'gate_manifest.py run "$placement"' in fast
    assert "0) placement=local-fast" in fast
    assert "gate_manifest.py run docs-only" in fast
    assert "gate_manifest.py run ci-guards" in guards
    assert "scripts/check-" not in fast
    assert "scripts/check-" not in guards


def self_test() -> int:
    rows = load()
    errors = validate(rows)
    assert not errors, errors
    assert not tree_errors(), tree_errors()
    unlisted_script_is_rejected(rows)
    stale_and_malformed_entries_are_rejected(rows)
    mandatory_ci_cannot_shrink(rows)
    local_only_placement_is_rejected(rows)
    ci_entry_requires_local_full(rows)
    new_entry_selects_both_runners()
    full_local_requires_explicit_flag()
    docs_only_selection_skips_builds(rows)
    local_full_contains_fast(rows)
    mandatory_local_checks_remain(rows)
    serial_runner_stops_on_failure()
    parallel_runner_collects_all_results()
    runners_read_the_manifest()
    print("check-gate-manifest: self-test OK")
    return 0


def main() -> int:
    if "--self-test" in sys.argv[1:]:
        return self_test()
    errors = tree_errors()
    if errors:
        for error in errors:
            print(f"FAIL: {error}")
        return 1
    print("check-gate-manifest: OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
