#!/usr/bin/env bash
#
# verify-full.sh - full verification: build the CMake/Qt app first, then
# verify-fast.sh, whose test step routes through scripts/test-report.sh and
# therefore exercises the app self-tests and prints the unified report.
# Slow; run verify-fast.sh during the edit loop.

set -euo pipefail
cd "$(dirname "$0")/.."

echo "== cmake =="
if [ ! -d build ]; then
  cmake -S . -B build -G Ninja -DCMAKE_EXE_LINKER_FLAGS=-fuse-ld=lld
fi
cmake --build build --parallel
echo "cmake_build=ok"

echo "== build cache =="
bash scripts/check-local-build-caching.sh --rebuild --dir build

bash scripts/verify-fast.sh

echo "== control =="
bash scripts/verify-control.sh

echo "verify-full: OK"
