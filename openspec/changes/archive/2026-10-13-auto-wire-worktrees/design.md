# Design

## Context

No `.githooks/` exists, `core.hooksPath` is unset in this clone (samples only in `.git/hooks`), and zero linked worktrees exist. `post-checkout` runs after `git worktree add` / `clone` / branch checkout, with cwd at the root of the working tree being updated, and its exit status *is* the checkout status for the checkout family (githooks(5); empirically the same holds for `worktree add` — a hook exiting non-zero fails it). Relative `core.hooksPath` resolves against that same directory (git-config(1)), so each worktree finds its own checked-out `.githooks/`. `git worktree remove` refuses when `git status --porcelain` outputs anything; ignored files are invisible to porcelain (`builtin/worktree.c` `check_clean_worktree`). See proposal.md for motivation.

## Goals / Non-Goals

**Goals:**
- Wire linked worktrees the moment they appear, with zero manual symlink steps.
- One shared cargo tree per worktrees root so N worktrees do not cost N × 52G.
- Keep the main checkout, CI, and `guard.sh` behavior untouched.

**Non-Goals:**
- Sharing `build/`, `.serena`, or migrating the main checkout's `target/` (proposal Out of scope).
- Making the hook run cmake configure or any minutes-long step.
- Automating `git config core.hooksPath` — the one-liner stays documented, not scripted.

## Decisions

**D1 — post-checkout hook, not a wrapper script for `worktree add`.**
A hook covers every future worktree (including `gh pr checkout` scratch trees) with one committed file; a helper script only covers invocations that remember to call it. Alternative rejected: shell function in docs — drifts, not shared via clone.

**D2 — Detect linked worktrees by `--git-dir` != `--git-common-dir`, evaluated from the toplevel.**
The standard idiom, but only valid when both paths are resolved the same way: at the worktree root (where githooks(5) always chdirs before running a hook) the main checkout reports `.git` for both. From a *subdirectory* the raw strings differ (`/abs/.git` vs `../../.git`), which would misclassify the main checkout — so the hook starts with `cd "$(git rev-parse --show-toplevel)"`, a no-op for real hook runs that makes hand-invocation from any depth safe. Alternative rejected: `GIT_WORK_TREE` env probing — less portable across git versions.

**D3 — `mkdir -p ../.target` *before* `ln -s`.**
A dangling `target -> ../.target` symlink makes cargo's `create_dir_all("target")` fail: `mkdir("target")` hits EEXIST, `stat` follows the link, target is not a directory, error. The issue text said "created lazily" — lazily means *in the hook, before the symlink*, not at first cargo invocation. The `-e target` guard makes this a no-op on later runs.

**D4 — Relative symlinks (`../.target`, main-root `reference-fixtures` via `--git-common-dir`).**
`../.target` lands at the worktrees root — sibling of each worktree dir — so every worktree under one root shares one pool; worktrees elsewhere get their own parent's `.target` (harmless isolation). Main checkout root is derived from `--git-common-dir` so the fixtures link survives the main checkout moving relative to the worktree. Alternatives rejected: absolute hardcoded path (breaks on move), `CARGO_TARGET_DIR` env (does not survive non-shell tooling, needs per-terminal setup).

**D5 — `.gitignore` pattern drops the trailing slash: `/reference-fixtures`.**
Directory-only patterns (`/reference-fixtures/`) do not match a *symlink* (lstat says not-a-directory), so the wired symlink would show as untracked and `git worktree remove` would refuse — failing acceptance criterion #4. No-trailing-slash matches both the real directory in the main checkout and the symlink in worktrees. `guard.sh` check 2d uses `git ls-files` and is unaffected.

**D6 — Hook always exits 0; diagnostics to stderr.**
The hook's exit status becomes the checkout status; a non-zero return would make `git worktree add`/`checkout` fail for a convenience step. Concretely: no `set -e` (or an explicit final `exit 0`), and entry guards test `-e || -L`, because `-e` is false for a *dangling* symlink while `ln -s` against it still fails EEXIST. Missing sources and pre-existing entries are skip conditions, not errors.

**D7 — Check script at `scripts/check-worktree-hook.sh`; guard pathspec gains one exemption.**
The sandbox must seed its own `reference-fixtures/` and assert on the symlink, so the script inevitably contains the literal `reference-fixtures/` that `guard.sh` check 2d greps for in `scripts/*`. Rather than mandating slash-avoidance conventions in the spec (fragile — the natural `mkdir reference-fixtures/` trips it), add `':!scripts/check-worktree-hook.sh'` to check 2d's pathspec list, mirroring guard's existing `':!scripts/guard.sh'` self-exclusion; the check's *intent* (tracked code depending on fixtures absent in clones/CI) doesn't cover a check that seeds its own copy. Shape follows `verify-control.sh`: `mktemp -d` sandbox, `git init` seed with throwaway identity, asserts, `trap` cleanup — no bats/shellspec (no repo precedent, no new deps).

**D8 — Pin #69 to plain shell hooks under `.githooks/`.**
#69's body offers "lefthook or plain shell"; lefthook typically owns `core.hooksPath`, which would fight D1's relative-per-worktree resolution. Recorded here so #69's design lands compatibly; not enforced by code in this change.

## Risks / Trade-offs

- [Relative hooksPath is per-worktree and branch-dependent] → empirical resolution: `worktree add` runs the hook file from the *invoking* checkout's root (so once the main checkout has `.githooks/`, even a pre-change branch gets wired at add time), while checkouts *inside* a worktree resolve against that worktree's own checkout. The remaining gaps: a `--no-checkout` worktree whose first checkout runs in a branch predating `.githooks/` is never wired, and switching to such a branch inside a worktree runs no hooks. Mitigation: documented; acceptable because branches forked from master after this change inherit `.githooks/`.
- [Shared `target` serializes concurrent cargo via the build-dir lock; same-named bins (`pictura-diff`, examples) can be overwritten across worktrees] → Mitigation: DEVELOPING.md caveat "one builder at a time"; spike task 5.1 measures A→B→A rebuild counts (criterion: record the counts; A-fresh vs A-recompiled are both acceptable outcomes — thrash is absorbed by sccache per design, confirmed thrash just gets documented). No `CARGO_BIN_EXE` tests exist, so test-execution exposure is nil today.
- [Personal `.git/hooks` go dark once `core.hooksPath` is set] → one-line doc warning; this clone has samples only; CI never uses local hooks.
- [Check script briefly creates a real git repo under `/tmp`] → isolated `mktemp -d`, removed by `trap`; never touches the project checkout. Wire into guards so a broken hook cannot merge.

## Migration Plan

Existing clones: pull, run the documented `git config core.hooksPath .githooks` once, recreate any pre-existing unwired worktrees (or run the hook by hand inside them). Rollback: `git config --unset core.hooksPath` restores default hooks; symlinks in existing worktrees are inert leftovers removable with the worktree.

## Open Questions

None — the spike (task 5.1) is a measurement, not a decision fork: both outcomes are recorded and the design stands either way (sccache absorbs workspace-crate recompiles), per Risks.
