# Spec Delta

## ADDED Requirements

### Requirement: Opt-in serialization of local verification runs

The local verification scripts SHALL support an **opt-in** lock shared by all
worktrees of the invoking user on the machine, so concurrent runs queue instead
of overlapping their heavy CMake, cargo, Qt link, and test steps. The lock SHALL
be disabled unless enabled by `PICTURA_VERIFY_LOCK=1`. Both
`PICTURA_VERIFY_LOCK` and `PICTURA_VERIFY_LOCK_TIMEOUT` SHALL be resolved from
the process environment first and, only when a variable is absent there, from
the user-global env file
`${KOOKA_ENV_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/kooka-pictura/env}` read as
data. A variable present in the process environment SHALL win even when its
value is `0` or empty. No repository-tracked file SHALL be read by default;
`KOOKA_ENV_FILE` SHALL be taken only from the process environment.

When enabled, the lock SHALL use an atomic, portable primitive that works on
local filesystems on Linux, macOS, and git-bash without requiring `flock`,
`/proc`, GNU `stat`, or Linux-only sockets. It SHALL be scoped per user and
SHALL create its directory with owner-only permissions. Before reading or
reclaiming a pre-existing lock directory, the scripts SHALL verify that the
invoking user owns it and that it is not a symlink; otherwise they SHALL fail
with a clear message. The lock SHALL record the owner PID, worktree, branch (or
short commit when detached), script, and start time.

A run that cannot take the lock SHALL wait, report the current holder and its
worktree, and proceed once the holder releases it. While one run holds the lock,
a nested invocation of the other verification script SHALL recognize that it is
already inside the critical section, SHALL NOT acquire the lock again, and SHALL
NOT release a lock it did not acquire. `scripts/verify-full.sh` SHALL acquire the
lock before its first heavy build step and hold it through its whole run,
including the nested `scripts/verify-fast.sh` and the live control check.
`scripts/verify-fast.sh` SHALL acquire the lock only after its docs-only early
exit, so a docs-only `verify-fast` run SHALL NOT create, acquire, or wait on the
lock; `scripts/verify-full.sh` always builds and therefore always locks when
enabled.

The wait SHALL be bounded by `PICTURA_VERIFY_LOCK_TIMEOUT` in seconds (default
`3600`; `0` means no limit). A value that is neither `0` nor a positive decimal
integer SHALL be rejected with a clear error and a non-zero exit. On expiry the
run SHALL exit non-zero, name the holder, and leave no lock behind. A lock whose
recorded owner PID is not a live process, or whose owner record is absent after
a short grace period, SHALL be reclaimed instead of blocking indefinitely, and
the reclaim SHALL be race-safe so two waiters cannot remove a freshly created
lock. Releasing the lock SHALL happen on normal exit and on interrupt or
termination.

#### Scenario: The lock is disabled by default [bl_off]

- **WHEN** a verification script runs with `PICTURA_VERIFY_LOCK` absent from the
  process environment and the user-global env file, or set to `0`
- **THEN** no lock is created or acquired and the run behaves as before

#### Scenario: The process environment wins over the env file [bl_precedence]

- **WHEN** the user-global env file sets `PICTURA_VERIFY_LOCK=1` and the process
  environment sets it to `0`
- **THEN** the lock is disabled for that run

#### Scenario: Concurrent runs queue [bl_queue]

- **WHEN** the lock is enabled and a second run starts while another run holds it
- **THEN** the second run reports the holder and its worktree, waits, and
  proceeds only after the first releases the lock

#### Scenario: The nested full-to-fast invocation does not deadlock [bl_nested]

- **WHEN** `scripts/verify-full.sh` holds the lock and invokes
  `scripts/verify-fast.sh`
- **THEN** the nested script treats itself as already inside the critical
  section, does not acquire the lock again, and completes the full run

#### Scenario: The nested invocation does not release the parent's lock [bl_nested_release]

- **WHEN** a nested `scripts/verify-fast.sh` exits while the parent holds the lock
- **THEN** the lock directory still exists and a third run continues to queue

