# Spec Delta

## ADDED Requirements

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
