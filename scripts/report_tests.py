#!/usr/bin/env python3
"""Unified Kooka Pictura test report (python3 standard library only).

Aggregates three layers into one pytest/vitest-like report:

  * nextest JUnit XML      (--junit)
  * C++ self-test tokens   (--selftest)
  * Qt Test JUnit XML      (--qt-junit), whose per-suite `time` is retained
                             and printed beside the counts

Exits non-zero when any layer failed or a provided input could not be parsed.
Run ``python3 scripts/report_tests.py --self-check`` for the parser fixture
checks.
"""

import argparse
import json
import os
import re
import sys
import tempfile
import xml.etree.ElementTree as ET

RULE = "\u2500" * 48
TOKEN_PREFIX_RE = re.compile(
    r"^pictura self-test: (SUITE|PASS|SKIP|FAIL|SUMMARY)(?:\s+(.*))?$"
)
SUMMARY_RE = re.compile(r"passed=(\d+) failed=(\d+) skipped=(\d+)")


class Counts:
    __slots__ = ("passed", "failed", "skipped")

    def __init__(self, passed=0, failed=0, skipped=0):
        self.passed = passed
        self.failed = failed
        self.skipped = skipped

    def add(self, other):
        self.passed += other.passed
        self.failed += other.failed
        self.skipped += other.skipped

    def total(self):
        return self.passed + self.failed + self.skipped


def sum_counts(suites):
    total = Counts()
    for counts in suites.values():
        total.add(counts)
    return total


def parse_ctest_json(text):
    """Return [(test_name, expected_report_path)] for the registered tests.

    CTest's ``--show-only=json-v1`` output lists every registered test with
    the command it runs. A Qt Test suite is registered with a
    ``-o <path>,junitxml`` argument, so the report it is expected to write is
    read from that command. A test without that argument reports nothing and
    is skipped.
    """
    data = json.loads(text)
    tests = []
    for test in data.get("tests", []):
        name = test.get("name", "")
        command = test.get("command") or []
        report = None
        for index, argument in enumerate(command):
            if argument == "-o" and index + 1 < len(command):
                report = command[index + 1].split(",", 1)[0]
                break
        tests.append((name, report))
    return tests


def missing_ctest_reports(text, errors):
    """Return [(test_name, message)] for the registered suites with no report.

    A suite writes its own JUnit report from inside the test process, so a
    suite that never started -- a missing executable in a stale build tree, or
    a crash before the report was written -- leaves no report behind. Such a
    suite is a failure, not a silently missing line.
    """
    try:
        tests = parse_ctest_json(text)
    except (ValueError, AttributeError) as exc:
        errors.append(f"invalid CTest JSON: {exc}")
        return []
    missing = []
    for name, report in tests:
        if not report:
            continue
        if not os.path.exists(report):
            missing.append(
                (name, f"the registered suite {name} produced no report ({report})")
            )
    return missing


def _node_message(node):
    if node is None:
        return ""
    text = (node.text or "").strip()
    if text:
        return text
    bits = [node.get("message"), node.get("type")]
    return " ".join(bit for bit in bits if bit).strip()


def parse_junit(xml_text):
    """Return ({package: Counts}, entries, [(test_id, message)]).

    ``entries`` is ``[(package, test_name, status, detail)]`` in document order;
    ``status`` is ``"pass"``/``"skip"``/``"fail"``. Skipped cases carry the
    ``"(ignored)"`` detail.
    """
    root = ET.fromstring(xml_text)
    suites = {}
    entries = []
    failures = []
    for testsuite in root.iter("testsuite"):
        name = testsuite.get("name") or ""
        package = name.split("::", 1)[0] or name
        counts = suites.setdefault(package, Counts())
        cases = testsuite.findall("testcase")
        if cases:
            for case in cases:
                test_name = case.get("name") or "?"
                bad = case.find("failure")
                if bad is None:
                    bad = case.find("error")
                if bad is not None:
                    counts.failed += 1
                    container = case.get("classname") or name
                    tid = f"{container}::{test_name}"
                    failures.append((tid, _node_message(bad)))
                    entries.append((package, test_name, "fail", ""))
                elif case.find("skipped") is not None:
                    counts.skipped += 1
                    entries.append((package, test_name, "skip", "(ignored)"))
                else:
                    counts.passed += 1
                    entries.append((package, test_name, "pass", ""))
        else:
            tests = int(testsuite.get("tests") or 0)
            failed = int(testsuite.get("failures") or 0) + int(
                testsuite.get("errors") or 0
            )
            skipped = int(testsuite.get("disabled") or testsuite.get("skipped") or 0)
            counts.passed += max(tests - failed - skipped, 0)
            counts.failed += failed
            counts.skipped += skipped
    return suites, entries, failures


