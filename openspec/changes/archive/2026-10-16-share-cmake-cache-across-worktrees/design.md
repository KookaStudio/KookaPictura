# Design

## Context

See `proposal.md`. The post-checkout hook (`meta/worktree-hooks`) already wires
each new worktree (`target`, `reference-fixtures`) and is idempotent with an
always-exit-0 contract. This change adds a third, opt-in responsibility.

Verified on the installed `sccache` 0.17.0 (spike, two directories, absolute
source paths, per-directory cwd):

```
basedirs + hash_working_directory=false   hits=1 misses=1   (share works)
hash_working_directory=false only         hits=0 misses=2   (basedirs is essential)
basedirs only                             hits=1 misses=1   (hits, but direct-mode cache key still varies)
```

So `basedirs` is the mechanism; `hash_working_directory=false` keeps the
preprocessor (direct-mode) cache key from varying by directory, preserving the
fast path.

## Goals / Non-Goals

**Goals:**
- A second worktree reuses C++ objects compiled in a sibling worktree.
- The behaviour is strictly opt-in and never touches a config the repo did not write.

**Non-Goals:**
- Rust caching (the shared `../.target` already covers it).
- Precompiled headers, worktree renaming, Windows local dev.

## Decisions

### D1. Opt-in via git config (cascades), env as override

`git config kooka.sccache-share true` writes the repository's *common* config
(`.git/config`), which every worktree reads — the same mechanism that makes
`core.hooksPath` reach worktrees. Set once, every new worktree's hook sees it.
`KOOKA_SCCACHE_SHARE=1` is honored too, for shells that prefer env. Absent both
→ the step is skipped (non-goals stay non-goals).

### D2. basedirs from `git worktree list`

The hook regenerates the list from `git worktree list --porcelain` on each run,
so a newly created worktree adds itself automatically. No enumeration to
maintain by hand.

### D3. Also disable `hash_working_directory`

`basedirs` normalizes the source and argument paths; the direct-mode cache key
additionally hashes the working directory, which differs per worktree. Setting
`hash_working_directory = false` under `[cache.disk.preprocessor_cache_mode]`
keeps that fast path shared. Both live in the generated file.

### D4. Repo-specific sentinel + never clobber

The hook owns the file only if it is non-empty and its first line is the managed
sentinel for *this* repository. The sentinel embeds the repo's common-dir root,
so a config written by another clone or project is treated as unmanaged and left
untouched (with a stderr note); an empty file is treated as writable. This
respects a user's own cache settings and avoids two opted-in repos thrashing one
machine-global file.

### D5. Write only on change, restart only on write

The generated content is compared with what is on disk; identical → return
without touching the daemon. This satisfies the existing idempotency contract
(a branch switch is a silent no-op) and avoids needless daemon churn.

### D6. Config path: `SCCACHE_CONF`, else platform default

Honour `SCCACHE_CONF`; otherwise `${XDG_CONFIG_HOME:-$HOME/.config}/sccache/config`
on Linux and `~/Library/Application Support/Mozilla.sccache/config` on macOS.

### D7. Optional step never fails the checkout

Every failure path reports on stderr and exits 0, matching the hook's existing
contract. The step is skipped entirely when `sccache` is absent.

## Risks / Trade-offs

- [`hash_working_directory=false` weakens key uniqueness] → basedirs already
  normalizes the paths that would otherwise differ; the residual collision case
  (same source and flags compiled from a different cwd) is not real here.
- [A removed worktree leaves a stale root in `basedirs`] → harmless: a prefix
  that matches nothing is never stripped; it is dropped on the next `worktree add`.
- [The config and the daemon are machine-global, not per-repo] → the config file
  is user-wide, so `hash_working_directory = false` also applies to other
  projects using sccache on the account; and `sccache --stop-server` admits no
  per-repo restart, so adding a worktree can restart a server a concurrent build
  is using. Mitigated by explicit opt-in, change-only writes, and documenting
  `SCCACHE_IGNORE_SERVER_IO_ERROR=1`; scoping the file per repo is not possible —
  sccache reads one config path.
- [Only one opted-in repository can own the file at a time] → the sentinel is
  repo-specific, so another repo's managed config is left alone rather than
  clobbered; documented.
- [Only identical source and flags hit; different branches still miss] → expected
  and documented; the win is same-base worktrees (the agent workflow).
- [Windows local dev is untested] → documented as Linux/macOS.
