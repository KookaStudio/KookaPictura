#!/usr/bin/env bash
#
# check-build-lock.sh - prove the opt-in build lock without touching the real
# lock, the real user-global env file, or running a verification build.
#
# The lock helper is tested directly; verify-fast.sh/verify-full.sh are
# inspected statically for placement and marker wiring. Sandboxed
# HOME/XDG/TMPDIR/KOOKA_ENV_FILE keep it hermetic. Exits non-zero on any failed
# assertion.

set -euo pipefail
cd "$(dirname "$0")/.."

helper="$PWD/scripts/build-lock.sh"
if [ ! -f "$helper" ]; then
    echo "check-build-lock: missing $helper" >&2
    exit 1
fi

sandbox=$(mktemp -d "${TMPDIR:-/tmp}/pictura-build-lock-check.XXXXXX")
cleanup() {
    local p
    for p in $(jobs -p 2>/dev/null); do
        kill -TERM "$p" 2>/dev/null || true
    done
    rm -rf "$sandbox"
}
trap cleanup EXIT

export HOME="$sandbox/home"
export XDG_CONFIG_HOME="$HOME/.config"
export XDG_RUNTIME_DIR="$sandbox/run"
export TMPDIR="$sandbox/tmp"
export KOOKA_ENV_FILE="$sandbox/env"
unset PICTURA_VERIFY_LOCK PICTURA_VERIFY_LOCK_TIMEOUT PICTURA_VERIFY_LOCK_HELD
export PICTURA_VERIFY_LOCK_TIMEOUT=10
mkdir -p "$HOME" "$XDG_CONFIG_HOME" "$XDG_RUNTIME_DIR" "$TMPDIR"

. "$helper"
lockdir=$(build_lock_dir_path)

fail() { echo "check-build-lock: FAIL: $*" >&2; exit 1; }
assert_absent() { [ ! -e "$lockdir" ] && [ ! -L "$lockdir" ] || fail "lock directory left behind: $lockdir"; }
assert_present() { [ -d "$lockdir" ] || fail "expected lock directory: $lockdir"; }
assert_mode_700() {
    local mode
    mode=$(stat -c %a "$lockdir" 2>/dev/null || stat -f %Lp "$lockdir" 2>/dev/null || true)
    [ "$mode" = "700" ] || fail "lock directory mode '$mode', want 700"
}
assert_owner_fields() {
    local field
    for field in pid worktree branch script since; do
        grep -q "^$field=" "$lockdir/owner" 2>/dev/null || fail "owner record lacks $field"
    done
}

wait_for_lock() {
    local i
    for i in {1..100}; do
        [ -d "$lockdir" ] && return 0
        sleep 0.05
    done
    return 1
}

wait_for_owner() {
    local i
    for i in {1..100}; do
        grep -q '^worktree=' "$lockdir/owner" 2>/dev/null && return 0
        sleep 0.02
    done
    return 1
}

hold_async() { # <seconds>; sets HOLDER_PID
    (
        . "$helper"
        build_lock_install_traps
        build_lock_acquire || exit 1
        sleep "$1" &
        wait $!
    ) &
    HOLDER_PID=$!
}

child_of() { # <pid>
    local out
    out=$(ps -eo pid=,ppid= 2>/dev/null | awk -v p="$1" '$2==p {print $1; exit}')
    if [ -z "$out" ]; then
        out=$(ps -axo pid=,ppid= 2>/dev/null | awk -v p="$1" '$2==p {print $1; exit}')
    fi
    printf '%s' "$out"
}

# bl_off: unset and 0 both leave the lock disabled.
unset PICTURA_VERIFY_LOCK
build_lock_enabled && fail "enabled with no opt-in"
build_lock_acquire || fail "disabled acquire returned failure"
assert_absent
build_lock_release
export PICTURA_VERIFY_LOCK=0
build_lock_enabled && fail "enabled with PICTURA_VERIFY_LOCK=0"
build_lock_acquire || fail "disabled acquire returned failure"
assert_absent
unset PICTURA_VERIFY_LOCK

# bl_envfile: the user-global env file enables the lock.
printf 'PICTURA_VERIFY_LOCK=1\n' >"$KOOKA_ENV_FILE"
build_lock_enabled || fail "env file did not enable the lock"
build_lock_acquire || fail "env-file acquire failed"
assert_present
assert_mode_700
wait_for_owner || fail "owner record was not published"
assert_owner_fields
build_lock_release
assert_absent
rm -f "$KOOKA_ENV_FILE"