QT_SYNTHETIC_CASES = ("initTestCase", "cleanupTestCase")


def junit_timings(xml_text):
    """Retain the JUnit ``time`` evidence instead of discarding it.

    Returns ``(suite_seconds, cases)``: ``{suite: seconds}`` and
    ``[(suite, case, seconds)]``, in document order. A missing or malformed
    ``time`` is 0.0, so a supplier that omits it cannot fail the report. This is
    the evidence ``scripts/qt_test_timings.py`` turns into a shard plan.
    """
    root = ET.fromstring(xml_text)
    suite_seconds = {}
    cases = []
    for testsuite in root.iter("testsuite"):
        name = testsuite.get("name") or ""
        try:
            total = float(testsuite.get("time") or 0.0)
        except ValueError:
            total = 0.0
        suite_cases = []
        case_total = 0.0
        for case in testsuite.findall("testcase"):
            case_name = case.get("name") or ""
            try:
                seconds = float(case.get("time") or 0.0)
            except ValueError:
                seconds = 0.0
            case_total += seconds
            if case_name not in QT_SYNTHETIC_CASES:
                suite_cases.append((case_name, seconds))
        # A report without a suite-level time still carries case times, setup and
        # teardown included; summing them keeps the evidence for the shard plan.
        if name:
            measured = total or case_total
            if measured:
                suite_seconds[name] = suite_seconds.get(name, 0.0) + measured
        cases.extend((name, case_name, seconds) for case_name, seconds in suite_cases)
    return suite_seconds, cases


def parse_qt_junit(xml_text):
    """Parse a Qt Test JUnit report, dropping the lifecycle pseudo-tests.

    Qt Test wraps every suite with ``initTestCase``/``cleanupTestCase``, which
    are harness setup rather than user tests, so they are removed from both the
    counts and the listing. Returns the same shape as :func:`parse_junit`.
    """
    _, entries, failures = parse_junit(xml_text)
    kept = [entry for entry in entries if entry[1] not in QT_SYNTHETIC_CASES]
    suites = {}
    for package, _name, status, _detail in kept:
        counts = suites.setdefault(package, Counts())
        if status == "pass":
            counts.passed += 1
        elif status == "skip":
            counts.skipped += 1
        else:
            counts.failed += 1
    kept_failures = [
        (tid, message)
        for tid, message in failures
        if tid.rsplit("::", 1)[-1] not in QT_SYNTHETIC_CASES
    ]
    return suites, kept, kept_failures


def parse_selftest_stream(text):
    """Parse one self-test stream into (order, checks, summary).

    Only the machine tokens count. Human ``key=value`` progress lines and
    ``FAIL:`` lines do not match the token keyword grammar. ``order`` lists
    ``(suite, name)`` keys first-seen; ``checks`` maps each key to its latest
    record; ``summary`` is the Counts from a SUMMARY line, or None.
    """
    order = []
    checks = {}
    summary = None
    for raw in text.splitlines():
        match = TOKEN_PREFIX_RE.match(raw.strip())
        if not match:
            continue
        keyword = match.group(1)
        rest = (match.group(2) or "").strip()
        if keyword == "SUMMARY":
            match = SUMMARY_RE.fullmatch(rest)
            if match:
                summary = Counts(int(match.group(1)), int(match.group(2)), int(match.group(3)))
            continue
        if keyword == "SUITE":
            continue
        parts = rest.split(None, 2)
        if len(parts) < 2:
            continue
        key = (parts[0], parts[1])
        tail = parts[2] if len(parts) == 3 else ""
        record = {"status": keyword, "code": "", "message": "", "detail": ""}
        if keyword == "FAIL" and tail:
            bits = tail.split(None, 1)
            record["code"] = bits[0]
            record["message"] = bits[1] if len(bits) == 2 else ""
        elif keyword in ("PASS", "SKIP"):
            record["detail"] = tail
        if key not in checks:
            order.append(key)
        checks[key] = record
    return order, checks, summary


