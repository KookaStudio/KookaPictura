#!/usr/bin/env python3
"""Retain Qt Test duration evidence and turn it into a shard plan.

The Qt Test layer already writes one JUnit report per registered entry. Those
reports carry a ``time`` attribute per test function and per suite, and CTest's
own XML carries the wall time per entry. This script keeps that evidence in one
machine-readable file and derives the shard membership and count from it, so a
shard decision cites a duration instead of a hand-maintained list.

Three outputs:

  * ``collect`` reads the per-suite JUnit reports and CTest's ``--output-junit``
    report, merges them into the timing store, writes it, and prints how any
    entry compares with the layer's quarter budget.
  * ``plan`` writes ``cmake/qt-test-shards.json``: one entry per suite that
    needs more than one share, with the duration evidence, the share count, and
    the per-function shares the plan is made of. A suite with no measured
    functions gets a count only, and the runner splits it in declaration order.
  * ``verify`` proves a plan is complete: every shard non-empty, every test
    function of the suite in exactly one shard, and the count not above the
    number of functions.

A plan that would leave a share empty is a configuration error, not a silent
pass: ``shardedMain`` fails such a share, and this script refuses to write one.

Run from the repository root:

    python3 scripts/qt_test_timings.py collect --junit build/qt-test-results/*.xml \
        --ctest-junit build/ctest-junit.xml --out build/qt-test-timings.json
    python3 scripts/qt_test_timings.py plan --timings build/qt-test-timings.json \
        --out cmake/qt-test-shards.json
    python3 scripts/qt_test_timings.py verify --plan cmake/qt-test-shards.json
    python3 scripts/qt_test_timings.py report --timings build/qt-test-timings.json
    python3 scripts/qt_test_timings.py --self-check
"""

from __future__ import annotations

import json
import math
import re
import sys
import tempfile
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TESTS = ROOT / "crates" / "pictura-app" / "cpp" / "tests"
DEFAULT_TIMINGS = ROOT / "build" / "qt-test-timings.json"
DEFAULT_PLAN = ROOT / "cmake" / "qt-test-shards.json"
QUARTER = 0.25
# A suite below this share of the layer budget buys nothing from a second entry.
MIN_SHARD_SECONDS = 1.0

SYNTHETIC_CASES = ("initTestCase", "cleanupTestCase")
SLOT = re.compile(r"^\s*void\s+([A-Za-z_]\w*)\s*\(\s*\)\s*;", re.MULTILINE)
LIFECYCLE = {"init", "cleanup", "initTestCase", "cleanupTestCase"}


def test_functions(source: Path) -> list[str]:
    """The suite's test functions, as `private slots` members without parameters."""
    text = source.read_text(encoding="utf-8", errors="replace")
    found = []
    for name in SLOT.findall(text):
        if name in LIFECYCLE or name.endswith("_data"):
            continue
        found.append(name)
    return found


def parse_junit_timings(xml_text: str) -> dict:
    """(suite_name, seconds, {function: seconds}) from one Qt Test JUnit report."""
    root = ET.fromstring(xml_text)
    suite = root.find("testsuite")
    if suite is None:
        suite = root
    name = suite.get("name") or ""
    functions = {}
    for case in suite.findall("testcase"):
        case_name = case.get("name") or ""
        if case_name in SYNTHETIC_CASES:
            continue
        try:
            functions[case_name] = float(case.get("time") or 0.0)
        except ValueError:
            functions[case_name] = 0.0
    try:
        seconds = float(suite.get("time") or sum(functions.values()))
    except ValueError:
        seconds = sum(functions.values())
    return {"suite": name, "seconds": seconds, "functions": functions}


def parse_ctest_junit(text: str) -> dict[str, float]:
    """CTest entry name -> execution seconds from ``ctest --output-junit``.

    CTest aggregates one ``testcase`` per registered entry, named as the entry
    (``tst_quick_mask_0``) and carrying that entry's wall time, which the
    per-suite JUnit cannot: the suite writes its own class name.
    """
    times: dict[str, float] = {}
    try:
        root = ET.fromstring(text)
    except ET.ParseError:
        return times
    for case in root.iter("testcase"):
        name = case.get("name") or ""
        if not name:
            continue
        try:
            times[name] = float(case.get("time") or 0.0)
        except ValueError:
            times[name] = 0.0
    return times


