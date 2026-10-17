# Design

## Context

See `proposal.md` — Why. The constraints that shape the approach:

- `scripts/verify-full.sh` invokes `scripts/verify-fast.sh` as a child process,
  then runs `scripts/verify-control.sh`. `scripts/verify-fast.sh` has a
  docs-only early exit before format, clippy, tests, and the guard checks.
- CI runs the individual verification steps on isolated runners; it does not
  invoke either script. This change is a local developer-tool concern and adds
  no CI wiring.
- The project already has a user-global env file at
  `${KOOKA_ENV_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/kooka-pictura/env}`.
  `.githooks/post-checkout` reads one key from it with a `sed` parser and never
  executes it.
- Supported development platforms are Linux, macOS, and Windows through WSL or
  git-bash. `flock`, `/proc`, GNU `stat`, and abstract Unix sockets are not
  available on every one of those.

## Goals / Non-Goals

**Goals:**

- Queue concurrent heavy local verification runs with no new dependency and no
  change to default behavior when the lock is off.
- Make the nested `verify-full` → `verify-fast` invocation safe by construction:
  no re-acquire and no release of the parent's lock.
- Keep docs-only `verify-fast` runs lock-free.
- Recover from a killed holder without manual cleanup, and bound the wait so an
  agent cannot hang forever.

**Non-Goals:**

- Capping per-build job counts (`CARGO_BUILD_JOBS`, `ninja -j`), systemd
  `MemoryMax`, or per-worktree target directories.
- Serializing direct `cargo`, `cmake`, `scripts/test-report.sh`, or
  `check-local-build-caching.sh --rebuild` invocations.
- Strict first-in-first-out fairness among waiters.
- Cross-user serialization; the lock is per user on the machine.
- Running the check in CI; it is wired into the local fast gate only.

## Decisions

### A portable atomic lock directory, not `flock`

Acquire by `mkdir -m 700` of
`${XDG_RUNTIME_DIR:-${TMPDIR:-/tmp}}/kooka-pictura-build-$(id -u).lock`.
`mkdir` is atomic on local filesystems on Linux, macOS, and NTFS/MSYS, needs no
dependency, and gives a directory for the owner record. Network filesystems are
not claimed. Alternatives rejected: `flock` (absent on macOS and git-bash), an
abstract Unix socket (Linux-only), and a Python `fcntl`/`msvcrt` wrapper (adds a
process layer and OS branching for a problem with a POSIX primitive).

The path is per-UID and the directory mode is `0700`. Before reading or
reclaiming a pre-existing directory, the helper requires `[ -O ]` and
`[ ! -L ]`; anything else is reported and the run fails rather than deleting an
unexpected path. Removal uses `rm -rf --` on the path without a trailing slash,
so it cannot follow a symlink presented as the lock itself.

### Both settings resolved from the process environment, then the env file

`PICTURA_VERIFY_LOCK` and `PICTURA_VERIFY_LOCK_TIMEOUT` are read from the process
environment first, then from the user-global env file using a per-key `sed`
parser, matching `.githooks/post-checkout`. The file is never `source`d, so it
cannot execute code, and a repository-tracked file cannot enable the lock.

Precedence is **set-versus-unset**, not truthiness: if a variable is present in
the process environment (even as `0` or empty), that value wins and the file is
not consulted. This is why `PICTURA_VERIFY_LOCK=0` in the process environment
disables the lock for one run even when the file enables it — the hook's
`[ "${X:-}" = 1 ] || [ file = 1 ]` pattern would not, so it is not copied.

`PICTURA_VERIFY_LOCK_TIMEOUT` accepts only `0` (no limit) or a positive decimal
integer. Anything else is a configuration error: the helper prints the offending
value and the run exits non-zero instead of silently losing the bound.

### One acquisition point per entry script, with ownership-aware nesting

`verify-full.sh` acquires before CMake. If it actually acquired the lock it
exports `PICTURA_VERIFY_LOCK_HELD=1`; `verify-fast.sh` acquires only after its
docs-only exit and returns immediately when that marker is present. The helper
also tracks whether *this process* created the lock. Release removes the
directory only when this process owns it, so the nested `verify-fast.sh` cannot
delete the parent's lock. Acquiring twice in one process is idempotent.