def validate_selftest_stream(text, label):
    errors = []
    counts = Counts()
    summary = None
    summary_count = 0
    closed = False
    for raw in text.splitlines():
        match = TOKEN_PREFIX_RE.match(raw.strip())
        if not match:
            continue
        keyword = match.group(1)
        rest = (match.group(2) or "").strip()
        if closed:
            errors.append(f"{label}: self-test token after SUMMARY")
            if keyword == "SUMMARY":
                summary_count += 1
            continue
        if keyword == "SUMMARY":
            summary_count += 1
            parsed = SUMMARY_RE.fullmatch(rest)
            if parsed is None:
                errors.append(f"{label}: invalid SUMMARY: {raw.strip()}")
            else:
                summary = Counts(int(parsed.group(1)), int(parsed.group(2)), int(parsed.group(3)))
            closed = True
            continue
        if keyword == "PASS":
            counts.passed += 1
        elif keyword == "FAIL":
            counts.failed += 1
        elif keyword == "SKIP":
            counts.skipped += 1
        if keyword in ("PASS", "SKIP", "FAIL"):
            parts = rest.split(None, 2)
            if len(parts) < 2:
                errors.append(f"{label}: malformed {keyword} token: {raw.strip()}")
    if summary_count == 0:
        errors.append(f"{label}: self-test stream has no SUMMARY (truncated)")
    elif summary_count > 1:
        errors.append(f"{label}: self-test stream has multiple SUMMARY tokens")
    if summary is not None and (
        counts.passed != summary.passed
        or counts.failed != summary.failed
        or counts.skipped != summary.skipped
    ):
        errors.append(
            f"{label}: SUMMARY counts {summary.passed}/{summary.failed}/{summary.skipped} "
            f"do not match tokens {counts.passed}/{counts.failed}/{counts.skipped}"
        )
    return errors, summary, counts


def merge_selftest_streams(texts):
    """Merge self-test streams, deduping checks by (suite, name).

    A check present in several runs is counted once. The last stream's status
    wins only among PASS and SKIP: a FAIL in any capture stays a failure, so a
    later passing run cannot erase an earlier red one. Layer totals are
    recomputed from the merged unique checks. Returns ({suite: Counts}, totals,
    [(label, message)], entries) where ``entries`` is
    ``[(suite, name, status, detail)]`` in stream order.
    """
    order = []
    merged = {}
    for text in texts:
        stream_order, checks, _summary = parse_selftest_stream(text)
        for key in stream_order:
            if key not in merged:
                order.append(key)
                merged[key] = checks[key]
                continue
            previous = merged[key]
            if previous["status"] == "FAIL" and checks[key]["status"] != "FAIL":
                continue
            merged[key] = checks[key]

    suites = {}
    entries = []
    failures = []
    for key in order:
        suite, name = key
        record = merged[key]
        counts = suites.setdefault(suite, Counts())
        status = record["status"]
        if status == "PASS":
            counts.passed += 1
            entries.append((suite, name, "pass", record["detail"]))
        elif status == "SKIP":
            counts.skipped += 1
            entries.append((suite, name, "skip", record["detail"]))
        else:
            counts.failed += 1
            entries.append((suite, name, "fail", ""))
            label = f"{suite}::{name}"
            if record["code"]:
                label += f" (exit {record['code']})"
            failures.append((label, record["message"]))

    totals = sum_counts(suites)
    return suites, totals, failures, entries


def selftest_summaries(texts):
    """The parsed SUMMARY of each capture, in the order given, or None."""
    return [parse_selftest_stream(text)[2] for text in texts]


def parse_selftest(text):
    """Return ({suite: Counts}, totals, failures, entries) for one stream."""
    return merge_selftest_streams([text])


def _counts_compact(counts):
    parts = [f"{counts.passed} passed"]
    if counts.skipped:
        parts.append(f"{counts.skipped} skipped")
    if counts.failed:
        parts.append(f"{counts.failed} failed")
    return ", ".join(parts)


def _counts_dot(counts):
    return f"{counts.passed} passed \u00b7 {counts.skipped} skipped \u00b7 {counts.failed} failed"


def _counts_brief(counts):
    parts = [f"{counts.passed} passed"]
    if counts.skipped:
        parts.append(f"{counts.skipped} skipped")
    if counts.failed:
        parts.append(f"{counts.failed} failed")
    return " \u00b7 ".join(parts)


PASS_GLYPH = "\u2713"
FAIL_GLYPH = "\u2717"
SKIP_GLYPH = "\u25cb"


def _style(text, code, color):
    if not color:
        return text
    return f"\x1b[{code}m{text}\x1b[0m"


class Layer:
    def __init__(
        self, title, suites=None, totals=None, entries=None, captures=None, times=None
    ):
        self.title = title
        self.suites = suites or {}
        self.totals = totals if totals is not None else sum_counts(self.suites)
        self.entries = entries or []
        # {suite: seconds} from JUnit's time attribute: retained, not discarded,
        # so the slowest entry is visible and a shard plan can cite it.
        self.times = times or {}
        # [(label, Counts)] for one line per capture, since the captures run
        # different check sets and the merged total is not any one of them.
        self.captures = captures or []

    def empty(self):
        return self.totals.total() == 0 and not self.suites