def merge_store(store: dict, entry: dict) -> dict:
    """Merge one measured entry into the store.

    A sharded suite writes one report per share, each with the same class name
    and a disjoint subset of the functions, so the reports of one collection are
    unioned and their seconds summed. Collection starts from an empty store, so
    this is one run's evidence, not a history.
    """
    merged = dict(store)
    merged["suites"] = {name: dict(fields) for name, fields in store.get("suites", {}).items()}
    name = entry["suite"]
    previous = merged["suites"].get(name, {})
    functions = dict(previous.get("functions") or {})
    for function, seconds in entry["functions"].items():
        functions[function] = round(seconds, 3)
    merged["suites"][name] = {
        "seconds": round(float(previous.get("seconds") or 0.0) + entry["seconds"], 3),
        "functions": {function: functions[function] for function in sorted(functions)},
        "reports": int(previous.get("reports", 0)) + 1,
        "evidence": "measured JUnit/CTest duration",
    }
    return merged


def merge_external_times(store: dict, entry_times: dict[str, float]) -> dict:
    """Record CTest's per-entry wall time beside the JUnit-derived totals."""
    merged = json.loads(json.dumps(store))
    merged.setdefault("entries", {})
    for name, seconds in sorted(entry_times.items()):
        merged["entries"][name] = round(seconds, 3)
    return merged


def layer_seconds(store: dict) -> float:
    stated = store.get("layer_seconds")
    if stated:
        return float(stated)
    return sum(float(suite.get("seconds") or 0.0) for suite in store.get("suites", {}).values())


def shard_plan(store: dict, layer: float | None = None) -> dict:
    """Duration-aware shares: longest functions first, filling the emptiest bin.

    The share count for a suite is the fewest bins that keep one entry under a
    quarter of the layer's measured total, capped by the suite's function count
    so no share is ever empty. The plan cites the measured seconds it used.
    """
    total = layer if layer is not None else layer_seconds(store)
    budget = max(total * QUARTER, MIN_SHARD_SECONDS)
    suites = []
    for name, evidence in sorted(store.get("suites", {}).items()):
        seconds = float(evidence.get("seconds") or 0.0)
        functions = evidence.get("functions") or {}
        if seconds <= 0.0:
            continue
        count = max(1, math.ceil(seconds / budget))
        if count > 1 and seconds < MIN_SHARD_SECONDS:
            count = 1
        if not functions:
            if count > 1:
                suites.append(
                    {
                        "name": name,
                        "count": count,
                        "evidence": f"{seconds:.1f}s measured; {count} shares target "
                        f"{budget:.1f}s per entry (declaration-order split)",
                    }
                )
            continue
        count = min(count, len(functions))
        if count <= 1:
            continue
        bins: list[list[str]] = [[] for _ in range(count)]
        loads = [0.0] * count
        for function, function_seconds in sorted(
            functions.items(), key=lambda item: (-item[1], item[0])
        ):
            target = loads.index(min(loads))
            bins[target].append(function)
            loads[target] += function_seconds
        if any(not share for share in bins):
            raise SystemExit(
                f"qt_test_timings: {name}: a plan of {count} shares leaves an empty share"
            )
        suites.append(
            {
                "name": name,
                "count": count,
                "evidence": f"{seconds:.1f}s measured; {count} shares by longest-function-first "
                f"fill, largest share {max(loads):.1f}s of a {budget:.1f}s quarter",
                "shards": [sorted(share) for share in bins],
            }
        )
    return {"generated_by": "scripts/qt_test_timings.py", "suites": suites}


def carry_forward(plan: dict, baseline: dict) -> dict:
    """Keep a baseline entry whose suite has no fresh measurement.

    Regenerating from a partial timing store must not silently un-shard a suite
    that a previous measured run put in the plan: the baseline entry keeps its
    count and names its evidence as stale rather than vanishing.
    """
    measured = {suite.get("name") for suite in plan.get("suites", [])}
    carried = list(plan.get("suites", []))
    for suite in baseline.get("suites", []):
        if suite.get("name") in measured:
            continue
        if not suite.get("name") or int(suite.get("count") or 0) < 2:
            continue
        entry = dict(suite)
        entry["evidence"] = (
            f"{suite.get('evidence', 'previous plan entry')}; carried forward, "
            "no fresh measurement in this store"
        )
        carried.append(entry)
    return {"generated_by": plan.get("generated_by", "scripts/qt_test_timings.py"), "suites": carried}