# bl_envfile_forms: CRLF, quotes, trailing spaces, and inline comments parse.
for value in '"1"' "'1'" '1 ' '1 # opt in'; do
    printf 'PICTURA_VERIFY_LOCK=%s\r\n' "$value" >"$KOOKA_ENV_FILE"
    unset PICTURA_VERIFY_LOCK
    build_lock_enabled || fail "env-file value '$value' did not enable the lock"
done
rm -f "$KOOKA_ENV_FILE"

# bl_envfile_default: the default user-global path is read.
unset KOOKA_ENV_FILE PICTURA_VERIFY_LOCK
mkdir -p "$XDG_CONFIG_HOME/kooka-pictura"
printf 'PICTURA_VERIFY_LOCK=1\n' >"$XDG_CONFIG_HOME/kooka-pictura/env"
build_lock_enabled || fail "default env path did not enable the lock"
build_lock_acquire || fail "default-env-path acquire failed"
build_lock_release
assert_absent
rm -f "$XDG_CONFIG_HOME/kooka-pictura/env"
export KOOKA_ENV_FILE="$sandbox/env"

# bl_precedence: a process value wins even when it is 0 (both variables).
printf 'PICTURA_VERIFY_LOCK=1\nPICTURA_VERIFY_LOCK_TIMEOUT=1\n' >"$KOOKA_ENV_FILE"
export PICTURA_VERIFY_LOCK=0 PICTURA_VERIFY_LOCK_TIMEOUT=2
build_lock_enabled && fail "process PICTURA_VERIFY_LOCK=0 did not override the file"
[ "$(build_lock_setting PICTURA_VERIFY_LOCK_TIMEOUT)" = "2" ] \
    || fail "process timeout did not override the file"
build_lock_acquire || fail "disabled acquire returned failure"
assert_absent
unset PICTURA_VERIFY_LOCK
export PICTURA_VERIFY_LOCK_TIMEOUT=10
rm -f "$KOOKA_ENV_FILE"

# bl_notracked: a repository-local .env cannot enable the lock.
mkdir -p "$sandbox/repo"
printf 'PICTURA_VERIFY_LOCK=1\n' >"$sandbox/repo/.env"
unset KOOKA_ENV_FILE PICTURA_VERIFY_LOCK
(
    cd "$sandbox/repo"
    . "$helper"
    build_lock_enabled && exit 1
    build_lock_acquire || exit 1
    [ ! -e "$(build_lock_dir_path)" ] || exit 1
) || fail "a repo-local .env enabled the lock"
export KOOKA_ENV_FILE="$sandbox/env"

# bl_queue: a waiter reports the holder and proceeds after release.
export PICTURA_VERIFY_LOCK=1
hold_async 2
holder=$HOLDER_PID
wait_for_lock || fail "holder did not acquire the lock"
wait_for_owner || fail "holder owner record was not published"
assert_mode_700
assert_owner_fields
queue_log="$sandbox/queue.log"
queue_start=$(date +%s)
( . "$helper"; build_lock_acquire || exit 1; build_lock_release ) 2>"$queue_log" \
    || fail "queued acquire failed"
queue_elapsed=$(( $(date +%s) - queue_start ))
[ "$queue_elapsed" -ge 1 ] || fail "waiter did not wait for the holder"
grep -q '^  worktree=' "$queue_log" || fail "waiting message does not name the holder worktree"
wait "$holder" || fail "holder failed"
assert_absent
unset PICTURA_VERIFY_LOCK

# bl_nested: the held marker skips acquisition and creates no lock.
export PICTURA_VERIFY_LOCK=1 PICTURA_VERIFY_LOCK_HELD=1
build_lock_acquire || fail "nested acquire returned failure"
assert_absent
build_lock_release
unset PICTURA_VERIFY_LOCK_HELD

# bl_nested_release: the nested script must not release the parent's lock.
build_lock_acquire || fail "parent acquire failed"
assert_present
(
    export PICTURA_VERIFY_LOCK_HELD=1
    . "$helper"
    build_lock_acquire || exit 1
    build_lock_release
) || fail "nested invocation failed"
assert_present
build_lock_release
assert_absent
unset PICTURA_VERIFY_LOCK

# bl_release_ownership: release must not delete a successor's lock.
export PICTURA_VERIFY_LOCK=1
build_lock_acquire || fail "ownership acquire failed"
sleep 10 &
successor=$!
printf 'pid=%s\nworktree=/successor\nbranch=other\nscript=other\nsince=now\n' "$successor" >"$lockdir/owner"
build_lock_release
assert_present
rm -rf -- "$lockdir"
kill "$successor" 2>/dev/null || true
wait "$successor" 2>/dev/null || true
unset PICTURA_VERIFY_LOCK

