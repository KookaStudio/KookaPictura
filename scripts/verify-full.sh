#!/usr/bin/env bash
#
# verify-full.sh - full verification: build the CMake/Qt app first, then
# verify-fast.sh, whose test step routes through scripts/test-report.sh and
# therefore exercises the app self-tests and prints the unified report.
# Slow; run verify-fast.sh during the edit loop.

set -euo pipefail

script_dir=$(cd "$(dirname "$0")" && pwd)
if [ -z "${PICTURA_CPU_BUDGET_APPLIED:-}" ]; then
    exec python3 "$script_dir/cpu-budget.py" exec -- \
        bash "$script_dir/$(basename "$0")" "$@"
fi
: "${PICTURA_JOBS:?cpu-budget marker set without PICTURA_JOBS; run the gate through scripts/cpu-budget.py}"
cd "$script_dir/.."

# shellcheck source=scripts/build-lock.sh
. scripts/build-lock.sh

build_lock_install_traps
build_lock_acquire || exit 1
if [ "${_build_lock_owned:-0}" = "1" ]; then
    export PICTURA_VERIFY_LOCK_HELD=1
fi

echo "== cmake =="
# -U drops linker flags cached by an older configure so the mold/lld probe is
# not masked by a stale -DCMAKE_EXE_LINKER_FLAGS value. An existing build keeps
# its generator; a new one defaults to Ninja.
if [ -f build/CMakeCache.txt ]; then
  cmake -S . -B build -U CMAKE_EXE_LINKER_FLAGS -U CMAKE_SHARED_LINKER_FLAGS
else
  cmake -S . -B build -G Ninja -U CMAKE_EXE_LINKER_FLAGS -U CMAKE_SHARED_LINKER_FLAGS
fi
cmake --build build --parallel "${CMAKE_BUILD_PARALLEL_LEVEL:-$PICTURA_JOBS}"
echo "cmake_build=ok"

echo "== build tree =="
# The build just ran, so the tree has to match the sources now. A tree that
# does not would make the self-test and the Qt suites below meaningless.
bash scripts/check-build-tree.sh

echo "== build cache =="
bash scripts/check-local-build-caching.sh --rebuild --dir build

bash scripts/verify-fast.sh

echo "== control =="
bash scripts/verify-control.sh

echo "== memory harness =="
# The memory benchmark's own checks (#1210): the fixture generator, then the
# profiler's helpers and every M scenario at 512x512 against the build above.
python3 scripts/gen-large-fixtures.py --self-check
python3 scripts/profile-tools.py --self-check --bin ./build/pictura

echo "verify-full: OK"