def plan_errors(plan: dict, functions_for=None) -> list[str]:
    """Empty shares, duplicate or missing functions, and impossible counts."""
    errors = []
    names = set()
    for suite in plan.get("suites", []):
        name = suite.get("name") or ""
        if not name:
            errors.append("a plan suite has no name")
            continue
        if name in names:
            errors.append(f"{name}: listed twice")
            continue
        names.add(name)
        count = int(suite.get("count") or 0)
        if count < 1:
            errors.append(f"{name}: shard count {count} runs nothing")
            continue
        shares = suite.get("shards")
        if shares is None:
            continue
        if len(shares) != count:
            errors.append(f"{name}: {len(shares)} shares declared for count {count}")
        seen = set()
        for index, share in enumerate(shares):
            if not share:
                errors.append(f"{name}: share {index}/{count} is empty")
                continue
            for function in share:
                if function in seen:
                    errors.append(f"{name}: {function} appears in more than one share")
                seen.add(function)
        if functions_for is not None:
            known = set(functions_for(name))
            if known:
                missing = sorted(known - seen)
                unknown = sorted(seen - known)
                if missing:
                    errors.append(f"{name}: no share runs {', '.join(missing)}")
                if unknown:
                    errors.append(f"{name}: shares name unknown functions {', '.join(unknown)}")
    return errors


def source_functions(name: str) -> list[str]:
    source = TESTS / f"{name}.cpp"
    return test_functions(source) if source.is_file() else []


def longest_entry_lines(store: dict, budget: float | None = None) -> list[str]:
    """One line per measured entry vs the quarter-of-the-layer budget."""
    total = layer_seconds(store)
    limit = budget if budget is not None else total * QUARTER
    lines = []
    for name, seconds in sorted(
        (store.get("entries") or {}).items(), key=lambda item: -item[1]
    ):
        verdict = "over" if seconds > limit else "within"
        lines.append(f"  {name:<40}{seconds:8.1f}s  {verdict} the {limit:.1f}s quarter")
    return lines


def load_json(path: str | None, default):
    if not path:
        return default
    try:
        return json.loads(Path(path).read_text(encoding="utf-8"))
    except OSError:
        return default


def write_json(path: str, data) -> None:
    target = Path(path)
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(json.dumps(data, indent=2, sort_keys=False) + "\n", encoding="utf-8")


def read_junits(paths) -> dict:
    store: dict = {"suites": {}, "entries": {}}
    for path in paths:
        try:
            entry = parse_junit_timings(Path(path).read_text(encoding="utf-8"))
        except (OSError, ET.ParseError) as error:
            print(f"qt_test_timings: skipping {path}: {error}", file=sys.stderr)
            continue
        if entry["functions"]:
            store = merge_store(store, entry)
    return store


def cmd_collect(args) -> int:
    store = read_junits(args.junit or [])
    if args.ctest_junit and Path(args.ctest_junit).is_file():
        try:
            times = parse_ctest_junit(Path(args.ctest_junit).read_text(encoding="utf-8"))
        except OSError as error:
            print(f"qt_test_timings: {error}", file=sys.stderr)
            return 1
        store = merge_external_times(store, times)
    if not store["suites"]:
        print("qt_test_timings: no timed suites in the given reports", file=sys.stderr)
        return 1
    if args.layer_seconds is not None:
        store["layer_seconds"] = round(args.layer_seconds, 3)
    if args.out:
        write_json(args.out, store)
    print("qt_test_timings: retained " f"{len(store['suites'])} suites ({layer_seconds(store):.1f}s)")
    lines = longest_entry_lines(store)
    if lines:
        print("qt_test_timings: longest entries")
        for line in lines[:10]:
            print(line)
    return 0


def cmd_plan(args) -> int:
    store = load_json(args.timings, {"suites": {}})
    plan = shard_plan(store, args.layer_seconds)
    baseline = load_json(args.out, {"suites": []})
    plan = carry_forward(plan, baseline)
    plan["suites"].sort(key=lambda suite: suite.get("name") or "")
    errors = plan_errors(plan, source_functions)
    if errors:
        for error in errors:
            print(f"qt_test_timings: {error}", file=sys.stderr)
        return 1
    if args.out:
        write_json(args.out, plan)
    for suite in plan["suites"]:
        print(f"qt_test_timings: {suite['name']}: {suite['evidence']}")
    return 0


def cmd_verify(args) -> int:
    plan = load_json(args.plan, {})
    errors = plan_errors(plan, source_functions)
    if errors:
        for error in errors:
            print(f"qt_test_timings: {error}", file=sys.stderr)
        return 1
    print(f"qt_test_timings: plan OK ({len(plan.get('suites', []))} sharded suites)")
    return 0


