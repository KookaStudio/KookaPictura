#!/usr/bin/env python3
"""Prove the shared Qt execution contract in ``scripts/test-report.sh``.

python3 standard library only. Every check builds a throwaway repository, copies
the real executor and reporter, and puts fake ``cmake``, ``cargo``, ``ctest``
and ``pictura`` commands on ``PATH``. The fakes append start/end events to one
log, so ordering, overlap, single-writer behavior, capture separation, failure
identity, stale cleanup, timings, and signal cleanup are asserted against the
real shell script instead of a model of it.

The fake build refuses a second writer, so a second concurrent ``cmake --build``
is a recorded violation, not a silent pass.

Run from the repository root: python3 scripts/test_report_execution.py --self-check
"""

from __future__ import annotations

import os
import re
import shutil
import signal
import subprocess
import sys
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SCRIPTS = ROOT / "scripts"

COPY = ("test-report.sh", "report_tests.py", "qt_test_timings.py")

# The executor only uses build_lock_enter_managed from the real helper. The lock
# behavior has its own check, so the sandbox gets a stub: no cgroup routing and
# no dependency on the agent launcher, which keeps the fake run hermetic.
BUILD_LOCK_STUB = """#!/usr/bin/env bash
build_lock_enter_managed() { :; }
"""

FAKE_CMAKE = """#!/usr/bin/env bash
set -u
log() { printf '%s %s %s\\n' "$1" "$2" "$(date +%s%N)" >>"$EVENT_LOG"; }
mkdir -p "$STATE"
if [ "${1:-}" = "--build" ]; then
    target="tail"
    for arg in "$@"; do
        [ "$arg" = "pictura" ] && target="pictura"
    done
    if ! mkdir "$STATE/build.lock" 2>/dev/null; then
        log violation second-writer
        exit 3
    fi
    log "$target-start" build
    sleep "${FAKE_BUILD_SECONDS:-0}"
    log "$target-end" build
    rmdir "$STATE/build.lock" 2>/dev/null || true
    case "${FAKE_FAIL:-}" in
        "$target") log "$target-failed" build; exit 9 ;;
    esac
    if [ "${FAKE_CARGO_JSON:-}" = "1" ]; then
        printf '{"reason":"build-finished","success":true,"target":"%s"}\\n' "$target"
    fi
    exit 0
fi
log configure-start build
sleep "${FAKE_CONFIGURE_SECONDS:-0}"
if [ "${FAKE_FAIL:-}" = "configure" ]; then
    log configure-failed build
    exit 4
fi
mkdir -p build
log configure-end build
exit 0
"""

FAKE_CARGO = """#!/usr/bin/env bash
set -u
log() { printf '%s %s %s\\n' "$1" "$2" "$(date +%s%N)" >>"$EVENT_LOG"; }
log nextest-start cargo
mkdir -p target/nextest/default
cat >target/nextest/default/junit.xml <<'XML'
<testsuites>
  <testsuite name="pictura-core::lib" tests="1">
    <testcase name="works" classname="pictura-core"/>
  </testsuite>
</testsuites>
XML
sleep "${FAKE_NEXTEST_SECONDS:-0}"
log nextest-end cargo
exit 0
"""

FAKE_CTEST = """#!/usr/bin/env bash
set -u
log() { printf '%s %s %s\\n' "$1" "$2" "$(date +%s%N)" >>"$EVENT_LOG"; }
for arg in "$@"; do
    if [ "$arg" = "--show-only=json-v1" ]; then
        python3 - "$PWD" <<'PY'
import json, os, sys
root = sys.argv[1]
suites = os.environ.get("FAKE_SUITES", "tst_one").split()
tests = []
for name in suites:
    tests.append({
        "name": name,
        "command": [f"{root}/build/{name}", "-o",
                    f"{root}/build/qt-test-results/{name}.xml,junitxml"],
    })
print(json.dumps({"kind": "ctestInfo", "version": {"major": 1, "minor": 0},
                  "tests": tests}))
PY
        exit 0
    fi
done
log ctest-start ctest
mkdir -p build/qt-test-results
for name in ${FAKE_SUITES:-tst_one}; do
    [ "$name" = "${FAKE_MISSING:-}" ] && continue
    cat >"build/qt-test-results/$name.xml" <<XML
<testsuite name="$name" tests="1" failures="0">
  <testcase name="runs" classname="$name"/>
</testsuite>
XML
done
sleep "${FAKE_CTEST_SECONDS:-0}"
log ctest-end ctest
exit 0
"""

