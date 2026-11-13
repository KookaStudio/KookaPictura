#!/usr/bin/env python3
"""Apply the local verification CPU budget to a gate or command.

Usage:
    scripts/cpu-budget.py exec -- <command...>
    scripts/cpu-budget.py --self-check

The budget is max(1, floor(allowed_cpus * percent / 100)); percent defaults to
80 and is overridden with PICTURA_CPU_BUDGET_PERCENT (1-100). On a platform
with process-affinity control the helper restricts the process to the lowest
budget CPUs before exec; everywhere it exports PICTURA_JOBS and the per-tool
job variables.
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
from typing import Iterable, Sequence

DEFAULT_PERCENT = 80
JOB_VARIABLES = (
    "CARGO_BUILD_JOBS",
    "CMAKE_BUILD_PARALLEL_LEVEL",
    "CTEST_PARALLEL_LEVEL",
    "NEXTEST_TEST_THREADS",
)
MARKER = "PICTURA_CPU_BUDGET_APPLIED"
CHILD_PROBE = """
import json, os
print(json.dumps({
    "marker": os.environ.get("PICTURA_CPU_BUDGET_APPLIED"),
    "jobs": os.environ.get("PICTURA_JOBS"),
    "cargo": os.environ.get("CARGO_BUILD_JOBS"),
    "cmake": os.environ.get("CMAKE_BUILD_PARALLEL_LEVEL"),
    "ctest": os.environ.get("CTEST_PARALLEL_LEVEL"),
    "nextest": os.environ.get("NEXTEST_TEST_THREADS"),
    "affinity": sorted(os.sched_getaffinity(0))
    if hasattr(os, "sched_getaffinity") else None,
}))
"""


class BudgetError(Exception):
    """A configuration value that cannot produce a budget."""


def validate_percent(raw: str | None) -> int:
    if raw is None:
        return DEFAULT_PERCENT
    if not isinstance(raw, str) or not raw.isascii() or not raw.isdigit():
        raise BudgetError(
            f"invalid PICTURA_CPU_BUDGET_PERCENT={raw!r} "
            "(want a decimal integer from 1 through 100)"
        )
    value = int(raw)
    if not 1 <= value <= 100:
        raise BudgetError(
            f"invalid PICTURA_CPU_BUDGET_PERCENT={raw!r} "
            "(want a decimal integer from 1 through 100)"
        )
    return value


def budget_for(allowed_cpus: int, percent: int) -> int:
    return max(1, allowed_cpus * percent // 100)


def choose_slice(allowed: Iterable[int], budget: int) -> list[int]:
    return sorted(allowed)[:budget]


def allowed_cpus() -> int:
    if hasattr(os, "sched_getaffinity"):
        return len(os.sched_getaffinity(0))
    process_cpu_count = getattr(os, "process_cpu_count", None)
    if process_cpu_count is not None:
        return process_cpu_count()
    return os.cpu_count() or 1


def current_mask() -> list[int] | None:
    if not hasattr(os, "sched_getaffinity"):
        return None
    return sorted(os.sched_getaffinity(0))


def apply_slice(budget: int) -> bool:
    mask = current_mask()
    if mask is None:
        return False
    try:
        os.sched_setaffinity(0, choose_slice(mask, budget))
    except OSError as error:
        print(
            f"cpu-budget: warning: could not apply the CPU slice: {error}",
            file=sys.stderr,
        )
        return False
    return True


def export_jobs(budget: int) -> None:
    os.environ["PICTURA_JOBS"] = str(budget)
    for name in JOB_VARIABLES:
        os.environ.setdefault(name, str(budget))
    os.environ[MARKER] = "1"


def exec_command(argv: Sequence[str]) -> int:
    percent = validate_percent(os.environ.get("PICTURA_CPU_BUDGET_PERCENT"))
    budget = budget_for(allowed_cpus(), percent)
    if os.environ.get(MARKER) == "1":
        inherited = os.environ.get("PICTURA_JOBS")
        if inherited and inherited.isdigit() and int(inherited) > 0:
            budget = int(inherited)
    else:
        apply_slice(budget)
    export_jobs(budget)
    try:
        os.execvp(argv[0], list(argv))
    except OSError as error:
        print(f"cpu-budget: cannot run {argv[0]!r}: {error}", file=sys.stderr)
        return 127
    return 127


def _clean_environment() -> dict[str, str]:
    cleared = {MARKER, "PICTURA_JOBS", "PICTURA_CPU_BUDGET_PERCENT", *JOB_VARIABLES}
    return {key: value for key, value in os.environ.items() if key not in cleared}


def _run_helper(
    environment: dict[str, str],
) -> tuple[dict[str, object], str]:
    result = subprocess.run(
        [
            sys.executable,
            os.path.abspath(__file__),
            "exec",
            "--",
            sys.executable,
            "-c",
            CHILD_PROBE,
        ],
        env=environment,
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        raise AssertionError(
            f"helper exited {result.returncode}: {result.stderr.strip()}"
        )
    return json.loads(result.stdout), result.stderr


def self_check() -> int:
    if sys.flags.optimize:
        print(
            "cpu-budget: self-check requires assertions; run without -O",
            file=sys.stderr,
        )
        return 2
    assert budget_for(16, 80) == 12
    assert budget_for(3, 80) == 2
    assert budget_for(1, 80) == 1
    assert budget_for(16, 100) == 16
    assert validate_percent(None) == 80
    assert validate_percent("1") == 1
    assert validate_percent("100") == 100
    for bad in ("0", "101", "abc", "", "1.5", " 80x", "1_0", " 80 ", "+80"):
        try:
            validate_percent(bad)
        except BudgetError:
            pass
        else:
            raise AssertionError(f"accepted invalid percent {bad!r}")
    assert choose_slice(range(16), 12) == list(range(12))
    assert choose_slice(range(8, 16), 6) == list(range(8, 14))
    assert choose_slice([15, 2, 7], 2) == [2, 7]

    clean = _clean_environment()
    budget = budget_for(allowed_cpus(), DEFAULT_PERCENT)
    parent_mask = current_mask()

    def check_slice(
        probe: dict[str, object], stderr: str, mask: list[int]
    ) -> None:
        if probe["affinity"] == choose_slice(mask, budget):
            return
        assert "could not apply the CPU slice" in stderr, probe
        assert probe["affinity"] == mask, probe

    exported, exported_err = _run_helper(clean)
    assert exported["marker"] == "1"
    for key in ("jobs", "cargo", "cmake", "ctest", "nextest"):
        assert exported[key] == str(budget), (key, exported)
    if parent_mask is not None:
        check_slice(exported, exported_err, parent_mask)

    overridden, overridden_err = _run_helper({**clean, "CARGO_BUILD_JOBS": "3"})
    assert overridden["cargo"] == "3"
    for key in ("jobs", "cmake", "ctest", "nextest"):
        assert overridden[key] == str(budget), (key, overridden)
    if parent_mask is not None:
        check_slice(overridden, overridden_err, parent_mask)

    preset, _ = _run_helper({**clean, "PICTURA_JOBS": "99"})
    assert preset["jobs"] == str(budget), preset

    marked, _ = _run_helper({**clean, MARKER: "1"})
    assert marked["marker"] == "1"
    for key in ("jobs", "cargo", "cmake", "ctest", "nextest"):
        assert marked[key] == str(budget), (key, marked)
    if parent_mask is not None:
        assert marked["affinity"] == parent_mask, marked

    inherited, _ = _run_helper({**clean, MARKER: "1", "PICTURA_JOBS": "3"})
    assert inherited["jobs"] == "3", inherited
    for key in ("cargo", "cmake", "ctest", "nextest"):
        assert inherited[key] == "3", (key, inherited)
    if parent_mask is not None:
        assert inherited["affinity"] == parent_mask, inherited

    print("cpu-budget: self-check OK")
    return 0


def usage() -> str:
    return (
        "usage: cpu-budget.py exec -- <command...>\n"
        "       cpu-budget.py --self-check"
    )


def main(argv: Sequence[str]) -> int:
    if not argv or argv[0] in ("-h", "--help"):
        print(usage())
        return 0
    if argv[0] == "--self-check":
        return self_check()
    if argv[0] != "exec":
        print(usage(), file=sys.stderr)
        return 2
    command = list(argv[1:])
    if command and command[0] == "--":
        command = command[1:]
    if not command:
        print(usage(), file=sys.stderr)
        return 2
    try:
        return exec_command(command)
    except BudgetError as error:
        print(f"cpu-budget: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
