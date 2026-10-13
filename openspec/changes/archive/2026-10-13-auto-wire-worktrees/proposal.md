# Proposal

## Why

Worktrees are the intended development layout, but creating one today is manual and half-broken: the local-only `reference-fixtures/` is missing until hand-copied, a fresh `target/` means recompiling into another multi-GB tree (the main checkout's is already 52G), and nothing tells a contributor what a worktree needs afterward. A committed post-checkout hook wires each linked worktree the moment it appears.

## What Changes

- Commit `.githooks/post-checkout` that, in linked worktrees only, symlinks `reference-fixtures` to the main checkout's copy and creates `../.target` then symlinks `target` to it, so all worktrees under one worktrees root share one cargo artifact tree.
- Enable the hook once per clone with `git config core.hooksPath .githooks` (documented, not automated).
- Drop the trailing slash on the `.gitignore` `/reference-fixtures/` pattern so the symlink (not just a directory) stays ignored and plain `git worktree remove` still sees a clean tree.
- Add a `## Worktrees` section to `DEVELOPING.md` (root file — no `TASK-ALLOWS-DOCS`): setup line, naming, cmake configure, sccache and serena pointers, shared-target caveat, cleanup.
- Add `scripts/check-worktree-hook.sh` — a temp-repo self-check that exercises the hook end to end — and wire it into `scripts/verify-fast.sh` and the guards CI workflow.

## Capabilities

### New Capabilities
- `meta/worktree-hooks`: The committed post-checkout hook, its linked-worktree-only idempotent behavior, the shared `target`/`reference-fixtures` symlinks, the one-time `core.hooksPath` opt-in, and the DEVELOPING.md worktree workflow documentation.

### Modified Capabilities
- `verification/verification-harness`: The guards workflow and `verify-fast.sh` requirements gain `scripts/check-worktree-hook.sh` alongside the existing script checks.

## Impact

- New files: `.githooks/post-checkout`, `scripts/check-worktree-hook.sh`.
- Edited: `.gitignore` (one pattern), `scripts/guard.sh` (one pathspec exemption for the check script), `DEVELOPING.md` (one section), `scripts/verify-fast.sh` (a check block), `.github/workflows/guards.yml` (one step).
- No Rust/C++ code, no new dependencies, no CI beyond the guards workflow.
- Contributors opt in per clone; checkouts of pre-change commits find no `.githooks/` and silently skip the hook.
- Design-only note: `.githooks/` is reserved for #69's commit-msg linters (plain shell — lefthook must not own `core.hooksPath`); nothing in this change implements that.
