#!/usr/bin/env bash
#
# verify-full.sh - full verification: run verify-fast.sh with the Qt build
# enabled, then the live checks.
#
# The Qt execution (configure, staged build, self-tests, CTest, report) lives in
# scripts/test-report.sh, which verify-fast.sh reaches with --configure --build.
# This script owns the lock, the build-tree validation, the cache proof, and the
# live control and memory checks.

set -euo pipefail

script_dir=$(cd "$(dirname "$0")" && pwd)
# shellcheck source=scripts/build-lock.sh
. "$script_dir/build-lock.sh"
build_lock_enter_managed "$script_dir/agent-build.py" bash "$script_dir/verify-full.sh" "$@"
if [ -z "${PICTURA_CPU_BUDGET_APPLIED:-}" ]; then
    exec python3 "$script_dir/cpu-budget.py" exec -- \
        bash "$script_dir/$(basename "$0")" "$@"
fi
: "${PICTURA_JOBS:?cpu-budget marker set without PICTURA_JOBS; run the gate through scripts/cpu-budget.py}"
cd "$script_dir/.."

build_lock_install_traps
build_lock_acquire || exit 1
if [ "${_build_lock_owned:-0}" = "1" ]; then
    export PICTURA_VERIFY_LOCK_HELD=1
fi

export PICTURA_FULL_GATES=1
bash scripts/verify-fast.sh --configure --build

echo "== build tree =="
# The build just ran, so the tree has to match the sources now. A tree that does
# not would make the live checks below meaningless.
bash scripts/check-build-tree.sh

echo "== build cache =="
bash scripts/check-local-build-caching.sh --rebuild --dir build

echo "== control =="
bash scripts/verify-control.sh

echo "== memory harness =="
# The memory benchmark's own checks (#1210): the fixture generator, then the
# profiler's helpers and every M scenario at 512x512 against the build above.
python3 scripts/gen-large-fixtures.py --self-check
python3 scripts/profile-tools.py --self-check --bin ./build/pictura

echo "verify-full: OK"
