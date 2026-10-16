# Proposal

## Why

The worktree cache-sharing opt-in is currently `git config kooka.sccache-share
true` or the environment variable `KOOKA_SCCACHE_SHARE=1`. The environment
variable is a good local-only lever, but there is no committed place that
documents which environment variables this project uses, and nothing stops a
future contributor from dropping a repo-local `.env`/`.envrc` that exports the
opt-in — which would then enable it for everyone who checks the file out. The
opt-in should be sourced from a file that cannot be committed, with a committed
example that tracks the project's variables.

## What Changes

- The opt-in becomes **`KOOKA_SCCACHE_SHARE=1` from the process environment or a
  user-global env file** `${XDG_CONFIG_HOME:-$HOME/.config}/kooka-pictura/env`
  (overridable by `KOOKA_ENV_FILE`). The `git config kooka.sccache-share` path is
  removed — one local-only mechanism. The user-global file still reaches every
  worktree, so the cascade is preserved.
- A committed `.env.example` documents every environment variable the project
  uses, with all values commented out so it enables nothing.
- `.gitignore` ignores local env files (`.env`, `.env.*`, `.envrc`) while keeping
  `.env.example`.
- `AGENTS.md` / `DEVELOPING.md` point at the example and the user-global path.
- `scripts/check-worktree-hook.sh` exercises the opt-in through the sandbox's
  user-global env file and the process environment.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `meta/worktree-hooks`: the cache-sharing opt-in is re-specified as the
  user-global env file / process env, and the documentation requirement covers
  the committed example.

## Impact

- `.githooks/post-checkout`: opt-in source changes; no other behaviour.
- `.env.example` (new), `.gitignore`, `AGENTS.md`, `DEVELOPING.md`,
  `scripts/check-worktree-hook.sh`.
- No new dependencies; default remains off. The user-global env file is outside
  the repository and cannot be committed.