def cmd_report(args) -> int:
    store = load_json(args.timings, {"suites": {}})
    lines = longest_entry_lines(store)
    if not lines:
        print("qt_test_timings: no entry times retained yet")
        return 0
    print(f"qt_test_timings: layer {layer_seconds(store):.1f}s; entries:")
    for line in lines:
        print(line)
    return 0


def check_ci_wiring(root: Path = ROOT) -> list[str]:
    """The Qt-aware CI job keeps the evidence and verifies the plan and waits."""
    workflow = root / ".github" / "workflows" / "ci.yml"
    guard = root / "scripts" / "guard.sh"
    runner = root / "scripts" / "test-report.sh"
    errors = []
    if not workflow.is_file():
        return ["ci.yml is missing"]
    text = workflow.read_text(encoding="utf-8")
    for needle in (
        "qt-test-timings",
        "scripts/test-report.sh",
        "scripts/qt_test_timings.py verify",
        "scripts/check-qt-test-waits.py",
    ):
        if needle not in text:
            errors.append(f"ci.yml: the Qt job does not run {needle}")
    if not runner.is_file() or "scripts/qt_test_timings.py collect" not in runner.read_text(encoding="utf-8"):
        errors.append("test-report.sh: does not collect Qt timing evidence")
    if not guard.is_file():
        errors.append("guard.sh is missing")
    elif "check-qt-test-waits.py" not in guard.read_text(encoding="utf-8"):
        errors.append("guard.sh: does not run check-qt-test-waits.py")
    return errors


