# Proposal

## Why

`sccache` keys compiled objects by absolute source path, so the same C++ unit
built in two worktrees of the same repository produces two cache entries and a
second worktree misses everything the first already compiled. The repository's
whole workflow is worktrees, and each one currently pays a cold C++ build even
when its sources are identical to a sibling's. `SCCACHE_BASEDIRS` normalizes the
path prefix so those builds share one cache; the worktree hook is the natural
place to keep that list current.

## What Changes

- `.githooks/post-checkout` gains an **opt-in** step: when enabled, it writes a
  managed `sccache` config whose `basedirs` lists every worktree root and sets
  `hash_working_directory = false`, so C++ compilation is shared across
  worktrees. It regenerates only when the content changes, restarts the daemon
  only then, and never touches a config file it did not create.
- Opt-in is explicit: `git config kooka.sccache-share true` (or
  `KOOKA_SCCACHE_SHARE=1`). Default behaviour is unchanged.
- `AGENTS.md` gets a short pointer; `DEVELOPING.md` gets the extended
  instructions (what it writes, how to opt out, storage bound, caveats).
- `scripts/check-worktree-hook.sh` grows cases for the opt-in, the no-clobber
  rule, and the no-op path.

Out of scope: changing worktree naming, precompiled headers, `rustc` caching
(Rust already shares `../.target`), and Windows.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `meta/worktree-hooks`: the post-checkout hook gains a third, opt-in
  responsibility (sharing the C++ compiler cache across worktrees), and the
  worktree documentation requirement gains the opt-in and its `DEVELOPING.md`
  and `AGENTS.md` coverage.

## Impact

- `.githooks/post-checkout`: new opt-in cache-sharing step.
- `scripts/check-worktree-hook.sh`: new assertions.
- `AGENTS.md`, `DEVELOPING.md`: documentation (root files; no
  `TASK-ALLOWS-DOCS`).
- Writes `~/.config/sccache/config` (Linux) only when opted in and only if that
  file is absent or already managed; restarts the local `sccache` daemon on a
  content change. No new dependencies; no effect unless opted in.
