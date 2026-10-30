#!/usr/bin/env bash
#
# test-report.sh - run every test layer and print the unified report.
#
# Runs nextest (which writes target/nextest/default/junit.xml) and both app
# self-test invocations, then hands the captured output to
# report_tests.py. Exits non-zero when the reporter reports a failure, so a
# failing layer fails this script.
#
# Usage: scripts/test-report.sh [auto|always|never]   (colour mode, default auto)

set -euo pipefail
cd "$(dirname "$0")/.."

color="${1:-auto}"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

junit="target/nextest/default/junit.xml"

echo "== nextest =="
set +e
cargo nextest run --workspace --no-fail-fast 2>&1 | tee "$tmp/nextest.log"
nextest_rc=${PIPESTATUS[0]}
set -e

# The two self-test invocations and the Qt Test layer are independent, so they
# run at the same time instead of one after another. Each self-test invocation
# isolates its own state home (see crates/pictura-app/cpp/main.cpp), so the two
# do not share state.
selftest_pids=()
selftest_args=()
if [ -x ./build/pictura ]; then
  echo "== app self-test =="
  bare="$tmp/selftest-bare.log"
  psd="$tmp/selftest-psd.log"
  ./build/pictura --headless --self-test >"$bare" 2>&1 &
  selftest_pids+=("$!")
  ./build/pictura --headless --self-test \
    crates/pictura-codec/tests/fixtures/two_layers.psd >"$psd" 2>&1 &
  selftest_pids+=("$!")
  selftest_args=(--selftest "$bare" --selftest "$psd")
else
  echo "== app self-test == (skipped: ./build/pictura not built)"
fi

# Qt Test suites are independent of the pictura executable: they are their own
# CTest-registered binaries. Run them via CTest so CMake's per-executable JUnit
# (build/qt-test-results/*.xml) is what the reporter consumes. The registration
# list decides whether the layer runs at all: a build tree whose suite
# executables are missing has to be reported as a failure, not skipped.
qt_args=()
qt_rc=0
qt_parallel="${CTEST_PARALLEL_LEVEL:-4}"
ctest_json="$tmp/ctest.json"
if ctest --test-dir build --show-only=json-v1 >"$ctest_json" 2>/dev/null; then
  echo "== qt tests (ctest -j ${qt_parallel}) =="
  rm -f build/qt-test-results/*.xml
  set +e
  ctest --test-dir build -R '^tst_' --output-on-failure -j "$qt_parallel"
  qt_rc=$?
  set -e
  for xml in build/qt-test-results/*.xml; do
    [ -f "$xml" ] || continue
    qt_args+=(--qt-junit "$xml")
  done
  # A registered suite that wrote no report never started: its executable was
  # missing or it died before writing one. The reporter fails the run for it.
  qt_args+=(--ctest-json "$ctest_json")
else
  echo "== qt tests == (skipped: no CTest registration under ./build)"
fi

selftest_rc=0
for pid in ${selftest_pids[@]+"${selftest_pids[@]}"}; do
  wait "$pid" || selftest_rc=1
done

echo "== report =="
set +e
python3 scripts/report_tests.py \
  --junit "$junit" \
  "${selftest_args[@]}" \
  "${qt_args[@]}" \
  --color "$color"
report_rc=$?
set -e

if [ "$nextest_rc" -ne 0 ] || [ "$qt_rc" -ne 0 ] || [ "$selftest_rc" -ne 0 ]; then
  exit 1
fi
exit "$report_rc"