def _test_lines(tests, color):
    if not tests:
        return []
    width = max(len(name) for name, _, _ in tests)
    lines = []
    for name, status, detail in tests:
        if status == "pass":
            line = f"    {_style(PASS_GLYPH, '32', color)} {name}"
            if detail:
                line += " " * (width - len(name) + 2) + _style(detail, "2", color)
        elif status == "skip":
            line = f"    {_style(SKIP_GLYPH, '33', color)} {name}"
            if detail:
                line += f" {detail}"
        else:
            line = f"    {_style(FAIL_GLYPH, '31', color)} {name}"
        lines.append(line)
    return lines


def _failure_lines(failures, color):
    lines = []
    if failures:
        lines.append("")
        lines.append(_style("FAILURES", "1;31", color))
        for index, (label, message) in enumerate(failures, 1):
            lines.append(f"  {index}) {label}")
            body = message if message.strip() else "(no message)"
            for line in body.splitlines():
                lines.append(f"       {line}")
    return lines


def _render_summary(visible, failures, color):
    labels = []
    for layer in visible:
        labels.extend(layer.suites)
        labels.extend(label for label, _ in layer.captures)
        labels.append("subtotal")
    # +2 keeps one space before the counts even when a label is the longest.
    width = max([24] + [len(label) + 2 for label in labels])

    lines = [_style("pictura test report", "1", color), RULE]
    totals = Counts()
    for layer in visible:
        totals.add(layer.totals)
        lines.append(_style(layer.title, "1", color))
        for name, counts in layer.suites.items():
            lines.append(f"  {name:<{width}}{_counts_compact(counts)}")
        for label, counts in layer.captures:
            lines.append(f"  {label:<{width}}{_counts_compact(counts)}")
        lines.append(f"  {'subtotal':<{width}}{_counts_dot(layer.totals)}")
    lines.append(RULE)
    lines.append(_style(f"TOTAL  {_counts_dot(totals)}", "1", color))
    lines.extend(_failure_lines(failures, color))
    return "\n".join(lines)


def render(layers, failures, color, show_tests=True):
    visible = [layer for layer in layers if not layer.empty()]
    if not show_tests:
        return _render_summary(visible, failures, color)

    lines = [_style("pictura test report", "1", color), RULE]
    totals = Counts()
    last = len(visible) - 1
    for index, layer in enumerate(visible):
        totals.add(layer.totals)
        lines.append(
            _style(f"{layer.title} \u2014 {_counts_brief(layer.totals)}", "1", color)
        )
        by_suite = {}
        for suite, name, status, detail in layer.entries:
            by_suite.setdefault(suite, []).append((name, status, detail))
        for suite, counts in layer.suites.items():
            seconds = layer.times.get(suite)
            suffix = f" {seconds:.1f}s" if seconds else ""
            lines.append(f"  {suite} ({_counts_compact(counts)}){suffix}")
            lines.extend(_test_lines(by_suite.get(suite, []), color))
        for label, counts in layer.captures:
            lines.append(f"  {label} ({_counts_compact(counts)})")
        lines.append(f"  subtotal {_counts_brief(layer.totals)}")
        if index != last:
            lines.append("")
    lines.append(RULE)
    lines.append(_style(f"TOTAL  {_counts_dot(totals)}", "1", color))
    lines.extend(_failure_lines(failures, color))
    return "\n".join(lines)


def _read(path, errors):
    try:
        with open(path, "r", encoding="utf-8", errors="replace") as handle:
            return handle.read()
    except OSError as exc:
        errors.append(f"{path}: {exc.strerror or exc}")
        return None


