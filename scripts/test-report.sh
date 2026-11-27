#!/usr/bin/env bash
#
# test-report.sh - run every test layer and print the unified report.
#
# This is the single Qt execution path: CI and local verification both use it.
# It owns optional configuration, the staged `pictura`-first build, the Qt Test
# build tail, the self-test capture, stale-result cleanup, CTest selection,
# missing-report validation, reporter invocation, and stage timings. At most one
# `cmake --build` runs against the build tree at a time.
#
# Usage: scripts/test-report.sh [auto|always|never] [options]
#   auto|always|never       reporter colour mode (default auto)
#   --qt-only               skip nextest (the CI Qt job)
#   --configure             configure the CMake tree before building
#   --build                 build `pictura`, then the remaining targets
#   --cmake-arg ARG         extra configure argument (repeatable)
#   --app-build-log PATH    also write the pictura build output to PATH
#
# PICTURA_QT_SELFTEST_OVERLAP=1 lets the self-test overlap the Qt Test build
# tail. It defaults off until #1570 measures it on four vCPUs.
#
# Stage order: nextest (unless --qt-only), configure, stale cleanup, CTest
# registration, `pictura` build, the self-test, Qt Test build tail, CTest,
# report. Exits with the first non-zero stage status in that order; the reporter
# always runs first so test names and missing reports are never lost.

set -euo pipefail

script_dir=$(cd "$(dirname "$0")" && pwd)
# shellcheck source=scripts/build-lock.sh
. "$script_dir/build-lock.sh"
build_lock_enter_managed "$script_dir/agent-build.py" bash "$script_dir/test-report.sh" "$@"
if [ -z "${PICTURA_CPU_BUDGET_APPLIED:-}" ]; then
    exec python3 "$script_dir/cpu-budget.py" exec -- \
        bash "$script_dir/$(basename "$0")" "$@"
fi
: "${PICTURA_JOBS:?cpu-budget marker set without PICTURA_JOBS; run the gate through scripts/cpu-budget.py}"
cd "$script_dir/.."

color=auto
qt_only=0
configure=0
build=0
app_build_log=
cmake_args=()
while [ "$#" -gt 0 ]; do
    case "$1" in
        auto | always | never) color=$1 ;;
        --qt-only) qt_only=1 ;;
        --configure) configure=1 ;;
        --build) build=1 ;;
        --cmake-arg)
            shift
            [ "$#" -gt 0 ] || { echo "test-report.sh: --cmake-arg needs a value" >&2; exit 2; }
            cmake_args+=("$1")
            ;;
        --app-build-log)
            shift
            [ "$#" -gt 0 ] || { echo "test-report.sh: --app-build-log needs a path" >&2; exit 2; }
            app_build_log=$1
            ;;
        -h | --help) sed -n '2,24p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *)
            echo "test-report.sh: unknown argument '$1'" >&2
            exit 2
            ;;
    esac
    shift
done
# A requested build configures first: the spec stages configure before any
# `cmake --build`, and reconfiguring an existing tree keeps its generator.
[ "$build" -eq 0 ] || configure=1

tmp="$(mktemp -d)"
stage_pids=()

_now_us() {
    local stamp="${EPOCHREALTIME:-}"
    if [ -n "$stamp" ]; then
        stamp="${stamp/[.,]/}"
        printf '%s' "${stamp%%[!0-9]*}"
    else
        printf '%s000000' "$(date +%s)"
    fi
}

print_timing() { # <name> <start-us|skipped>
    if [ "${2:-skipped}" = "skipped" ]; then
        printf 'timing: %s=skipped\n' "$1"
        return 0
    fi
    local elapsed_ms=$(( ($(_now_us) - $2) / 1000 ))
    printf 'timing: %s=%d.%03ds\n' "$1" "$((elapsed_ms / 1000))" "$((elapsed_ms % 1000))"
}

print_ms_timing() { # <name> <ms>
    printf 'timing: %s=%s\n' "$1" \
        "$(awk -v ms="${2:-0}" 'BEGIN{printf "%.3fs", ms/1000}')"
}

signal_tree() { # <pid> <signal>
    local child
    for child in $(ps -eo pid=,ppid= 2>/dev/null | awk -v parent="$1" '$2 == parent {print $1}'); do
        signal_tree "$child" "$2"
    done
    kill -"$2" "$1" 2>/dev/null || true
}

stop_children() {
    local pid
    for pid in ${stage_pids[@]+"${stage_pids[@]}"}; do
        signal_tree "$pid" TERM
    done
    sleep 0.2
    for pid in ${stage_pids[@]+"${stage_pids[@]}"}; do
        signal_tree "$pid" KILL
        wait "$pid" 2>/dev/null || true
    done
    stage_pids=()
}

cleanup() {
    stop_children
    rm -rf "$tmp"
}

on_signal() { # <exit-code>
    trap - EXIT INT TERM
    stop_children
    rm -rf "$tmp"
    exit "$1"
}

