#!/usr/bin/env bash
#
# with-build-lock.sh - apply the CPU budget and run a direct heavy command
# under the opt-in build lock.
#
# Usage: scripts/with-build-lock.sh -- <command...>
#
# The first pass re-execs once through scripts/cpu-budget.py, exactly like the
# gates, so the command and its children run in the shared affinity slice with
# the budget's job counts. The lock is then enabled by PICTURA_VERIFY_LOCK=1
# (process environment or the user-global env file), exactly as
# scripts/verify-fast.sh reads it. The wrapper runs the command as a child so
# the EXIT trap can release the lock; it does not change the caller's working
# directory.

set -euo pipefail

script_dir=$(cd "$(dirname "$0")" && pwd)
# shellcheck source=scripts/build-lock.sh
. "$script_dir/build-lock.sh"
build_lock_enter_managed "$script_dir/agent-build.py" bash "$script_dir/with-build-lock.sh" "$@"
if [ -z "${PICTURA_CPU_BUDGET_APPLIED:-}" ]; then
    exec python3 "$script_dir/cpu-budget.py" exec -- \
        bash "$script_dir/with-build-lock.sh" "$@"
fi
: "${PICTURA_JOBS:?cpu-budget marker set without PICTURA_JOBS; run the wrapper directly or through scripts/cpu-budget.py}"

if [ "${1:-}" = "--" ]; then
    shift
fi
if [ "$#" -eq 0 ]; then
    printf 'with-build-lock: usage: %s -- <command...>\n' "${0##*/}" >&2
    exit 2
fi

build_lock_install_traps
build_lock_acquire || exit 1
if [ "${_build_lock_owned:-0}" = "1" ]; then
    export PICTURA_VERIFY_LOCK_HELD=1
fi

set +e
"$@" <&0 &
child=$!
wait "$child"
status=$?
set -e
build_lock_release
exit "$status"
