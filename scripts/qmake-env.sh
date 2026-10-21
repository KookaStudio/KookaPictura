#!/usr/bin/env bash
#
# qmake-env.sh - export QMAKE to the value the CMake build selected.
#
# qt-build-utils emits `cargo::rerun-if-env-changed=QMAKE`, and CMake exports
# QMAKE from cxx_qt_import_crate. A direct cargo run without it reruns the
# cxx-qt build script and recompiles pictura_app. Source this after cd'ing to the
# repository root from any script that runs cargo after a CMake build.

if [ -z "${QMAKE:-}" ]; then
    qmake_value=""
    if [ -f build/CMakeCache.txt ]; then
        qmake_value=$(sed -n 's/^QMAKE_EXECUTABLE:[^=]*=//p' build/CMakeCache.txt | head -n1)
    fi
    if [ -z "$qmake_value" ]; then
        qmake_value=$(command -v qmake6 || command -v qmake || true)
    fi
    if [ -n "$qmake_value" ]; then
        export QMAKE="$qmake_value"
    fi
fi