trap cleanup EXIT
trap 'on_signal 130' INT
trap 'on_signal 143' TERM

# Start one stage in the background so a signal can reach it. The child records
# its own status and wall time; the parent reaps it explicitly at the staged
# point, so a failed stage cannot leave a writer or test running behind the
# reporter.
start_stage() { # <label> <capture|-> <command...>
    local label=$1 capture=$2
    shift 2
    (
        set +e
        started=$(_now_us)
        if [ "$capture" = "-" ]; then
            "$@"
        else
            "$@" >"$capture" 2>&1
        fi
        rc=$?
        finished=$(_now_us)
        printf '%s' "$rc" >"$tmp/$label.rc"
        printf '%s' "$(( (finished - started) / 1000 ))" >"$tmp/$label.ms"
        exit 0
    ) &
    STAGE_PID=$!
    stage_pids+=("$STAGE_PID")
}

drop_pid() { # <pid>
    local entry keep=()
    for entry in ${stage_pids[@]+"${stage_pids[@]}"}; do
        [ "$entry" = "$1" ] || keep+=("$entry")
    done
    stage_pids=("${keep[@]+"${keep[@]}"}")
}

reap_stage() { # <pid|empty> <label>; sets STAGE_RC
    local pid=$1 label=$2
    STAGE_RC=0
    if [ -z "$pid" ]; then
        print_timing "$label" skipped
        return 0
    fi
    wait "$pid" 2>/dev/null || true
    drop_pid "$pid"
    STAGE_RC=$(cat "$tmp/$label.rc" 2>/dev/null || printf 1)
    print_ms_timing "$label" "$(cat "$tmp/$label.ms" 2>/dev/null || printf 0)"
}

junit="target/nextest/default/junit.xml"

nextest_rc=0
configure_rc=0
pictura_rc=0
selftest_document_rc=0
tail_rc=0
ctest_rc=0
report_rc=0

if [ "$qt_only" -eq 1 ]; then
    echo "== nextest == (skipped: --qt-only)"
    print_timing nextest skipped
else
    echo "== nextest =="
    start_stage nextest - cargo nextest run --workspace --no-fail-fast
    reap_stage "$STAGE_PID" nextest
    nextest_rc=$STAGE_RC
fi

if [ "$configure" -eq 1 ]; then
    echo "== configure =="
    # -U drops linker flags cached by an older configure so the mold/lld probe
    # is not masked by a stale value. An existing build keeps its generator; a
    # new one defaults to Ninja.
    configure_args=(-S . -B build -U CMAKE_EXE_LINKER_FLAGS -U CMAKE_SHARED_LINKER_FLAGS)
    if [ ! -f build/CMakeCache.txt ]; then
        configure_args+=(-G Ninja)
    fi
    configure_args+=("${cmake_args[@]+"${cmake_args[@]}"}")
    start_stage configure - cmake "${configure_args[@]}"
    reap_stage "$STAGE_PID" configure
    configure_rc=$STAGE_RC
else
    echo "== configure == (skipped: --configure not requested)"
    print_timing configure skipped
fi