#### Scenario: A docs-only fast run stays lock-free [bl_docs_only]

- **WHEN** the lock is enabled and `scripts/verify-fast.sh` detects a docs-only
  working diff
- **THEN** it runs its docs-only fast path without creating, acquiring, or
  waiting on the lock

#### Scenario: The wait expires with the holder identified [bl_timeout]

- **WHEN** the lock remains held until `PICTURA_VERIFY_LOCK_TIMEOUT` elapses
- **THEN** the waiting run exits non-zero, names the holder, and leaves no lock
  behind

#### Scenario: The opt-in is read from the user-global env file [bl_envfile]

- **WHEN** `PICTURA_VERIFY_LOCK` is absent from the process environment and the
  user-global env file sets it to `1`
- **THEN** the scripts treat the lock as enabled

#### Scenario: The timeout is overridden [bl_timeout_override]

- **WHEN** `PICTURA_VERIFY_LOCK_TIMEOUT` is set to a positive number of seconds
- **THEN** the run waits at most that long instead of the 3600-second default,
  and `0` waits without a deadline

#### Scenario: An invalid timeout is rejected [bl_invalid_timeout]

- **WHEN** `PICTURA_VERIFY_LOCK_TIMEOUT` is not `0` or a positive decimal integer
- **THEN** the run reports the invalid value and exits non-zero instead of
  silently using a different bound

#### Scenario: A dead holder is reclaimed [bl_stale]

- **WHEN** the lock records an owner PID that is not a live process
- **THEN** the next run reclaims the lock and proceeds instead of blocking

#### Scenario: Reclaim is race-safe [bl_reclaim_race]

- **WHEN** two waiters find the same dead holder at the same time
- **THEN** exactly one reclaims it and the waiters do not both enter the
  critical section or remove a freshly created lock

#### Scenario: An owner-less lock directory is reclaimed [bl_empty_owner]

- **WHEN** a lock directory has no owner record
- **THEN** a waiter re-reads it after a short grace period and reclaims it if it
  is still empty, rather than blocking until the full timeout

#### Scenario: Interrupt or termination releases the lock [bl_signal]

- **WHEN** a lock holder receives `INT` or `TERM`
- **THEN** it removes the lock it owns before exiting

#### Scenario: A repository-tracked file cannot enable the lock [bl_notracked]

- **WHEN** a repository-local `.env` sets `PICTURA_VERIFY_LOCK=1` and no
  `KOOKA_ENV_FILE` override is present in the process environment
- **THEN** the lock stays disabled

#### Scenario: The opt-in is documented [bl_docs]

- **WHEN** a reader opens the worktree guidance in `DEVELOPING.md`
- **THEN** it states that serialization is opt-in and off by default, names the
  enabling and timeout variables, gives the default timeout, and explains that
  the lock covers CMake and cargo work across worktrees

### Requirement: A runnable build-lock check

The repository SHALL provide a check that proves the opt-in lock without
touching the real lock or the real user-global env file. It SHALL build an
isolated sandbox and test the lock helper directly, without invoking
`scripts/verify-fast.sh` or `scripts/verify-full.sh`, so the check cannot
recurse or run a build. It SHALL assert that: a second run waits for a held lock
and proceeds after release; a nested invocation neither acquires nor releases
the parent's lock; a docs-only fast run takes no lock; an expired wait fails
without leaving a lock; a dead owner is reclaimed race-safely; signal release
works; the process environment wins over the env file; and the disabled path
never locks. The check SHALL exit non-zero on any failed assertion and SHALL be
run by the full local fast gate. The repository SHALL NOT wire it into CI for
this change.

#### Scenario: The check passes on a correct lock [bl_check]

- **WHEN** the build-lock check runs against the committed scripts
- **THEN** every assertion holds and the check exits 0

#### Scenario: A broken lock fails the check [bl_check_fail]

- **WHEN** a change lets a nested invocation re-acquire or release the held lock,
  or skips a required wait
- **THEN** the check reports the failing assertion and exits non-zero
