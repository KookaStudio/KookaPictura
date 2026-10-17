# Proposal

## Why

Several worktrees or agent instances can run the heavy local verification
scripts at the same time. `scripts/verify-full.sh:24` builds CMake/Ninja with
`--parallel`, and `scripts/verify-fast.sh` follows with `cargo clippy`,
`cargo nextest`, doctests, and `ctest`; each defaults to `nproc` jobs. Only the
shared cargo `target` tree is serialized by cargo's own lock — the per-worktree
CMake trees and Qt link steps are not. Overlapping runs have thrashed a desktop
until it needed a hard reset (issue #468). Developers who want protection need a
way to keep those runs from overlapping without changing the default workflow.

## What Changes

- Add an **opt-in** lock shared by all worktrees of the invoking user on the
  machine. It is off unless `PICTURA_VERIFY_LOCK=1`, resolved from the process
  environment first and then from the user-global env file
  (`${KOOKA_ENV_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/kooka-pictura/env}`).
  A value set in the process environment wins even when it is `0`.
- Bound the wait with `PICTURA_VERIFY_LOCK_TIMEOUT` (seconds, default `3600`;
  `0` means no limit), resolved the same way. Reject a non-numeric value with a
  clear error. On timeout, exit non-zero naming the current holder and leave no
  lock behind.
- Use an atomic `mkdir` lock directory at
  `${XDG_RUNTIME_DIR:-${TMPDIR:-/tmp}}/kooka-pictura-build-$(id -u).lock`, with
  mode `700`, an owner metadata file (`pid`, worktree, branch, script, `since`),
  and ownership/symlink checks on the fallback path, so the portable primitive
  works on local filesystems on Linux, macOS, and git-bash without `flock`,
  `/proc`, GNU `stat`, or abstract sockets. Reclaim the lock when the recorded
  PID is not live, using a reap lock so two waiters cannot remove a fresh lock.
- `scripts/verify-full.sh` acquires once and holds the lock across CMake
  configure/build, the cache rebuild check, `verify-fast.sh`, and
  `verify-control.sh`. It exports an internal held marker so a nested
  `verify-fast.sh` recognizes it is already inside, does not acquire again, and
  does not release the parent's lock.
- `scripts/verify-fast.sh` acquires only after its docs-only early exit, so a
  docs-only `verify-fast` run still runs lock-free. `verify-full.sh` always
  builds and therefore always locks when the opt-in is enabled.
- Document the opt-in and timeout next to the worktree guidance in
  `DEVELOPING.md`, and list the variables in `.env.example` (commented).
- Add a sandboxed helper-level self-check that proves queueing, nested reuse and
  non-release, timeout, stale reclaim, signal release, precedence, and the
  disabled path.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `verification/verification-harness`: add requirements specifying the opt-in
  serialization behavior (queueing, precedence, the nested full-to-fast path,
  the docs-only `verify-fast` exception, timeout semantics, stale-holder
  reclaim, release on signals) and the runnable build-lock check.

## Impact

- `scripts/verify-fast.sh`, `scripts/verify-full.sh`.
- New `scripts/build-lock.sh` (sourceable lock logic) and
  `scripts/check-build-lock.sh` (sandboxed check wired into `verify-fast.sh`).
- `.env.example` and `DEVELOPING.md`.
- No app or engine code, no new runtime dependency, and no CI change: this
  branch targets the local developer workflow only. Because the lock is
  opt-in, the default verification behavior is unchanged.
- Tracks issue #468. Out of scope: capping per-build job counts
  (`CARGO_BUILD_JOBS`/`ninja -j`), systemd `MemoryMax`, per-worktree target
  directories, and direct `scripts/test-report.sh` /
  `check-local-build-caching.sh --rebuild` invocations.