def self_test() -> int:
    junit = """<testsuite name="QuickMaskTest" time="12.0">
  <testcase name="initTestCase" classname="QuickMaskTest" time="0.1"/>
  <testcase name="a" classname="QuickMaskTest" time="6.0"/>
  <testcase name="b" classname="QuickMaskTest" time="4.0"/>
  <testcase name="c" classname="QuickMaskTest" time="2.0"/>
  <testcase name="cleanupTestCase" classname="QuickMaskTest" time="0.1"/>
</testsuite>"""
    entry = parse_junit_timings(junit)
    assert entry["suite"] == "QuickMaskTest", entry
    assert entry["functions"] == {"a": 6.0, "b": 4.0, "c": 2.0}, entry
    assert entry["seconds"] == 12.0, entry

    store = merge_store({"suites": {}, "entries": {}}, entry)
    assert store["suites"]["QuickMaskTest"]["reports"] == 1, store
    # A second share of the same suite carries disjoint functions: union them and
    # sum the seconds, or a sharded suite's real cost is lost.
    second = {
        "suite": "QuickMaskTest",
        "seconds": 8.0,
        "functions": {"d": 8.0},
    }
    store = merge_store(store, second)
    assert store["suites"]["QuickMaskTest"]["reports"] == 2, store
    assert store["suites"]["QuickMaskTest"]["seconds"] == 20.0, store
    assert set(store["suites"]["QuickMaskTest"]["functions"]) == {"a", "b", "c", "d"}, store

    # A 20s suite in a 40s layer must not produce an entry over the 10s quarter:
    # two shares, the longest function first, and d joins the emptiest bin.
    plan = shard_plan(store, layer=40.0)
    sharded = {suite["name"]: suite for suite in plan["suites"]}
    assert sharded["QuickMaskTest"]["count"] == 2, sharded
    assert sharded["QuickMaskTest"]["shards"] == [["c", "d"], ["a", "b"]], sharded
    assert plan_errors(plan, lambda name: ["a", "b", "c", "d"]) == []
    assert "20.0s measured" in sharded["QuickMaskTest"]["evidence"], sharded

    # A duration that suggests more shares than the suite has functions is
    # capped at the function count, so the planner never writes an empty share.
    capped = shard_plan(
        {"suites": {"Tiny": {"seconds": 100.0, "functions": {"a": 100.0}}}},
        layer=100.0,
    )
    assert capped["suites"] == [], capped

    empty = {"suites": [{"name": "X", "count": 2, "shards": [["a"], []]}]}
    assert any("empty" in e for e in plan_errors(empty)), plan_errors(empty)
    duplicate = {"suites": [{"name": "X", "count": 2, "shards": [["a"], ["a"]]}]}
    assert any("more than one" in e for e in plan_errors(duplicate)), plan_errors(duplicate)
    missing = {"suites": [{"name": "X", "count": 2, "shards": [["a"], ["b"]]}]}
    assert any("no share runs c" in e for e in plan_errors(missing, lambda _: ["a", "b", "c"]))
    unknown = {"suites": [{"name": "X", "count": 1, "shards": [["zz"]]}]}
    assert any("unknown functions" in e for e in plan_errors(unknown, lambda _: ["a"]))

    # A suite with no per-function evidence still gets a count, not empty shares.
    counted = shard_plan({"suites": {"Big": {"seconds": 40.0, "functions": {}}}}, layer=40.0)
    assert counted["suites"][0]["count"] == 4, counted
    assert "shards" not in counted["suites"][0], counted
    assert plan_errors(counted, lambda _: ["a", "b", "c", "d"]) == []

    # A partial store keeps a baseline entry instead of silently un-sharding it.
    carried = carry_forward(
        {"suites": []}, {"suites": [{"name": "Old", "count": 3, "evidence": "90s measured"}]}
    )
    assert carried["suites"][0]["count"] == 3, carried
    assert "carried forward" in carried["suites"][0]["evidence"], carried
    kept = carry_forward(
        {"suites": [{"name": "Old", "count": 2, "shards": [["a"], ["b"]]}]},
        {"suites": [{"name": "Old", "count": 4}]},
    )
    assert kept["suites"][0]["count"] == 2, kept

    ctest = """<testsuites>
  <testsuite name="ctest" tests="1">
    <testcase name="tst_quick_mask_0" classname="ctest" time="3.5"/>
  </testsuite>
</testsuites>"""
    assert parse_ctest_junit(ctest) == {"tst_quick_mask_0": 3.5}, parse_ctest_junit(ctest)

    merged = merge_external_times({"suites": {}}, {"tst_quick_mask_0": 3.5})
    lines = longest_entry_lines(merged, budget=1.0)
    assert "over" in lines[0], lines

    assert test_functions(Path(__file__)) == []

    # The committed plan stays complete and every suite it names is a source.
    plan = json.loads(DEFAULT_PLAN.read_text(encoding="utf-8"))
    assert plan_errors(plan, source_functions) == [], plan_errors(plan, source_functions)
    assert any(suite["name"] == "tst_quick_mask" for suite in plan["suites"]), plan

    assert check_ci_wiring() == [], check_ci_wiring()

    # A measured layer wall time is what the quarter budget is a quarter of.
    with tempfile.TemporaryDirectory() as tmp:
        report = Path(tmp) / "suite.xml"
        report.write_text(
            '<testsuite name="S" time="3.0">'
            '<testcase name="a" classname="S" time="3.0"/></testsuite>',
            encoding="utf-8",
        )
        out = Path(tmp) / "store.json"
        argv = _parse(["collect", "--junit", str(report), "--out", str(out),
                       "--layer-seconds", "40"])
        assert cmd_collect(argv) == 0
        collected = json.loads(out.read_text(encoding="utf-8"))
        assert collected["layer_seconds"] == 40.0, collected
    print("qt_test_timings: self-test OK")
    return 0


def _parser():
    import argparse

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-check", action="store_true")
    sub = parser.add_subparsers(dest="command")
    collect = sub.add_parser("collect")
    collect.add_argument("--junit", nargs="*", default=[])
    collect.add_argument("--ctest-junit", default=None)
    collect.add_argument(
        "--layer-seconds",
        type=float,
        default=None,
        help="the CTest stage's measured wall time, the quarter-budget base",
    )
    collect.add_argument("--out", default=str(DEFAULT_TIMINGS))
    plan = sub.add_parser("plan")
    plan.add_argument("--timings", default=str(DEFAULT_TIMINGS))
    plan.add_argument("--layer-seconds", type=float, default=None)
    plan.add_argument("--out", default=str(DEFAULT_PLAN))
    verify = sub.add_parser("verify")
    verify.add_argument("--plan", default=str(DEFAULT_PLAN))
    report = sub.add_parser("report")
    report.add_argument("--timings", default=str(DEFAULT_TIMINGS))
    return parser


def _parse(argv):
    return _parser().parse_args(argv)


def main(argv=None) -> int:
    parser = _parser()
    args = parser.parse_args(argv)
    if args.self_check:
        return self_test()
    if args.command == "collect":
        return cmd_collect(args)
    if args.command == "plan":
        return cmd_plan(args)
    if args.command == "verify":
        return cmd_verify(args)
    if args.command == "report":
        return cmd_report(args)
    parser.print_help()
    return 2


if __name__ == "__main__":
    sys.exit(main())
