#!/usr/bin/env bash
#
# check-build-tree.sh - report a build tree that is older than the sources it
# tests.
#
# Two staleness signals: the application binary in the build tree predating a
# tracked source under the application crate, and a CTest-registered suite with
# no executable in the tree or an executable older than its own source. Either
# one means the tests would run against something other than the current
# sources.
#
# Usage: scripts/check-build-tree.sh [--warn] [--build-dir DIR]
#                                      [--source-dir DIR] [--ctest-json FILE]
#          scripts/check-build-tree.sh --self-test
#
# --warn reports the staleness but exits 0: the fast gate reports it, the
# full gate fails on it. Without --warn the script exits non-zero.

set -uo pipefail
cd "$(dirname "$0")/.."

warn_only=0
build_dir="build"
source_dir="crates/pictura-app"
ctest_json=""
self_test=0

while [ "$#" -gt 0 ]; do
    case "$1" in
        --warn) warn_only=1 ;;
        --build-dir) shift; build_dir="${1:-}" ;;
        --source-dir) shift; source_dir="${1:-}" ;;
        --ctest-json) shift; ctest_json="${1:-}" ;;
        --self-test) self_test=1 ;;
        *) echo "check-build-tree: unknown argument '$1'" >&2; exit 2 ;;
    esac
    shift
done

report() { # <message>
    if [ "$warn_only" -eq 1 ]; then
        echo "check-build-tree: warning: $1" >&2
    else
        echo "check-build-tree: $1" >&2
    fi
}

# The registered suites whose executable is missing or older than its source.
# The registration comes from CTest's own JSON, so the list is the one the test
# run uses. Prints "<name>\t<executable>\t<reason>" per problem.
suite_problems() { # <ctest-json path> <test source dir>
    python3 - "$1" "$2" <<'PY'
import json
import os
import sys

try:
    with open(sys.argv[1], "r", encoding="utf-8") as handle:
        data = json.load(handle)
except (OSError, ValueError):
    sys.exit(0)

tests_dir = sys.argv[2]
for test in data.get("tests", []):
    name = test.get("name", "")
    command = test.get("command") or []
    # A Qt Test suite is registered with a "-o <path>,junitxml" argument; the
    # other registered tests report nothing and are not this check's business.
    if "-o" not in command:
        continue
    executable = command[0] if command else ""
    if not executable or not os.path.exists(executable):
        print(f"{name}\t{executable}\tno executable")
        continue
    source = os.path.join(tests_dir, f"{name}.cpp")
    if os.path.exists(source) and os.path.getmtime(source) > os.path.getmtime(executable):
        print(f"{name}\t{executable}\tolder than {source}")
PY
}

check_tree() { # <build-dir> <source-dir> <ctest-json>
    local problems=0
    local binary="$1/pictura"
    local newer=""

    if [ ! -x "$binary" ]; then
        report "$binary is missing"
        problems=1
    else
        # The test suites are built from their own sources; a change there does
        # not rebuild the application binary, so they are checked per suite
        # below instead of against it.
        newer=$(find "$2" -type f \
            \( -name '*.cpp' -o -name '*.h' -o -name '*.rs' \) \
            -not -path "$2/cpp/tests/*" \
            -newer "$binary" -print -quit 2>/dev/null)
        if [ -n "$newer" ]; then
            report "$binary is older than $newer"
            problems=1
        fi
    fi

    if [ -n "$3" ] && [ -f "$3" ]; then
        local problems_out
        problems_out=$(suite_problems "$3" "$2/cpp/tests")
        if [ -n "$problems_out" ]; then
            while IFS=$'\t' read -r name executable reason; do
                [ -n "$name" ] || continue
                report "the registered suite $name is $reason ($executable)"
            done <<< "$problems_out"
            problems=1
        fi
    fi

    return "$problems"
}

if [ "$self_test" -eq 1 ]; then
    sandbox=$(mktemp -d "${TMPDIR:-/tmp}/pictura-build-tree-check.XXXXXX")
    trap 'rm -rf "$sandbox"' EXIT
    mkdir -p "$sandbox/src/cpp/tests" "$sandbox/build"
    printf 'int main() { return 0; }\n' >"$sandbox/src/main.cpp"
    printf 'int suite() { return 0; }\n' >"$sandbox/src/cpp/tests/tst_gone.cpp"
    printf '#!/bin/sh\nexit 0\n' >"$sandbox/build/pictura"
    chmod +x "$sandbox/build/pictura"
    printf '#!/bin/sh\nexit 0\n' >"$sandbox/build/tst_gone"
    chmod +x "$sandbox/build/tst_gone"
    touch -d '2020-01-01' "$sandbox/build/pictura" "$sandbox/build/tst_gone"
    printf '{\n  "tests": [\n    {"name": "tst_gone",\n     "command": ["%s/build/tst_gone", "-o", "/tmp/tst_gone.xml,junitxml"]}\n  ]\n}\n' \
        "$sandbox" >"$sandbox/ctest.json"

    check_tree "$sandbox/build" "$sandbox/src" "$sandbox/ctest.json"
    rc=$?
    [ "$rc" -eq 1 ] || { echo "check-build-tree: self-test: a stale tree was not reported" >&2; exit 1; }

    touch "$sandbox/build/pictura" "$sandbox/build/tst_gone"
    check_tree "$sandbox/build" "$sandbox/src" "$sandbox/ctest.json"
    rc=$?
    [ "$rc" -eq 0 ] || { echo "check-build-tree: self-test: a current tree was reported stale" >&2; exit 1; }

    # A suite whose source is newer than its executable is stale even though
    # the application binary is current.
    touch "$sandbox/src/cpp/tests/tst_gone.cpp"
    check_tree "$sandbox/build" "$sandbox/src" "$sandbox/ctest.json"
    rc=$?
    [ "$rc" -eq 1 ] || { echo "check-build-tree: self-test: a stale suite was not reported" >&2; exit 1; }

    # A registered suite with no executable is stale.
    rm -f "$sandbox/build/tst_gone"
    check_tree "$sandbox/build" "$sandbox/src" "$sandbox/ctest.json"
    rc=$?
    [ "$rc" -eq 1 ] || { echo "check-build-tree: self-test: a missing suite was not reported" >&2; exit 1; }

    echo "check-build-tree: self-test OK"
    exit 0
fi

if [ -z "$ctest_json" ]; then
    ctest_json=$(mktemp "${TMPDIR:-/tmp}/pictura-ctest-list.XXXXXX.json")
    trap 'rm -f "$ctest_json"' EXIT
    ctest --test-dir "$build_dir" --show-only=json-v1 >"$ctest_json" 2>/dev/null || true
fi

check_tree "$build_dir" "$source_dir" "$ctest_json"
problems=$?
if [ "$problems" -ne 0 ] && [ "$warn_only" -eq 1 ]; then
    exit 0
fi
exit "$problems"
