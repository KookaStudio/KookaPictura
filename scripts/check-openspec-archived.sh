#!/usr/bin/env bash
#
# check-openspec-archived.sh - fail when openspec/changes/ holds an unarchived change.
#
# The zero-open policy (AGENTS.md): a change is authored, implemented, and
# archived in the pull request that ships it, so only archive/ may remain under
# openspec/changes/. The guards workflow and scripts/verify-fast.sh run this.
#
# Usage: check-openspec-archived.sh [--self-test] [<changes-dir>]
#   --self-test   build a throwaway sandbox and assert the fail and pass cases
#   <changes-dir> directory to scan (default: <repo>/openspec/changes)

set -euo pipefail

repo_root=$(cd "$(dirname "$0")/.." && pwd)

scan() {
    find "$1" -mindepth 1 -maxdepth 1 \( -type d -o -type l \) ! -name archive | sort
}

check() {
    local dir="${1:-$repo_root/openspec/changes}"
    if [ ! -d "$dir" ]; then
        echo "check-openspec-archived: FAILED (no such directory: $dir)" >&2
        return 2
    fi
    local strays
    strays=$(scan "$dir")
    if [ -n "$strays" ]; then
        echo "check-openspec-archived: FAILED (non-archived changes under $dir):"
        echo "$strays"
        return 1
    fi
    echo "check-openspec-archived: OK ($dir)"
}

sandbox=""
cleanup() { [ -n "$sandbox" ] && rm -rf "$sandbox"; }

self_test() {
    local fail_dir pass_dir missing_dir rc=0
    sandbox=$(mktemp -d "${TMPDIR:-/tmp}/pictura-archive-check.XXXXXX") || exit 1
    trap cleanup EXIT
    fail_dir="$sandbox/fail"
    pass_dir="$sandbox/pass"
    missing_dir="$sandbox/missing"
    mkdir -p "$fail_dir/archive" "$fail_dir/some-change" "$pass_dir/archive"
    ln -s "$pass_dir" "$fail_dir/linked-change"

    if check "$fail_dir" >/dev/null 2>&1; then
        echo "check-openspec-archived: self-test FAILED (a stray change was not flagged)" >&2
        rc=1
    fi
    if ! check "$pass_dir" >/dev/null 2>&1; then
        echo "check-openspec-archived: self-test FAILED (an archive-only tree was flagged)" >&2
        rc=1
    fi
    if check "$missing_dir" >/dev/null 2>&1; then
        echo "check-openspec-archived: self-test FAILED (a missing directory was accepted)" >&2
        rc=1
    fi

    [ "$rc" -eq 0 ] && echo "check-openspec-archived: self-test OK"
    return "$rc"
}

case "${1:-}" in
    --self-test) self_test ;;
    --*)
        echo "check-openspec-archived: unknown option: $1" >&2
        exit 2
        ;;
    *) check "${1:-}" ;;
esac