# bl_symlink: a symlink at the lock path is refused, not followed.
export PICTURA_VERIFY_LOCK=1
ln -s "$sandbox/not-a-lock" "$lockdir"
set +e
( . "$helper"; build_lock_acquire ) 2>"$sandbox/symlink.log"
symlink_rc=$?
set -e
[ "$symlink_rc" -ne 0 ] || fail "symlink lock path was accepted"
grep -q 'not a directory or a symlink' "$sandbox/symlink.log" || fail "symlink rejection message missing"
rm -f "$lockdir"
assert_absent

# bl_docs_only: verify-fast acquires only after the docs-only exit, and
# verify-full exports the nesting marker only when it owns the lock.
fast=scripts/verify-fast.sh
full=scripts/verify-full.sh
docs_msg=$(grep -n 'verify-fast: OK (docs-only)' "$fast" | cut -d: -f1 | head -1)
docs_exit=$(awk -v start="$docs_msg" 'NR > start && $0 ~ /^[[:space:]]*exit 0[[:space:]]*$/ {print NR; exit}' "$fast")
acquire_line=$(grep -n 'build_lock_acquire' "$fast" | cut -d: -f1 | head -1)
[ -n "$docs_msg" ] || fail "docs-only marker not found in $fast"
[ -n "$docs_exit" ] || fail "docs-only exit not found in $fast"
[ -n "$acquire_line" ] || fail "build_lock_acquire not found in $fast"
[ "$acquire_line" -gt "$docs_exit" ] || fail "acquire precedes the docs-only exit in $fast"
grep -q 'build_lock_install_traps' "$fast" || fail "verify-fast does not install the lock traps"
grep -q 'build_lock_install_traps' "$full" || fail "verify-full does not install the lock traps"
grep -q 'PICTURA_VERIFY_LOCK_HELD=1' "$full" || fail "verify-full does not export the nesting marker"
grep -q '_build_lock_owned' "$full" || fail "verify-full does not gate the marker on ownership"

# bl_timeout: a bounded wait exits non-zero, names the holder, and leaves no lock.
export PICTURA_VERIFY_LOCK=1 PICTURA_VERIFY_LOCK_TIMEOUT=1
hold_async 2
holder=$HOLDER_PID
wait_for_lock || fail "timeout holder did not acquire the lock"
timeout_log="$sandbox/timeout.log"
set +e
( . "$helper"; build_lock_acquire ) 2>"$timeout_log"
timeout_rc=$?
set -e
[ "$timeout_rc" -ne 0 ] || fail "waiter did not time out"
grep -q 'timed out' "$timeout_log" || fail "timeout message missing"
grep -q 'worktree=' "$timeout_log" || fail "timeout did not name the holder"
wait "$holder" || fail "timeout holder failed"
assert_absent
export PICTURA_VERIFY_LOCK_TIMEOUT=10

# bl_timeout_override: 0 waits until release instead of using the default.
export PICTURA_VERIFY_LOCK_TIMEOUT=0
hold_async 2
holder=$HOLDER_PID
wait_for_lock || fail "zero-timeout holder did not acquire the lock"
( . "$helper"; build_lock_install_traps; build_lock_acquire ) 2>"$sandbox/zero.log" &
zero_waiter=$!
sleep 1
kill -0 "$zero_waiter" 2>/dev/null || fail "PICTURA_VERIFY_LOCK_TIMEOUT=0 exited early"
wait "$holder" || fail "zero-timeout holder failed"
wait "$zero_waiter" || fail "zero-timeout waiter failed"
assert_absent
export PICTURA_VERIFY_LOCK_TIMEOUT=10

# bl_invalid_timeout: a non-numeric value is rejected.
for bad in abc 00 -1 10abc; do
    export PICTURA_VERIFY_LOCK_TIMEOUT="$bad"
    invalid_log="$sandbox/invalid.log"
    set +e
    ( . "$helper"; build_lock_acquire ) 2>"$invalid_log"
    invalid_rc=$?
    set -e
    [ "$invalid_rc" -ne 0 ] || fail "invalid timeout '$bad' was accepted"
    grep -q 'invalid PICTURA_VERIFY_LOCK_TIMEOUT' "$invalid_log" || fail "invalid-timeout message missing for '$bad'"
    assert_absent
