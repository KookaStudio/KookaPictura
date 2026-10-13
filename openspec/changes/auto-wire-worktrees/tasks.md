# Tasks

## 1. Hook and ignore

- [x] 1.1 Add executable `.githooks/post-checkout` (`#!/usr/bin/env bash`): `cd` to `$(git rev-parse --show-toplevel)`, exit 0 unless linked worktree (`--git-dir` != `--git-common-dir`); `[ -e x ] || [ -L x ]` entry guards; `mkdir -p ../.target` then `ln -s ../.target target`; symlink `reference-fixtures` to the main checkout (via `--git-common-dir` minus `/.git`); diagnostics on stderr, unconditional `exit 0` — verify `bash -n .githooks/post-checkout` and that a hand-run from a subdirectory behaves per design D2
- [x] 1.2 Change `.gitignore` `/reference-fixtures/` to `/reference-fixtures` — verify `git check-ignore reference-fixtures` still matches the main checkout's real directory and `git check-ignore` also matches a symlink path (pattern form per design D5)
- [x] 1.3 In a throwaway clone under `mktemp -d` (throwaway `user.name`/`user.email`): `git config core.hooksPath .githooks`, seed a `reference-fixtures/` dir, commit, `git worktree add`, assert both symlinks resolve, switch branches (silent), checkout in the main tree (no symlinks), `git worktree add --no-checkout` + first checkout wires it, `git worktree remove` without `--force` succeeds and the shared `.target` remains — then delete the throwaway
- [x] 1.4 Edge cases in the throwaway: run the hook with no `reference-fixtures` source (target symlink created, fixtures step skipped, exit 0); pre-create a real `target/` and a dangling `reference-fixtures` symlink (hook still exits 0, checkout succeeds) — then delete the throwaway

## 2. Runnable check

- [x] 2.1 Add `scripts/check-worktree-hook.sh` (mktemp sandbox; `git init`; throwaway identity; seed `reference-fixtures/`; `git config core.hooksPath .githooks`; commit the hook; worktree add; assert both symlinks; assert idempotent second checkout; exercise the missing-fixtures skip path; `trap` cleanup; exit non-zero on failure) — verify `bash scripts/check-worktree-hook.sh` exits 0 and a second run (no leftover state) also exits 0
- [x] 2.2 Add `':!scripts/check-worktree-hook.sh'` to `scripts/guard.sh` check 2d's pathspec list (design D7) — verify `bash scripts/guard.sh` passes with the new script tracked
- [x] 2.3 Deliberate-failure check: break the hook's symlink line, confirm the script fails and names the assertion, restore the hook — verify the script is non-vacuous (broken rc=1, restored rc=0)
- [x] 2.4 Shared-target spike: two worktrees sharing `../.target`, build a workspace crate in A, then B, then A; record the fresh-vs-recompiled counts for the A return build as the task result (criterion: counts recorded, either outcome accepted per design Risks) — result: `pictura-core` (zero-dep crate), counts A1=1 compiled, B1=0, A2 (return)=0, A3=0; no cross-worktree recompile ping-pong observed

## 3. Wiring

- [x] 3.1 Run `scripts/check-worktree-hook.sh` from `scripts/verify-fast.sh`'s full gate next to `check-file-size.sh` (echo line + command, matching neighbors) — verify `bash scripts/verify-fast.sh` runs it on a code diff
- [x] 3.2 Add a guards-workflow step `bash scripts/check-worktree-hook.sh` — verify the workflow YAML parses (`actionlint` if available) and the step command matches the other script steps

## 4. Docs

- [x] 4.1 Add `## Worktrees` to `DEVELOPING.md` between Common tasks and Speeding up local builds: `git config core.hooksPath .githooks` (plus the personal-hooks warning), naming `../kooka-pictura-worktrees/{issue}-{name}` + link to AGENTS.md branch conventions, cmake configure line copied from Build and run, pointers to the sccache section and the one-time `serena project index`, one-builder-at-a-time shared-target caveat, `git worktree remove` cleanup — verify every command in the section runs as written on a fresh clone
- [x] 4.2 Confirm the docs commit needs no `TASK-ALLOWS-DOCS` (root `DEVELOPING.md`, not `docs/`) — verify `bash scripts/guard.sh` passes without the marker

## 5. Integration

- [x] 5.1 `openspec validate --all --strict` passes and `bash scripts/verify-fast.sh` is green end to end
- [x] 5.2 In this clone run the documented `git config core.hooksPath .githooks`, then acceptance: fresh `git worktree add ../kooka-pictura-worktrees/376-acceptance` wires both symlinks, branch switch inside is a silent no-op, main-checkout checkout creates nothing, `git worktree remove` succeeds without `--force`, shared `../.target` remains — then remove the acceptance worktree
- [ ] 5.3 Update issue #376 acceptance checkboxes: items 1–4 from 5.2, item 5 after `verify-fast` (5.1) and the guards workflow pass on the PR; note the spike counts from 2.4 in the PR description

## 6. Commit and PR

- [x] 6.1 Claim issue #376 (assign + short start comment) — verify `gh issue view 376` shows the assignment
- [x] 6.2 Commit the change as `build: auto-wire local worktrees via a post-checkout hook (#376)` (Conventional Commits per AGENTS.md; DEVELOPING.md root change needs no docs marker) — verify `git log -1 --format=%s` matches the convention
- [ ] 6.3 Open the PR titled `build: auto-wire local worktrees via a post-checkout hook (#376)` — verify `gh pr checks` shows the `pr-conventions` title check green