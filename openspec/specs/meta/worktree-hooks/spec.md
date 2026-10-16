# meta/worktree-hooks Specification

## Purpose
Defines how a linked git worktree is discovered and wired automatically when it
appears: the committed post-checkout hook, the symlinks it creates, the one-time
per-clone opt-in, and the DEVELOPING.md workflow documentation that surrounds it.

## Requirements

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

### Requirement: The worktree hook can share the C++ compiler cache across worktrees

The post-checkout hook SHALL support an explicit, off-by-default opt-in that
makes local C++ compilation share one `sccache` cache across every worktree of
the repository. The opt-in SHALL be enabled by `git config kooka.sccache-share
true` or the environment variable `KOOKA_SCCACHE_SHARE=1`, and SHALL cascade to
new worktrees because the git configuration lives in the shared common config
that every worktree reads.

When opted in and `sccache` is available, the hook SHALL write a managed
`sccache` configuration that lists every current worktree root as a `basedirs`
entry (sourced from `git worktree list`) and disables
`hash_working_directory` in the preprocessor cache mode, so an object compiled
at one worktree path is reused at another. The hook SHALL only act on a
configuration file it owns, identified by a sentinel line; if the target file
exists without that sentinel the hook SHALL leave it untouched and report the
skip on stderr. The hook SHALL rewrite the file and restart the `sccache` daemon
only when the generated content differs from what is on disk, so a branch switch
inside an already-wired worktree remains a silent no-op. When the opt-in is
absent or `sccache` is not installed, the step SHALL do nothing and SHALL NOT
fail the checkout. A failure in this optional step SHALL be reported on stderr
and SHALL still exit 0.

#### Scenario: opt-in disabled writes nothing [wtcache_off]

- **WHEN** a worktree is created or checked out and neither the `kooka.sccache-share` git config nor `KOOKA_SCCACHE_SHARE` is set
- **THEN** no `sccache` configuration is written and the hook exits 0

#### Scenario: opt-in enabled shares all worktree roots [wtcache_on]

- **WHEN** the opt-in is set and `sccache` is on `PATH` and a worktree is created
- **THEN** the managed `sccache` configuration lists every worktree root in `basedirs` and sets `hash_working_directory = false`

#### Scenario: an unmanaged config is left alone [wtcache_noclobber]

- **WHEN** the opt-in is set but the `sccache` configuration file already exists, is non-empty, and lacks this repository's managed sentinel (a user's own config, or another clone's)
- **THEN** the hook leaves the file unchanged, notes the skip on stderr, and exits 0

#### Scenario: an unchanged config is a no-op [wtcache_noop]

- **WHEN** the hook runs again with the same set of worktree roots and the managed configuration already matches
- **THEN** it does not rewrite the file and does not restart the `sccache` daemon

#### Scenario: sccache absent is tolerated [wtcache_absent]

- **WHEN** the opt-in is set but `sccache` is not on `PATH`
- **THEN** the cache-sharing step is skipped without failing the checkout

### Requirement: Worktree cache sharing is documented

`DEVELOPING.md` SHALL document the cross-worktree compiler-cache sharing: the
opt-in command (and that it is per-clone through the shared git config), what
file the hook writes, how to opt out (delete the managed file and stop the
daemon), the on-disk cache bound, that the config file and the daemon restart
are machine-global (affecting other projects and concurrent builds, with the
`SCCACHE_IGNORE_SERVER_IO_ERROR` escape hatch), and the limitation that only
identical source and flags hit while Rust uses the shared `target` tree instead.
`AGENTS.md` SHALL carry a short pointer to the opt-in so agents creating
worktrees know it exists.

#### Scenario: A contributor finds the opt-in and its limits [wtcache_docs]

- **WHEN** a contributor reads the local-build caching guidance
- **THEN** they find the opt-in command, what the hook writes, how to opt out, the cache bound, the machine-global config and daemon-restart caveats, and the same-source caveat, and `AGENTS.md` points at the opt-in