done
export PICTURA_VERIFY_LOCK_TIMEOUT=10

# bl_stale: a lock whose owner PID is dead is reclaimed.
sh -c 'exit 0' &
dead_pid=$!
wait "$dead_pid" 2>/dev/null || true
mkdir -m 700 "$lockdir"
printf 'pid=%s\nworktree=/stale\nbranch=stale\nscript=stale\nsince=stale\n' "$dead_pid" >"$lockdir/owner"
export PICTURA_VERIFY_LOCK=1
build_lock_acquire || fail "stale lock was not reclaimed"
assert_present
build_lock_release
assert_absent

# bl_invalid_pid: pid=0 is not a live owner.
mkdir -m 700 "$lockdir"
printf 'pid=0\nworktree=/stale\nbranch=stale\nscript=stale\nsince=stale\n' >"$lockdir/owner"
export PICTURA_VERIFY_LOCK=1
build_lock_acquire || fail "pid=0 lock was not reclaimed"
build_lock_release
assert_absent

# bl_empty_owner: a directory with no owner is reclaimed after the grace period.
mkdir -m 700 "$lockdir"
empty_start=$(date +%s)
build_lock_acquire || fail "owner-less lock was not reclaimed"
empty_elapsed=$(( $(date +%s) - empty_start ))
[ "$empty_elapsed" -lt 15 ] || fail "owner-less reclaim took too long"
build_lock_release
assert_absent

# bl_reclaim_race: two waiters reclaiming a dead lock must serialize.
mkdir -m 700 "$lockdir"
printf 'pid=%s\nworktree=/stale\nbranch=stale\nscript=stale\nsince=stale\n' "$dead_pid" >"$lockdir/owner"
race_log="$sandbox/race.log"
: >"$race_log"
race_pids=()
for _ in 1 2; do
    (
        . "$helper"
        build_lock_acquire || exit 1
        printf 'start\n' >>"$race_log"
        sleep 1
        printf 'end\n' >>"$race_log"
        build_lock_release
    ) 2>>"$sandbox/race.err" &
    race_pids+=("$!")
done
for race_pid in "${race_pids[@]}"; do
    wait "$race_pid" || fail "reclaim-race waiter failed"
done
race_sequence=$(tr '\n' ' ' <"$race_log")
[ "$race_sequence" = "start end start end " ] || fail "reclaim race interleaved: $race_sequence"
assert_absent

# bl_slow_git: a slow owner-record lookup must not open a publication window.
mkdir -p "$sandbox/bin"
cat >"$sandbox/bin/git" <<'GIT'
#!/usr/bin/env bash
sleep 1
case "$*" in
    *"branch --show-current"*) echo stub-branch ;;
    *"rev-parse --short HEAD"*) echo deadbee ;;
esac
exit 0
GIT
chmod +x "$sandbox/bin/git"
slow_log="$sandbox/slow-git.log"
: >"$slow_log"
slow_pids=()
for _ in 1 2; do
    (
        PATH="$sandbox/bin:$PATH"
        . "$helper"
        build_lock_acquire || exit 1
        printf 'start\n' >>"$slow_log"
        sleep 0.5
        printf 'end\n' >>"$slow_log"
        build_lock_release
    ) 2>>"$sandbox/slow-git.err" &
    slow_pids+=("$!")
done
for slow_pid in "${slow_pids[@]}"; do
    wait "$slow_pid" || fail "slow-git waiter failed"
done
slow_sequence=$(tr '\n' ' ' <"$slow_log")
[ "$slow_sequence" = "start end start end " ] || fail "slow-git publication race interleaved: $slow_sequence"
assert_absent

# bl_signal: TERM and INT release the lock and stop the heavy child.
export PICTURA_VERIFY_LOCK=1
for sig in TERM INT; do
    hold_async 30
    holder=$HOLDER_PID
    wait_for_lock || fail "$sig holder did not acquire the lock"
    child=""
    for _ in {1..100}; do
        child=$(child_of "$holder")
        [ -n "$child" ] && break
        sleep 0.05
    done
    [ -n "$child" ] || fail "$sig holder did not start a child"
    kill -"$sig" "$holder"
    wait "$holder" 2>/dev/null || true
    sleep 0.2
    assert_absent
    if kill -0 "$child" 2>/dev/null; then
        kill -KILL "$child" 2>/dev/null || true
        fail "heavy child survived $sig"
    fi
done
unset PICTURA_VERIFY_LOCK

echo "check-build-lock: OK"