# Old Qt JUnit files must not survive into a new run: a suite that no longer
# produces a report would otherwise look present.
rm -f build/qt-test-results/*.xml build/ctest-junit.xml build/qt-test-timings.json

# Capture the registration after configuration and before any build or test, so
# a suite with no current report is named even when its build fails.
ctest_json="$tmp/ctest.json"
ctest_registered=0
if ctest --test-dir build --show-only=json-v1 >"$ctest_json" 2>/dev/null; then
    ctest_registered=1
    if [ -n "${PICTURA_CTEST_REGISTRATION_FILE:-}" ]; then
        cp "$ctest_json" "$PICTURA_CTEST_REGISTRATION_FILE"
    fi
fi

build_ready=1
[ "$configure_rc" -eq 0 ] || build_ready=0

build_pictura() {
    local -a command=(cmake --build build --target pictura
        --parallel "${CMAKE_BUILD_PARALLEL_LEVEL:-$PICTURA_JOBS}")
    if [ -n "$app_build_log" ]; then
        "${command[@]}" 2>&1 | tee -- "$app_build_log"
    else
        "${command[@]}"
    fi
}

if [ "$build" -eq 1 ] && [ "$build_ready" -eq 1 ]; then
    echo "== pictura build =="
    start_stage pictura_build - build_pictura
    reap_stage "$STAGE_PID" pictura_build
    pictura_rc=$STAGE_RC
elif [ "$build" -eq 1 ]; then
    echo "== pictura build == (skipped: configure failed)"
    print_timing pictura_build skipped
else
    echo "== pictura build == (skipped: --build not requested)"
    print_timing pictura_build skipped
fi
[ "$pictura_rc" -eq 0 ] || build_ready=0

selftest_args=()
document_pid=""
if [ "$build_ready" -eq 1 ] && [ -x ./build/pictura ]; then
    echo "== app self-test =="
    start_stage selftest_document "$tmp/selftest-psd.log" \
        ./build/pictura --headless --self-test \
        crates/pictura-codec/tests/fixtures/two_layers.psd
    document_pid=$STAGE_PID
    selftest_args=(--selftest "$tmp/selftest-psd.log")
elif [ "$build_ready" -ne 1 ]; then
    echo "== app self-test == (skipped: build failed)"
elif [ "$build" -eq 1 ]; then
    echo "== app self-test == (skipped: ./build/pictura missing after the build)"
else
    echo "== app self-test == (skipped: ./build/pictura not built)"
fi

wait_selftests() {
    reap_stage "$document_pid" selftest_document
    selftest_document_rc=$STAGE_RC
    document_pid=""
}

ctest_allowed=$build_ready
tail_pid=""
if [ "$build" -eq 1 ] && [ "$ctest_allowed" -eq 1 ] &&
    [ "${PICTURA_QT_SELFTEST_OVERLAP:-0}" = "1" ]; then
    echo "== qt build tail == (overlapped with the self-test)"
    # The overlapped tail is the only background build writer; the `pictura`
    # build has already been reaped, so there is never a second writer.
    start_stage qt_build_tail - \
        cmake --build build --parallel "${CMAKE_BUILD_PARALLEL_LEVEL:-$PICTURA_JOBS}"
    tail_pid=$STAGE_PID
    wait_selftests
    reap_stage "$tail_pid" qt_build_tail
    tail_rc=$STAGE_RC
else
    wait_selftests
    if [ "$build" -eq 1 ] && [ "$ctest_allowed" -eq 1 ]; then
        echo "== qt build tail =="
        start_stage qt_build_tail - \
            cmake --build build --parallel "${CMAKE_BUILD_PARALLEL_LEVEL:-$PICTURA_JOBS}"
        reap_stage "$STAGE_PID" qt_build_tail
        tail_rc=$STAGE_RC
    else
        print_timing qt_build_tail skipped
    fi
fi
[ "$tail_rc" -eq 0 ] || ctest_allowed=0

qt_args=()
if [ "$ctest_allowed" -eq 1 ] && [ "$ctest_registered" -eq 1 ]; then
    # The budget helper exports the 80% slice and CTEST_PARALLEL_LEVEL; an
    # explicit level wins.
    qt_parallel="${CTEST_PARALLEL_LEVEL:-$PICTURA_JOBS}"
    echo "== qt tests (ctest -j ${qt_parallel}) =="
    start_stage ctest - \
        ctest --test-dir build -R '^tst_' --output-on-failure -j "$qt_parallel" \
            --output-junit build/ctest-junit.xml
    reap_stage "$STAGE_PID" ctest
    ctest_rc=$STAGE_RC
    qt_junit=()
    for xml in build/qt-test-results/*.xml; do
        [ -f "$xml" ] || continue
        qt_args+=(--qt-junit "$xml")
        qt_junit+=("$xml")
    done
    # A registered suite that wrote no report never started: its executable was
    # missing or it died before writing one. The reporter fails the run for it.
    qt_args+=(--ctest-json "$ctest_json")
    if [ "${#qt_junit[@]}" -gt 0 ]; then
        ctest_junit_args=()
        if [ -f build/ctest-junit.xml ]; then
            ctest_junit_args=(--ctest-junit build/ctest-junit.xml)
        fi
        if python3 scripts/qt_test_timings.py collect \
            --junit "${qt_junit[@]}" "${ctest_junit_args[@]}" \
            --out build/qt-test-timings.json; then
            python3 scripts/qt_test_timings.py report --timings build/qt-test-timings.json
        else
            echo "test-report: timing collection failed (not fatal)" >&2
        fi
    fi
else
    if [ "$ctest_allowed" -ne 1 ]; then
        echo "== qt tests == (skipped: build failed)"
    elif [ "$ctest_registered" -ne 1 ]; then
        echo "== qt tests == (skipped: no CTest registration under ./build)"
    fi
    print_timing ctest skipped
    # When configuration succeeded, the registration is still valid input: a
    # build failure must leave the missing Qt reports visible, not silent.
    if [ "$configure_rc" -eq 0 ] && [ "$ctest_registered" -eq 1 ]; then
        qt_args+=(--ctest-json "$ctest_json")
    fi
fi

report_args=()
[ "$qt_only" -eq 1 ] || report_args+=(--junit "$junit")
report_args+=("${selftest_args[@]+"${selftest_args[@]}"}")
report_args+=("${qt_args[@]+"${qt_args[@]}"}")

echo "== report =="
started=$(_now_us)
set +e
python3 scripts/report_tests.py \
    "${report_args[@]+"${report_args[@]}"}" \
    --color "$color"
report_rc=$?
set -e
print_timing report "$started"

status=0
for rc in "$nextest_rc" "$configure_rc" "$pictura_rc" \
    "$selftest_document_rc" "$tail_rc" \
    "$ctest_rc" "$report_rc"; do
    if [ "$rc" -ne 0 ]; then
        status=$rc
        break
    fi
done
exit "$status"