FAKE_PICTURA = """#!/usr/bin/env bash
set -u
log() { printf '%s %s %s\\n' "$1" "$2" "$(date +%s%N)" >>"$EVENT_LOG"; }
role=bare
for arg in "$@"; do
    case "$arg" in
        *.psd) role=document ;;
    esac
done
log "selftest-$role-start" "pid=$$"
if [ "$role" = document ]; then
    fail=${FAKE_SELFTEST_DOCUMENT_FAIL:-0}
else
    fail=${FAKE_SELFTEST_BARE_FAIL:-0}
fi
sleep "${FAKE_SELFTEST_SECONDS:-0}"
if [ "$fail" != "0" ]; then
    printf 'pictura self-test: FAIL core %s %s boom\\n' "$role" "$fail"
    printf 'pictura self-test: SUMMARY passed=0 failed=1 skipped=0\\n'
    log "selftest-$role-end" "pid=$$"
    exit "$fail"
fi
printf 'pictura self-test: PASS core %s ok\\n' "$role"
printf 'pictura self-test: SUMMARY passed=1 failed=0 skipped=0\\n'
log "selftest-$role-end" "pid=$$"
exit 0
"""


class Sandbox:
    """A throwaway repository wired with the fake tools."""

    def __init__(self) -> None:
        self.tmp = tempfile.TemporaryDirectory(prefix="pictura-qt-exec-")
        self.root = Path(self.tmp.name)
        (self.root / "scripts").mkdir()
        (self.root / "build").mkdir()
        (self.root / "crates/pictura-codec/tests/fixtures").mkdir(parents=True)
        (self.root / "crates/pictura-codec/tests/fixtures/two_layers.psd").write_text("x")
        for name in COPY:
            shutil.copy2(SCRIPTS / name, self.root / "scripts" / name)
        stub = self.root / "scripts" / "build-lock.sh"
        stub.write_text(BUILD_LOCK_STUB)
        stub.chmod(0o644)
        bins = {
            "cmake": FAKE_CMAKE,
            "cargo": FAKE_CARGO,
            "ctest": FAKE_CTEST,
        }
        bin_dir = self.root / "bin"
        bin_dir.mkdir()
        for name, body in bins.items():
            path = bin_dir / name
            path.write_text(body)
            path.chmod(0o755)
        pictura = self.root / "build" / "pictura"
        pictura.write_text(FAKE_PICTURA)
        pictura.chmod(0o755)
        self.events = self.root / "events.log"
        self.state = self.root / "state"
        self.state.mkdir()
        # The executor's temporary capture directory lands here, so a cleanup
        # check can prove it was removed.
        self.tmpdir = self.root / "tmp"
        self.tmpdir.mkdir()

    def close(self) -> None:
        self.tmp.cleanup()

    def env(self, **extra: str) -> dict[str, str]:
        # The CPU-budget variables and any explicit parallel level are set by
        # the harness, never inherited: a host CTest level would make the
        # budget assertion depend on the machine that runs the check.
        env = {
            key: value
            for key, value in os.environ.items()
            if not key.startswith(("PICTURA_", "FAKE_"))
            and key not in {"CTEST_PARALLEL_LEVEL", "CMAKE_BUILD_PARALLEL_LEVEL"}
        }
        env["PATH"] = f"{self.root / 'bin'}{os.pathsep}{env.get('PATH', '')}"
        env["EVENT_LOG"] = str(self.events)
        env["STATE"] = str(self.state)
        env["TMPDIR"] = str(self.tmpdir)
        env["PICTURA_CPU_BUDGET_APPLIED"] = "1"
        env["PICTURA_JOBS"] = "4"
        env.update(extra)
        return env

    def run(
        self, *args: str, timeout: float = 60, **environment: str
    ) -> subprocess.CompletedProcess:
        return subprocess.run(
            ["bash", "scripts/test-report.sh", *args],
            cwd=self.root,
            env=self.env(**environment),
            capture_output=True,
            text=True,
            timeout=timeout,
        )

    def order(self) -> list[tuple[str, str]]:
        if not self.events.is_file():
            return []
        lines = []
        for line in self.events.read_text().splitlines():
            parts = line.split()
            if len(parts) >= 3:
                lines.append((parts[0], parts[2]))
        return lines

    def names(self) -> list[str]:
        return [name for name, _ in self.order()]

    def index(self, name: str) -> int:
        names = self.names()
        assert name in names, f"{name!r} not in {names}"
        return names.index(name)