def build_layers(args, errors):
    layers = []
    failures = []

    if args.junit:
        text = _read(args.junit, errors)
        if text is not None:
            try:
                suites, entries, junit_failures = parse_junit(text)
            except ET.ParseError as exc:
                errors.append(f"{args.junit}: invalid JUnit XML: {exc}")
            else:
                layers.append(Layer("Rust tests (nextest)", suites, entries=entries))
                failures.extend(junit_failures)

    if args.selftest:
        texts = []
        labels = []
        captures = []
        for path in args.selftest:
            text = _read(path, errors)
            if text is None:
                continue
            stream_errors, summary, _raw_counts = validate_selftest_stream(text, path)
            errors.extend(stream_errors)
            if summary is not None and summary.failed:
                _order, checks, _summary = parse_selftest_stream(text)
                if not any(record["status"] == "FAIL" for record in checks.values()):
                    failures.append(
                        (os.path.basename(path), f"SUMMARY reports {summary.failed} failed check(s)")
                    )
            texts.append(text)
            labels.append(os.path.basename(path))
            captures.append(summary)
        if texts:
            suites, totals, selftest_failures, entries = merge_selftest_streams(texts)
            # One capture's own SUMMARY is authoritative: a check that passes and
            # then fails is two tokens but one check, so the deduped count would
            # under-report the passes the run itself counted.
            if len(texts) == 1 and captures[0] is not None:
                totals = captures[0]
            if suites or totals.total():
                layers.append(
                    Layer(
                        "App self-test (offscreen Qt)",
                        suites,
                        totals,
                        entries=entries,
                        captures=[
                            (label, counts)
                            for label, counts in zip(labels, captures)
                            if counts is not None
                        ],
                    )
                )
            failures.extend(selftest_failures)

    if args.qt_junit or args.ctest_json:
        qt_suites = {}
        qt_entries = []
        qt_times = {}
        for path in args.qt_junit or []:
            text = _read(path, errors)
            if text is None:
                continue
            try:
                suites, entries, qt_failures = parse_qt_junit(text)
                suite_seconds, _cases = junit_timings(text)
            except ET.ParseError as exc:
                errors.append(f"{path}: invalid JUnit XML: {exc}")
                continue
            for suite, seconds in suite_seconds.items():
                qt_times[suite] = qt_times.get(suite, 0.0) + seconds
            for suite, counts in suites.items():
                qt_suites.setdefault(suite, Counts()).add(counts)
            qt_entries.extend(entries)
            failures.extend(qt_failures)
        if args.ctest_json:
            text = _read(args.ctest_json, errors)
            if text is not None:
                for name, message in missing_ctest_reports(text, errors):
                    counts = qt_suites.setdefault("no report", Counts())
                    counts.failed += 1
                    qt_entries.append(("no report", name, "fail", ""))
                    failures.append((f"no report::{name}", message))
        if qt_suites:
            layers.append(
                Layer(
                    "Qt shell tests (offscreen)",
                    qt_suites,
                    entries=qt_entries,
                    times=qt_times,
                )
            )

    return layers, failures


def exit_code(failures, errors):
    return 1 if failures or errors else 0


def _color_enabled(mode):
    if mode == "always":
        return True
    if mode == "never":
        return False
    return sys.stdout.isatty()


def _parse_args(argv):
    parser = argparse.ArgumentParser(description="Unified Kooka Pictura test report")
    parser.add_argument("--junit", help="nextest JUnit XML path")
    parser.add_argument(
        "--selftest",
        action="append",
        help="captured self-test output path (repeatable; streams are merged)",
    )
    parser.add_argument(
        "--qt-junit",
        action="extend",
        nargs="+",
        help="Qt Test JUnit XML path(s) (accepts several, repeatable)",
    )
    parser.add_argument(
        "--ctest-json",
        help=(
            "CTest --show-only=json-v1 output; a registered suite that wrote "
            "no report is reported as a failure"
        ),
    )
    parser.add_argument(
        "--color", choices=("auto", "always", "never"), default="auto"
    )
    parser.add_argument("--no-color", action="store_true")
    parser.add_argument(
        "-q",
        "--quiet",
        "--summary",
        action="store_true",
        dest="quiet",
        help="counts only: suites, subtotals, TOTAL, FAILURES (no per-test lines)",
    )
    parser.add_argument("--self-check", action="store_true")
    return parser.parse_args(argv)


PASS_JUNIT = """<testsuites>
  <testsuite name="pictura-core::unittests" tests="2">
    <testcase name="a" classname="pictura-core"/>
    <testcase name="b" classname="pictura-core"/>
  </testsuite>
  <testsuite name="pictura-codec::oracle" tests="1">
    <testcase name="oracle::x" classname="pictura-codec"/>
  </testsuite>
</testsuites>"""

FAIL_JUNIT = """<testsuites>
  <testsuite name="pictura-codec::oracle" tests="2" failures="1">
    <testcase name="oracle::skipped" classname="pictura-codec"><skipped/></testcase>
    <testcase name="oracle::bad" classname="pictura-codec">
      <failure message="boom">assertion `left == right` failed</failure>
    </testcase>
  </testsuite>
</testsuites>"""

QT_JUNIT = """<testsuite name="SmokeTest" tests="3" failures="1">
  <testcase name="initTestCase" classname="SmokeTest" time="0.414"/>
  <testcase name="frameOpensOffscreen" classname="SmokeTest" time="0.000">
    <failure type="fail" message="boom">assertion failed</failure>
  </testcase>
  <testcase name="cleanupTestCase" classname="SmokeTest" time="0.000"/>
</testsuite>"""

