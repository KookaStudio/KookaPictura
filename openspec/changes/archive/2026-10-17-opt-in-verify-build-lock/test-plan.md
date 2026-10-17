# Test Plan

Shell-script behavior is exercised by `scripts/check-build-lock.sh`. The
repository's `scripts/check-test-plan.py` resolves only Rust (`<crate>::<fn>`)
and Qt (`tst_<suite>::<case>`) references, so those rows are marked `manual:`
with the exact shell check case named in the reason; the check still runs
automatically.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `bl_off` | Run the helper in a sandbox with `PICTURA_VERIFY_LOCK` unset, then `0` | No lock directory is created; acquire and release are no-ops | manual: `check-build-lock.sh` `bl_off` case; the parser resolves only Rust/Qt refs |
| `bl_precedence` | Set the env file to `1` and the process environment to `0` | The lock is disabled; the process value wins | manual: `check-build-lock.sh` `bl_precedence` case; the parser resolves only Rust/Qt refs |
| `bl_queue` | Hold the lock in the background, then acquire from a waiter | The waiter prints the holder and worktree, waits, then proceeds after release | manual: `check-build-lock.sh` `bl_queue` case; the parser resolves only Rust/Qt refs |
| `bl_nested` | Set `PICTURA_VERIFY_LOCK_HELD=1`, then acquire | Acquire returns immediately and creates no lock | manual: `check-build-lock.sh` `bl_nested` case; the parser resolves only Rust/Qt refs |
| `bl_nested_release` | Hold the lock, run a nested acquire/release with the marker, then inspect the lock | The lock directory still exists after the nested release | manual: `check-build-lock.sh` `bl_nested_release` case; the parser resolves only Rust/Qt refs |
| `bl_docs_only` | Check the line order in `verify-fast.sh` | The acquire call appears after the docs-only `exit 0` | manual: `check-build-lock.sh` `bl_docs_only` static assertion; the parser resolves only Rust/Qt refs |
| `bl_timeout` | Hold the lock with a 1-second timeout configured | The waiter exits non-zero, names the holder, and leaves no lock | manual: `check-build-lock.sh` `bl_timeout` case; the parser resolves only Rust/Qt refs |
| `bl_envfile` | Unset the process variable and put `1` in the sandbox env file | Acquire takes the lock | manual: `check-build-lock.sh` `bl_envfile` case; the parser resolves only Rust/Qt refs |
| `bl_timeout_override` | Use a 1-second override, then `0` with a short observation window | The override is honoured; `0` keeps waiting until release | manual: `check-build-lock.sh` `bl_timeout_override` case; the parser resolves only Rust/Qt refs |
| `bl_invalid_timeout` | Set the timeout to `abc` | The helper reports the value and exits non-zero | manual: `check-build-lock.sh` `bl_invalid_timeout` case; the parser resolves only Rust/Qt refs |
| `bl_stale` | Create a lock whose owner PID is not alive | The next acquire reclaims it and succeeds | manual: `check-build-lock.sh` `bl_stale` case; the parser resolves only Rust/Qt refs |
| `bl_reclaim_race` | Create a dead-owner lock and start two waiters | The waiters serialize (no interleaved critical sections) and both finish | manual: `check-build-lock.sh` `bl_reclaim_race` case; the parser resolves only Rust/Qt refs |
| `bl_empty_owner` | Create a lock directory with no owner record | A waiter reclaims it after the grace period instead of waiting for the timeout | manual: `check-build-lock.sh` `bl_empty_owner` case; the parser resolves only Rust/Qt refs |
| `bl_signal` | Acquire in a child, send `TERM`, then inspect the lock | The child removes the lock it owns | manual: `check-build-lock.sh` `bl_signal` case; the parser resolves only Rust/Qt refs |
| `bl_notracked` | Put `PICTURA_VERIFY_LOCK=1` in a repo-local `.env` and unset `KOOKA_ENV_FILE` | The lock stays disabled | manual: `check-build-lock.sh` `bl_notracked` case; the parser resolves only Rust/Qt refs |
| `bl_docs` | Read the worktree guidance in `DEVELOPING.md` | It says the lock is opt-in and off by default, names both variables, gives the default timeout, and explains the cross-worktree scope | manual: reviewed in the `DEVELOPING.md` diff |
| `bl_check` | Run `bash scripts/check-build-lock.sh` | Every assertion holds and the script exits 0 | manual: `check-build-lock.sh` full run |
| `bl_check_fail` | Temporarily weaken the nesting marker or the stale-reclaim path and rerun the check | The check reports the failing assertion and exits non-zero | manual: negative case verified by temporarily weakening the helper |