def _alive(pid: int) -> bool:
    """True while the process exists and is not a zombie.

    A killed child can linger as a zombie until its new parent reaps it, so
    ``kill(pid, 0)`` alone would call a reaped-elsewhere process alive. The
    fallback covers a platform without /proc: there a live zombie is not
    distinguishable, which only makes the assertion stricter in the wrong
    direction and is why this check is Linux-first.
    """
    stat = Path(f"/proc/{pid}/stat")
    if stat.is_file():
        fields = stat.read_text().rsplit(")", 1)
        if len(fields) != 2:
            return True
        return fields[1].split()[0] != "Z"
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    return True


def _assert(condition: bool, message: str, detail: object = "") -> None:
    if not condition:
        raise AssertionError(f"{message}: {detail}")


def single_selftest_capture() -> None:
    box = Sandbox()
    try:
        result = box.run("--qt-only", FAKE_SELFTEST_SECONDS="0.4")
        _assert(result.returncode == 0, "staged run failed", result.stderr)
        names = box.names()
        _assert(names.count("selftest-document-start") == 1, "expected one self-test", names)
        _assert("selftest-bare-start" not in names, "bare self-test ran", names)
        _assert((box.root / "build/qt-test-results/tst_one.xml").is_file(), "no qt report")
        _assert("selftest-psd.log" in result.stdout and "selftest-bare.log" not in result.stdout,
                "expected one capture", result.stdout)
    finally:
        box.close()


def overlap_is_opt_in() -> None:
    box = Sandbox()
    try:
        result = box.run(
            "--configure",
            "--build",
            PICTURA_QT_SELFTEST_OVERLAP="1",
            FAKE_SELFTEST_SECONDS="0.4",
            FAKE_BUILD_SECONDS="0.6",
        )
        _assert(result.returncode == 0, "overlapped run failed", result.stderr)
        _assert(
            box.index("selftest-document-start") < box.index("tail-end"),
            "document self-test did not start before the tail ended",
            box.names(),
        )
        _assert(
            box.index("ctest-start") > max(box.index("tail-end"), box.index("selftest-document-end")),
            "CTest started before the tail and self-tests finished",
            box.names(),
        )
    finally:
        box.close()

    box = Sandbox()
    try:
        result = box.run("--configure", "--build", FAKE_SELFTEST_SECONDS="0.1")
        _assert(result.returncode == 0, "sequential run failed", result.stderr)
        _assert(
            box.index("selftest-document-end")
            < box.index("tail-start"),
            "the tail started before the self-test ended",
            box.names(),
        )
    finally:
        box.close()


def one_ninja_writer() -> None:
    box = Sandbox()
    try:
        result = box.run(
            "--configure",
            "--build",
            PICTURA_QT_SELFTEST_OVERLAP="1",
            FAKE_SELFTEST_SECONDS="0.3",
            FAKE_BUILD_SECONDS="0.3",
        )
        _assert(result.returncode == 0, "overlapped run failed", result.stderr)
        _assert("violation" not in box.names(), "a second build writer appeared", box.names())
    finally:
        box.close()


def pictura_links_before_selftests() -> None:
    box = Sandbox()
    try:
        result = box.run("--configure", "--build")
        _assert(result.returncode == 0, "staged run failed", result.stderr)
        _assert(
            box.index("pictura-end") < box.index("selftest-document-start"),
            "pictura did not link before the document self-test",
            box.names(),
        )
        _assert(
            box.index("pictura-end") < box.index("tail-start"),
            "the tail started before pictura linked",
            box.names(),
        )
    finally:
        box.close()


def ctest_waits_for_build() -> None:
    box = Sandbox()
    try:
        result = box.run(
            "--configure",
            "--build",
            PICTURA_QT_SELFTEST_OVERLAP="1",
            FAKE_BUILD_SECONDS="0.4",
            FAKE_SELFTEST_SECONDS="0.1",
        )
        _assert(result.returncode == 0, "staged run failed", result.stderr)
        last_build = max(box.index("pictura-end"), box.index("tail-end"))
        _assert(box.index("ctest-start") > last_build, "CTest started early", box.names())
        _assert(
            box.index("ctest-start")
            > box.index("selftest-document-end"),
            "CTest started before a self-test finished",
            box.names(),
        )
    finally:
        box.close()