This avoids the non-reentrant request-for-the-same-lock deadlock (the reason a
naive "lock in both scripts" fails) without FD or `/proc` inspection that would
not be portable. Rejected alternatives: locking only inside `verify-fast` (leaves
the CMake build unprotected), per-heavy-step locking (interleaves runs and
repeats acquisition), and restructuring the scripts with `exec` (changes
ordering and output).

### Bounded wait, atomically published owner, and race-safe reclaim

After `mkdir` succeeds, a minimal `pid` owner record is written immediately so
a live creator is never mistaken for an empty lock; the full owner record is
then written to a temporary file inside the directory and renamed into place.
The branch lookup happens before `mkdir`, so no slow command sits in the
publication window. If the minimal write fails and the directory is gone, the
acquirer retries; if it remains, the directory is removed with `rmdir` and the
run fails. A waiter that finds a directory with no owner record waits a short
grace period, re-reads, and treats it as stale only if it is still empty.

The waiter prints the owner record once (including the worktree), then polls.
`PICTURA_VERIFY_LOCK_TIMEOUT` defaults to `3600` seconds; `0` means no limit. On
expiry the run exits non-zero, names the holder, and leaves no directory behind
(it never created one).

A dead owner is reclaimed under a sibling reap lock
(`mkdir "$lock_dir.reap"`). Only the waiter that wins the reap lock re-reads the
owner, renames the stale directory to a unique name, removes it, and releases the
reap lock; losers retry acquisition. This prevents two waiters from removing a
fresh lock one of them just created. Rejected alternatives: blocking forever (an
agent can hang indefinitely), and a bare `rm -rf` of a dead lock (the TOCTOU
race above).

### One sourceable helper plus a sandboxed, helper-level check

`scripts/build-lock.sh` holds the enable/acquire/release logic and is sourced by
both verify scripts. `scripts/check-build-lock.sh` drives the scenarios from a
sandbox: a temporary `HOME`, `XDG_CONFIG_HOME`, `XDG_RUNTIME_DIR`, `TMPDIR`, and
`KOOKA_ENV_FILE`, with `PICTURA_VERIFY_LOCK*` and the nesting marker cleared. It
tests the helper directly and never invokes `verify-fast.sh` or `verify-full.sh`,
so it cannot recurse or run a multi-minute build. The docs-only ordering is
checked statically in `verify-fast.sh` (the acquire appears after the docs-only
`exit 0`). Both use bash plus `git`, `mktemp`, `ps`, and POSIX tools.
`TERM`/`INT` are trapped before acquisition; the handler signals the descendant
tree, waits, then releases the lock.

## Risks / Trade-offs

- A `kill -9` or power loss leaves the lock directory behind → PID liveness
  reclaim handles it, and the owner record names the dead run for diagnosis.
- A kill between `mkdir` and the owner write is handled by the empty-owner
  grace period; after the minimal write, the recorded PID is enough to reclaim.
- PID reuse can make a dead holder look alive → the waiter still fails at the
  timeout with the holder named rather than hanging forever. The spec says
  "recorded PID is not live", not a stronger identity guarantee.
- No strict FIFO → waiters race after a release. Acceptable for a handful of
  local runs; a ticket lock is not worth the extra code.
- The held marker is trusted, so a stale inherited marker could skip locking →
  it is set only on the nested path, and the cost is a lost optimization on a
  developer machine, not a correctness failure.
- Opt-in means no protection until the developer enables it → `DEVELOPING.md`
  and `.env.example` state this prominently; the default behavior is unchanged
  intentionally.
- Per-user locking does not serialize two different accounts on one machine →
  the reported freeze case is one desktop user; cross-user coordination would
  need a shared system directory and is out of scope.
- `$UID` is not set in every shell → use `id -u`.
- Detached HEAD has no branch name → fall back to the short commit SHA in the
  owner record.
- A pre-existing foreign path in `/tmp` could block the lock → the ownership and
  symlink checks fail with a clear message instead of reading or deleting it.

## Migration Plan

- No migration. The feature is off by default; enabling or disabling it is an
  environment change, not a data or build change.
- Rollback: unset `PICTURA_VERIFY_LOCK` (or set it to `0`), or remove the new
  scripts and the docs entries.

## Open Questions

None. Direct `test-report.sh` and `--rebuild` invocations are explicitly out of
scope and can be added later behind the same helper if needed.