# Two registered suites: one whose report exists (the path is filled in with
# this file) and one whose report was never written.
CTEST_JSON = """{
  "kind": "ctestInfo",
  "version": {"major": 1, "minor": 0},
  "tests": [
    {"name": "tst_present",
     "command": ["/bin/tst_present", "-o", "%s,junitxml"]},
    {"name": "tst_gone",
     "command": ["/bin/tst_gone", "-o", "/nonexistent/tst_gone.xml,junitxml"]}
  ]
}"""

SELFTEST = """pictura self-test: document_size=4000
pictura self-test: SUITE core document_size
pictura self-test: PASS core document_size 4000x4000
pictura self-test: SKIP core gpu vk not available
pictura self-test: FAIL: human failure line
pictura self-test: FAIL panel compact_shade 195 panel compact shade
pictura self-test: SUMMARY passed=1 failed=1 skipped=1
"""

SELFTEST_BARE = """pictura self-test: PASS core shared token from bare
pictura self-test: PASS core only_bare only in bare
pictura self-test: SUMMARY passed=2 failed=0 skipped=0
"""

SELFTEST_PSD = """pictura self-test: FAIL core shared 7 boom
pictura self-test: PASS core only_psd only in psd
pictura self-test: SUMMARY passed=1 failed=1 skipped=0
"""


