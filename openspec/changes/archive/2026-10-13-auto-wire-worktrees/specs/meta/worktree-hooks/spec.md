# Spec Delta

## Purpose

Defines how a linked git worktree is discovered and wired automatically when it appears: the committed post-checkout hook, the symlinks it creates, the one-time per-clone opt-in, and the DEVELOPING.md workflow documentation that surrounds it.

## ADDED Requirements

### Requirement: A committed post-checkout hook wires linked worktrees

The repository SHALL contain an executable `.githooks/post-checkout` hook (bash, `#!/usr/bin/env bash`). Once a clone opts in with `git config core.hooksPath .githooks`, every `git worktree add` (without `--no-checkout`) SHALL run the hook in the new worktree, and the hook SHALL wire that worktree by:

- acting only in linked worktrees — detected by `git rev-parse --git-dir` differing from `--git-common-dir` — and leaving the main checkout untouched;
- creating `../.target` if it does not exist, then symlinking `target` to `../.target`, unless a `target` entry (directory or symlink) already exists;
- symlinking `reference-fixtures` to the main checkout's copy (derived from `--git-common-dir` minus `/.git`), unless a `reference-fixtures` entry (directory or symlink) already exists or the main checkout has no such directory.

The hook SHALL be idempotent: a branch switch inside an already-wired worktree SHALL be a silent no-op. The hook's exit status is the checkout status, so the hook SHALL terminate with exit 0 on every path — a failed optional step (for example `ln -s` against an unwritable parent) is reported on stderr and still exits 0. A worktree created with `--no-checkout` on a branch that contains `.githooks/post-checkout` SHALL be wired on its first subsequent checkout.

#### Scenario: A fresh linked worktree is wired

- **WHEN** `git worktree add` creates a linked worktree from a branch that contains `.githooks/post-checkout`, with `core.hooksPath` set to `.githooks`
- **THEN** the new worktree contains a `target` symlink resolving to `../.target` and, when the main checkout has `reference-fixtures/`, a `reference-fixtures` symlink resolving to the main checkout's directory

#### Scenario: Branch switches inside a worktree are no-ops

- **WHEN** a contributor switches branches inside an already-wired linked worktree
- **THEN** the hook runs, changes nothing, prints nothing on stdout, and exits 0

#### Scenario: The main checkout is unaffected

- **WHEN** a checkout runs in the main worktree (where `--git-dir` equals `--git-common-dir`)
- **THEN** the hook exits 0 without creating or modifying any symlink

#### Scenario: A missing fixtures source skips that step

- **WHEN** the hook runs in a linked worktree whose main checkout has no `reference-fixtures/` directory
- **THEN** the `target` symlink is still created and the `reference-fixtures` step is skipped without failing

#### Scenario: The shared target tree is never dangling

- **WHEN** the hook finishes creating the `target` symlink for the first time
- **THEN** the `target` symlink resolves to an existing directory immediately after the hook exits

#### Scenario: A no-checkout worktree is wired on first checkout

- **WHEN** `git worktree add --no-checkout` creates a linked worktree and the first real checkout later runs inside it
- **THEN** that checkout wires the worktree (both symlinks present afterwards)

#### Scenario: A failed optional step does not fail the checkout

- **WHEN** an optional step cannot complete (for example `../.target` cannot be created because the worktrees root is unwritable)
- **THEN** the hook reports the problem on stderr, exits 0, and the checkout or `git worktree add` succeeds

### Requirement: Worktree wiring keeps removal clean

The `.gitignore` pattern for `reference-fixtures` SHALL match a symlink named `reference-fixtures` at the repository root (not only a directory), so a wired linked worktree has no untracked entries from the hook. `git worktree remove` on a wired worktree SHALL therefore succeed without `--force` once the tree is otherwise clean, SHALL remove the worktree directory (including its symlinks) and its administrative entry, and SHALL NOT remove the shared `../.target` directory, which remains a cross-worktree cache reclaimable with `cargo clean`.

#### Scenario: Removing a wired worktree needs no force

- **WHEN** `git worktree remove` runs on a clean, hook-wired linked worktree
- **THEN** it succeeds without `--force` and leaves no worktree directory and no stale worktree administrative entry

#### Scenario: The shared target survives removal

- **WHEN** a linked worktree is removed
- **THEN** `../.target` still exists for the remaining worktrees

### Requirement: DEVELOPING.md documents the worktree workflow

`DEVELOPING.md` SHALL contain a worktree section that documents: the one-time `git config core.hooksPath .githooks` opt-in (and that it replaces the clone's default `.git/hooks`), worktree directory naming `../kooka-pictura-worktrees/{issue}-{name}` with branch names following the AGENTS.md `type/issue-number-kebab-description` convention (referenced, not restated), the manual cmake configure command used elsewhere in the file, pointers to the existing sccache guidance and the one-time `serena project index` for a new worktree, the shared-`target` caveat that cargo builds from one worktree at a time, and cleanup with `git worktree remove`.

#### Scenario: A contributor sets up a worktree from the docs

- **WHEN** a contributor follows the DEVELOPING.md worktree section on a fresh clone
- **THEN** they find the hooksPath command, the naming convention, the cmake configure line, the sccache and serena pointers, the concurrency caveat, and the removal command
