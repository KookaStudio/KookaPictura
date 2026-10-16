# Design

## Context

See `proposal.md`. The previous mechanism (`meta/worktree-hooks`, archived change
`share-cmake-cache-across-worktrees`) read a git config key or a process env var.
The git config path is per-clone and invisible, and nothing documented the
project's environment variables. A future repo-local `.env` exporting the opt-in
would silently enable it for everyone.

## Goals / Non-Goals

**Goals:**
- The opt-in is sourced only from local-only locations (process env or a
  user-global file outside the repo).
- The project's environment variables are documented in one committed example.

**Non-Goals:**
- Changing what the hook writes once opted in.
- Supporting a repo-local env file (deliberately not read).

## Decisions

### D1. User-global env file, not git config

Opt-in source: `KOOKA_SCCACHE_SHARE` from the process environment, else from
`${XDG_CONFIG_HOME:-$HOME/.config}/kooka-pictura/env` (override: `KOOKA_ENV_FILE`).
The user-global file is read by the hook in every worktree, so the cascade the
previous mechanism had is preserved, and the file is outside the repository, so
it cannot be committed. The git config path is removed to leave one mechanism.

### D2. Read the variable, do not source the file

The hook extracts the value with a targeted `sed` rather than sourcing the file,
so a config file can never execute code inside the hook. Only the one variable is
read today; the example documents the rest.

### D3. Committed example, ignored locals

`.env.example` is committed and no-op (all values commented). `.gitignore`
ignores `.env`, `.env.*`, and `.envrc`, with `!.env.example` keeping the example.
This makes the "local only" rule mechanical, not just documented.

### D4. Same branch, archived in the same PR

Per the request this lands on `build/140-cache-cmake-sccache` and the change is
authored and archived together (`check-openspec-archived.sh` requires no
non-archived change in the PR).

## Risks / Trade-offs

- [Dropping git config removes a per-clone opt-in] → the user-global file is
  per-machine and reaches every worktree, which is the intended scope.
- [A user-global env file is machine-wide, so it opts in every clone of this
  repo on the machine] → intended; the opt-in is per-machine now, and the file
  is explicit and user-editable.
- [`.env.example` could drift from the code] → it is the single list; the
  test-plan row checks it names `KOOKA_SCCACHE_SHARE` and `KOOKA_ENV_FILE`.