def run_self_check():
    suites, entries, failures = parse_junit(PASS_JUNIT)
    assert suites["pictura-core"].passed == 2, suites
    assert suites["pictura-codec"].passed == 1, suites
    assert not failures, failures
    assert [e[1] for e in entries] == ["a", "b", "oracle::x"], entries

    suites, entries, failures = parse_junit(FAIL_JUNIT)
    assert suites["pictura-codec"].failed == 1, suites
    assert suites["pictura-codec"].skipped == 1, suites
    assert suites["pictura-codec"].passed == 0, suites
    assert len(failures) == 1, failures
    assert failures[0][0] == "pictura-codec::oracle::bad", failures
    assert "assertion" in failures[0][1], failures
    assert entries[0][1:] == ("oracle::skipped", "skip", "(ignored)"), entries
    assert entries[1][1:] == ("oracle::bad", "fail", ""), entries

    # Durations are retained, not discarded. The fixture's suite element has no
    # time, so the suite total falls back to its case times.
    suite_seconds, cases = junit_timings(QT_JUNIT)
    assert suite_seconds == {"SmokeTest": 0.414}, suite_seconds
    assert cases == [("SmokeTest", "frameOpensOffscreen", 0.0)], cases
    timed_xml = (
        '<testsuite name="SmokeTest" time="1.5">'
        '<testcase name="initTestCase" classname="SmokeTest" time="0.4"/>'
        '<testcase name="frameOpensOffscreen" classname="SmokeTest" time="1.1"/>'
        "</testsuite>"
    )
    suite_seconds, cases = junit_timings(timed_xml)
    assert suite_seconds == {"SmokeTest": 1.5}, suite_seconds
    assert cases == [("SmokeTest", "frameOpensOffscreen", 1.1)], cases
    timed = Layer(
        "Qt shell tests (offscreen)",
        {"SmokeTest": Counts(1, 0, 0)},
        entries=[("SmokeTest", "frameOpensOffscreen", "pass", "")],
        times=suite_seconds,
    )
    assert "1.5s" in render([timed], [], False), render([timed], [], False)

    suites, entries, failures = parse_qt_junit(QT_JUNIT)
    assert suites["SmokeTest"].total() == 1, suites
    assert suites["SmokeTest"].failed == 1, suites
    assert suites["SmokeTest"].passed == 0, suites
    assert [e[1] for e in entries] == ["frameOpensOffscreen"], entries
    assert len(failures) == 1, failures
    assert failures[0][0] == "SmokeTest::frameOpensOffscreen", failures
    assert "assertion" in failures[0][1], failures

    errors = []
    missing = missing_ctest_reports(CTEST_JSON % os.path.abspath(__file__), errors)
    assert not errors, errors
    assert [name for name, _ in missing] == ["tst_gone"], missing
    assert "produced no report" in missing[0][1], missing

    errors = []
    missing = missing_ctest_reports("not json at all", errors)
    assert len(errors) == 1, errors
    assert not missing, missing

    suites, totals, failures, entries = parse_selftest(SELFTEST)
    assert suites["core"].passed == 1, suites
    assert suites["core"].skipped == 1, suites
    assert suites["panel"].failed == 1, suites
    assert (totals.passed, totals.failed, totals.skipped) == (1, 1, 1), totals
    assert len(failures) == 1, failures
    assert failures[0][0] == "panel::compact_shade (exit 195)", failures
    assert failures[0][1] == "panel compact shade", failures
    assert entries == [
        ("core", "document_size", "pass", "4000x4000"),
        ("core", "gpu", "skip", "vk not available"),
        ("panel", "compact_shade", "fail", ""),
    ], entries

    suites, totals, failures, entries = merge_selftest_streams(
        [SELFTEST_BARE, SELFTEST_PSD]
    )
    assert (totals.passed, totals.failed, totals.skipped) == (2, 1, 0), totals
    assert suites["core"].total() == 3, suites
    assert len(failures) == 1, failures
    assert failures[0][0] == "core::shared (exit 7)", failures
    assert failures[0][1] == "boom", failures
    assert [e[1] for e in entries] == ["shared", "only_bare", "only_psd"], entries

    suites, totals, _, _ = merge_selftest_streams([SELFTEST_BARE])
    assert (totals.passed, totals.failed, totals.skipped) == (2, 0, 0), totals

    # Truncation: a capture that never reached its SUMMARY, end to end through
    # build_layers (which records it in errors) and exit_code.
    truncated = "pictura self-test: PASS core only_ran ok\n"
    assert parse_selftest_stream(truncated)[2] is None, (
        "a truncated capture must not yield a summary"
    )
    with tempfile.TemporaryDirectory() as tmpdir:
        trunc_path = os.path.join(tmpdir, "truncated.log")
        whole_path = os.path.join(tmpdir, "complete.log")
        with open(trunc_path, "w", encoding="utf-8") as handle:
            handle.write(truncated)
        with open(whole_path, "w", encoding="utf-8") as handle:
            handle.write(SELFTEST_BARE)

        trunc_errors = []
        trunc_layers, trunc_failures = build_layers(
            _parse_args(["--selftest", trunc_path]), trunc_errors
        )
        assert any(
            trunc_path in error and "truncated" in error for error in trunc_errors
        ), trunc_errors
        assert exit_code(trunc_failures, trunc_errors) == 1, trunc_errors
        assert trunc_layers == [] or trunc_layers[0].captures == [], trunc_layers

        whole_errors = []
        whole_layers, whole_failures = build_layers(
            _parse_args(["--selftest", whole_path]), whole_errors
        )
        assert whole_errors == [], whole_errors
        assert exit_code(whole_failures, whole_errors) == 0, whole_failures
        assert len(whole_layers) == 1, whole_layers

    invalid_captures = {
        "partial": "pictura self-test: SUMMARY passed=1\n",
        "trailing": (
            "pictura self-test: PASS core one ok\n"
            "pictura self-test: SUMMARY passed=1 failed=0 skipped=0\n"
            "pictura self-test: PASS core two ok\n"
        ),
        "summary_only_failure": "pictura self-test: SUMMARY passed=0 failed=1 skipped=0\n",
        "multiple": (
            "pictura self-test: SUMMARY passed=0 failed=0 skipped=0\n"
            "pictura self-test: SUMMARY passed=0 failed=0 skipped=0\n"
        ),
    }
    with tempfile.TemporaryDirectory() as tmpdir:
        for name, capture in invalid_captures.items():
            path = os.path.join(tmpdir, f"{name}.log")
            with open(path, "w", encoding="utf-8") as handle:
                handle.write(capture)
            capture_errors = []
            capture_layers, capture_failures = build_layers(
                _parse_args(["--selftest", path]), capture_errors
            )
            assert capture_errors, (name, capture_errors)
            assert exit_code(capture_failures, capture_errors) == 1, name
            if name == "summary_only_failure":
                assert any("SUMMARY reports 1 failed" in message for _, message in capture_failures)

    # One truncated capture among two is still an error, and merging cannot hide
    # the missing SUMMARY.
    def triples(summaries):
        return [None if s is None else (s.passed, s.failed, s.skipped) for s in summaries]

    assert triples(selftest_summaries([SELFTEST_BARE, truncated])) == [
        (2, 0, 0),
        None,
    ], triples(selftest_summaries([SELFTEST_BARE, truncated]))
    assert triples(selftest_summaries([SELFTEST_BARE])) == [(2, 0, 0)]

    # FAIL wins: a red check in one capture is not erased by a pass in another.
    red_bare = "pictura self-test: FAIL core shared 7 boom\n" \
               "pictura self-test: SUMMARY passed=0 failed=1 skipped=0\n"
    green_psd = "pictura self-test: PASS core shared ok\n" \
                "pictura self-test: SUMMARY passed=1 failed=0 skipped=0\n"
    suites, totals, merge_failures, entries = merge_selftest_streams([red_bare, green_psd])
    assert totals.failed == 1, totals
    assert totals.passed == 0, totals
    assert suites["core"].failed == 1, suites
    assert len(merge_failures) == 1, merge_failures
    assert merge_failures[0][0] == "core::shared (exit 7)", merge_failures
    assert [e[1:] for e in entries] == [("shared", "fail", "")], entries

    # And the reverse order is still a failure, not a pass.
    suites, totals, merge_failures, _ = merge_selftest_streams([green_psd, red_bare])
    assert totals.failed == 1, totals
    assert suites["core"].failed == 1, suites

    # Each capture's own SUMMARY is available next to the merged total.
    per_capture = selftest_summaries([SELFTEST_BARE, SELFTEST_PSD])
    assert [(c.passed, c.failed, c.skipped) for c in per_capture] == [
        (2, 0, 0),
        (1, 1, 0),
    ], per_capture

    # A single capture's own SUMMARY is the layer total, so a check that passes
    # and then fails is counted as one pass and one failure, not zero passes.
    pass_then_fail = (
        "pictura self-test: PASS core two_phase ok\n"
        "pictura self-test: FAIL core two_phase 9 late\n"
        "pictura self-test: SUMMARY passed=1 failed=1 skipped=0\n"
    )
    with tempfile.TemporaryDirectory() as tmpdir:
        path = os.path.join(tmpdir, "single.log")
        with open(path, "w", encoding="utf-8") as handle:
            handle.write(pass_then_fail)
        single_errors = []
        single_layers, single_failures = build_layers(
            _parse_args(["--selftest", path]), single_errors
        )
        assert single_errors == [], single_errors
        assert len(single_layers) == 1, single_layers
        assert (single_layers[0].totals.passed, single_layers[0].totals.failed) == (1, 1), (
            single_layers[0].totals
        )
        assert exit_code(single_failures, single_errors) == 1, single_failures

    # Per-capture lines render beside the merged subtotal.
    capture_layer = Layer(
        "App self-test (offscreen Qt)",
        {"core": Counts(2, 1, 0)},
        entries=[("core", "shared", "fail", "")],
        captures=[("bare.log", Counts(2, 0, 0)), ("psd.log", Counts(1, 1, 0))],
    )
    rendered = render([capture_layer], [("core::shared (exit 7)", "boom")], False)
    assert "bare.log (2 passed" in rendered, rendered
    assert "psd.log (1 passed" in rendered, rendered
    assert "subtotal" in rendered, rendered

    _, _, human, _ = parse_selftest(
        "pictura self-test: document_size=4000\npictura self-test: FAIL: nope"
    )
    assert not human, human

    assert exit_code([], []) == 0
    assert exit_code([("x", "")], []) == 1
    assert exit_code([], ["e"]) == 1

    layer = [
        Layer(
            "Rust tests (nextest)",
            {"pictura-core": Counts(2, 0, 1)},
            entries=[
                ("pictura-core", "tests::known_pass", "pass", "detail=1"),
                ("pictura-core", "tests::known_skip", "skip", "(ignored)"),
            ],
        )
    ]
    listing = render(layer, [], False)
    assert "tests::known_pass" in listing, listing
    assert PASS_GLYPH in listing, listing
    assert "tests::known_skip" in listing and SKIP_GLYPH in listing, listing
    assert "\x1b[" not in listing, "color never leaked ANSI"

    failed = render(layer, [("pictura-core::bad", "boom")], False)
    assert "FAILURES" in failed and "pictura-core::bad" in failed, failed

    quiet = render(layer, [], False, show_tests=False)
    assert "tests::known_pass" not in quiet, quiet
    assert "pictura-core" in quiet and "subtotal" in quiet, quiet

    assert "\x1b[" in render(layer, [], True), "color always missing ANSI"

    print("report_tests.py: self-check OK")
    return 0


def main(argv=None):
    args = _parse_args(argv)
    if args.self_check:
        return run_self_check()

    errors = []
    layers, failures = build_layers(args, errors)
    color = _color_enabled("never" if args.no_color else args.color)
    sys.stdout.write(render(layers, failures, color, not args.quiet) + "\n")
    for error in errors:
        print(f"report_tests.py: error: {error}", file=sys.stderr)
    return exit_code(failures, errors)


if __name__ == "__main__":
    sys.exit(main())