def failure_identity_and_single_capture() -> None:
    box = Sandbox()
    try:
        result = box.run("--qt-only", FAKE_SELFTEST_DOCUMENT_FAIL="9")
        _assert(result.returncode == 9, "self-test status did not win", result.returncode)
        _assert("core::document (exit 9)" in result.stdout,
                "document capture was not reported", result.stdout)
    finally:
        box.close()


def app_build_log_excludes_qt_build_tail() -> None:
    box = Sandbox()
    try:
        capture = box.root / "app build.log"
        result = box.run(
            "--qt-only", "--build", "--app-build-log", str(capture), FAKE_CARGO_JSON="1"
        )
        _assert(result.returncode == 0, "staged run failed", result.stderr)
        app = '{"reason":"build-finished","success":true,"target":"pictura"}'
        tail = '{"reason":"build-finished","success":true,"target":"tail"}'
        _assert(app in result.stdout and tail in result.stdout,
                "both build stages should keep their console output", result.stdout)
        _assert(capture.read_text().splitlines() == [app],
                "audit capture includes output from the wrong stage", capture.read_text())
    finally:
        box.close()


def build_failure_stops_later_stages() -> None:
    box = Sandbox()
    try:
        result = box.run(
            "--configure", "--build", "--app-build-log", str(box.root / "app.log"),
            FAKE_FAIL="pictura",
        )
        _assert(result.returncode == 9, "the pictura build status did not win",
                result.returncode)
        names = box.names()
        _assert("selftest-document-start" not in names, "self-test ran after a failed build", names)
        _assert("ctest-start" not in names, "CTest ran after a failed build", names)
        _assert("no report" in result.stdout, "a missing report was not named", result.stdout)
    finally:
        box.close()

    box = Sandbox()
    try:
        result = box.run("--configure", "--build", FAKE_FAIL="tail")
        _assert(result.returncode == 9, "the tail status did not win", result.returncode)
        names = box.names()
        _assert("selftest-document-end" in names, "self-test did not run before the tail", names)
        _assert("ctest-start" not in names, "CTest ran after a failed tail build", names)
        _assert("no report" in result.stdout, "a missing report was not named", result.stdout)
    finally:
        box.close()

    box = Sandbox()
    try:
        result = box.run("--configure", "--build", FAKE_FAIL="configure")
        _assert(result.returncode == 4, "the configure status did not win", result.returncode)
        names = box.names()
        _assert("pictura-start" not in names, "the build ran after a failed configure", names)
        _assert("ctest-start" not in names, "CTest ran after a failed configure", names)
    finally:
        box.close()


