#!/usr/bin/env bash
#
# verify-full.sh - full verification: build the CMake/Qt app first, then
# verify-fast.sh, whose test step routes through scripts/test-report.sh and
# therefore exercises the app self-tests and prints the unified report.
# Slow; run verify-fast.sh during the edit loop.

set -euo pipefail
cd "$(dirname "$0")/.."

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
cmake --build build --parallel
echo "cmake_build=ok"

echo "== build cache =="
bash scripts/check-local-build-caching.sh --rebuild --dir build

bash scripts/verify-fast.sh

echo "== control =="
bash scripts/verify-control.sh

echo "verify-full: OK"