def signal_cleanup_reaps_children() -> None:
    box = Sandbox()
    process = subprocess.Popen(
        ["bash", "scripts/test-report.sh", "--qt-only"],
        cwd=box.root,
        env=box.env(FAKE_SELFTEST_SECONDS="30"),
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    pids: list[int] = []
    try:
        for _ in range(200):
            names = box.names()
            if "selftest-document-start" in names:
                break
            time.sleep(0.05)
        else:
            raise AssertionError(f"self-tests never started: {box.names()}")
        for line in box.events.read_text().splitlines():
            match = re.search(r"^selftest-\w+-start pid=(\d+)", line)
            if match:
                pids.append(int(match.group(1)))
        _assert(len(pids) == 1, "did not capture the self-test pid", pids)
        process.send_signal(signal.SIGTERM)
        status = process.wait(timeout=20)
        _assert(status == 143, "SIGTERM did not exit 143", status)
        _assert(
            "selftest-document-end" not in box.names(),
            "a self-test completed after the signal",
            box.names(),
        )
        _assert("ctest-start" not in box.names(), "CTest ran after a signal", box.names())
        deadline = time.monotonic() + 3
        while time.monotonic() < deadline:
            alive = [pid for pid in pids if _alive(pid)]
            if not alive:
                break
            time.sleep(0.05)
        else:
            raise AssertionError(f"children survived the signal: {alive}")
        deadline = time.monotonic() + 3
        while time.monotonic() < deadline and any(box.tmpdir.iterdir()):
            time.sleep(0.05)
        _assert(
            not any(box.tmpdir.iterdir()),
            "temporary captures survived the signal",
            [str(path) for path in box.tmpdir.iterdir()],
        )
    finally:
        if process.poll() is None:
            process.kill()
            process.wait()
        box.close()


def stale_junit_cleanup() -> None:
    box = Sandbox()
    try:
        results = box.root / "build/qt-test-results"
        results.mkdir()
        stale = results / "tst_removed.xml"
        stale.write_text(
            '<testsuite name="tst_removed" tests="1"><testcase name="x"/></testsuite>'
        )
        result = box.run("--qt-only")
        _assert(result.returncode == 0, "run failed", result.stderr)
        _assert(not stale.exists(), "the stale JUnit file survived")
        _assert("tst_removed" not in result.stdout, "the stale suite reached the report",
                result.stdout)
    finally:
        box.close()


def missing_report_is_named() -> None:
    box = Sandbox()
    try:
        result = box.run("--qt-only", FAKE_SUITES="tst_present tst_gone",
                         FAKE_MISSING="tst_gone")
        _assert(result.returncode != 0, "a missing report did not fail the run",
                result.returncode)
        _assert("tst_gone" in result.stdout, "the missing suite was not named", result.stdout)
        _assert("produced no report" in result.stdout, "the failure was not explained",
                result.stdout)
    finally:
        box.close()


def nextest_is_separate() -> None:
    box = Sandbox()
    try:
        result = box.run("--configure", "--build")
        _assert(result.returncode == 0, "staged run failed", result.stderr)
        _assert(
            box.index("nextest-end") < box.index("configure-start"),
            "nextest did not finish before configure",
            box.names(),
        )
        for later in ("pictura-start", "selftest-document-start", "ctest-start"):
            _assert(
                box.index("nextest-end") < box.index(later),
                f"nextest overlapped {later}",
                box.names(),
            )
    finally:
        box.close()

    box = Sandbox()
    try:
        result = box.run("--qt-only")
        _assert(result.returncode == 0, "qt-only run failed", result.stderr)
        _assert("nextest-start" not in box.names(), "--qt-only still ran nextest")
    finally:
        box.close()


def stage_timings_are_recorded() -> None:
    box = Sandbox()
    try:
        result = box.run("--configure", "--build")
        _assert(result.returncode == 0, "staged run failed", result.stderr)
        for stage in (
            "configure",
            "pictura_build",
            "qt_build_tail",
            "selftest_document",
            "ctest",
            "report",
        ):
            pattern = rf"^timing: {stage}=(\d+\.\d{{3}}s|skipped)$"
            _assert(
                re.search(pattern, result.stdout, re.M) is not None,
                f"no timing for {stage}",
                result.stdout,
            )
    finally:
        box.close()

    box = Sandbox()
    try:
        result = box.run("--qt-only")
        _assert(result.returncode == 0, "qt-only run failed", result.stderr)
        _assert(
            re.search(r"^timing: configure=skipped$", result.stdout, re.M) is not None,
            "a skipped configure was not named",
            result.stdout,
        )
        _assert(
            re.search(r"^timing: pictura_build=skipped$", result.stdout, re.M) is not None,
            "a skipped build was not named",
            result.stdout,
        )
    finally:
        box.close()


def local_adapter_uses_shared_executor() -> None:
    full = (SCRIPTS / "verify-full.sh").read_text(encoding="utf-8")
    fast = (SCRIPTS / "verify-fast.sh").read_text(encoding="utf-8")
    _assert(
        "verify-fast.sh --configure --build" in full,
        "verify-full does not forward configure/build to verify-fast",
        full,
    )
    _assert(
        "cmake --build" not in full and "cmake -S" not in full,
        "verify-full still configures or builds the tree itself",
        full,
    )
    _assert(
        "--configure" in fast and "--build" in fast,
        "verify-fast does not accept the forwarded build options",
        fast,
    )
    _assert(
        'gate_manifest.py run "$placement"' in fast,
        "verify-fast does not use the gate manifest",
        fast,
    )
    manifest = (SCRIPTS / "checks.toml").read_text(encoding="utf-8")
    tests_gate = manifest.split('id = "tests"', 1)[1].split("[[check]]", 1)[0]
    for flag, setting in (
        ("--configure", "PICTURA_VERIFY_QT_CONFIGURE"),
        ("--build", "PICTURA_VERIFY_QT_BUILD"),
    ):
        _assert(
            f"{setting}=1" in fast and setting in tests_gate and f"args+=({flag})" in tests_gate,
            f"the local gate does not forward {flag} to the shared executor",
            fast + tests_gate,
        )
    _assert(
        'bash scripts/test-report.sh "${args[@]}"' in tests_gate,
        "the manifest does not invoke the shared executor",
        tests_gate,
    )
    _assert(
        "check-build-tree.sh" in full and "check-local-build-caching.sh" in full,
        "verify-full dropped the build-tree or cache proof",
        full,
    )


def _job_block(text: str) -> str:
    match = re.search(r"^  qt-headless:\n(.*?)(?=^  [a-z-]+:\n|\Z)", text, re.M | re.S)
    assert match, "qt-headless job not found"
    return match.group(1)


def ci_adapter_uses_shared_executor() -> None:
    text = (ROOT / ".github/workflows/ci.yml").read_text(encoding="utf-8")
    job = _job_block(text)
    invocations = re.findall(r"bash scripts/test-report\.sh[^\n]*", job)
    _assert(len(invocations) == 1, "the job does not invoke the executor exactly once",
            invocations)
    invocation = invocations[0]
    for flag in ("--qt-only", "--configure", "--build"):
        _assert(flag in invocation, f"the CI invocation omits {flag}", invocation)
    for direct in ("cmake --build", "cmake -S", "ctest --test-dir",
                   "--headless --self-test", "report_tests.py"):
        _assert(direct not in job, f"the job still duplicates the executor: {direct}", job)
    _assert(job.count("bash scripts/test-report.sh") == 1,
            "more than one shared step", job)
    tiff = job.find("Provide the TIFF runtime for Qt")
    shared = job.find("bash scripts/test-report.sh")
    _assert(tiff != -1 and shared != -1 and tiff < shared,
            "TIFF setup does not precede the shared executor", job)
    library_path = job.find("export LD_LIBRARY_PATH")
    _assert(0 <= library_path < shared, "the TIFF library path is set too late", job)
    _assert("PICTURA_CPU_BUDGET_PERCENT=100" in job,
            "CI does not request the full runner CPU budget", job)
    # The budget helper sets CTEST_PARALLEL_LEVEL from the budget only when it is
    # unset, so CI's full-CPU request is the default and an explicit level wins.
    box = Sandbox()
    try:
        default_run = box.run("--qt-only")
        _assert(default_run.returncode == 0, "default run failed", default_run.stderr)
        _assert(
            "ctest -j 4" in default_run.stdout,
            "the CTest level did not come from the CPU budget",
            default_run.stdout,
        )
        explicit_run = box.run("--qt-only", CTEST_PARALLEL_LEVEL="3")
        _assert(explicit_run.returncode == 0, "explicit-level run failed",
                explicit_run.stderr)
        _assert(
            "ctest -j 3" in explicit_run.stdout,
            "an explicit CTEST_PARALLEL_LEVEL did not win",
            explicit_run.stdout,
        )
    finally:
        box.close()


CHECKS = {
    name: value
    for name, value in globals().items()
    if name
    in {
        "single_selftest_capture",
        "overlap_is_opt_in",
        "ci_adapter_uses_shared_executor",
        "local_adapter_uses_shared_executor",
        "nextest_is_separate",
        "pictura_links_before_selftests",
        "ctest_waits_for_build",
        "one_ninja_writer",
        "failure_identity_and_single_capture",
        "app_build_log_excludes_qt_build_tail",
        "build_failure_stops_later_stages",
        "signal_cleanup_reaps_children",
        "stale_junit_cleanup",
        "missing_report_is_named",
        "stage_timings_are_recorded",
    }
    and callable(value)
}


def self_check() -> int:
    if sys.flags.optimize:
        print("test_report_execution: self-check requires assertions", file=sys.stderr)
        return 2
    failures = 0
    for name in CHECKS:
        try:
            CHECKS[name]()
        except AssertionError as error:
            failures += 1
            print(f"test_report_execution: FAIL {name}: {error}", file=sys.stderr)
        else:
            print(f"test_report_execution: ok {name}")
    if failures:
        print(f"test_report_execution: FAILED ({failures})", file=sys.stderr)
        return 1
    print("test_report_execution: self-check OK")
    return 0


def main() -> int:
    if "--self-check" in sys.argv[1:] or not sys.argv[1:]:
        return self_check()
    print("usage: test_report_execution.py --self-check", file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main())
