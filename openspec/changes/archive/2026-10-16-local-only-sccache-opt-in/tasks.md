# Tasks

## 1. Hook test first

- [x] 1.1 Update `scripts/check-worktree-hook.sh`: opt in through the sandbox `HOME`'s user-global env file (`$HOME/.config/kooka-pictura/env`) and through the process env; replace the `git config kooka.sccache-share` cases; add a case where a repo-local env file sets the opt-in but the user-global file and process env do not (must stay off). Verify `bash scripts/check-worktree-hook.sh` passes.

## 2. Hook implementation

- [x] 2.1 Change `sccache_share_enabled` in `.githooks/post-checkout` to read `KOOKA_SCCACHE_SHARE` from the process env or `${KOOKA_ENV_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/kooka-pictura/env}` (targeted parse, no sourcing) and drop the `git config kooka.sccache-share` path; verify the opt-on, env-file, opt-off, and not-tracked cases pass.

## 3. Local-only opt-in sources

- [x] 3.1 Add a committed `.env.example` at the repo root listing the project env vars (`KOOKA_SCCACHE_SHARE`, `KOOKA_ENV_FILE`) with all values commented and a header explaining `mkdir -p ~/.config/kooka-pictura && cp .env.example ~/.config/kooka-pictura/env`; verify it enables nothing when copied as-is.
- [x] 3.2 Add `.env`, `.env.*`, `!.env.example`, and `.envrc` to `.gitignore`; verify `git check-ignore .env .envrc` reports them ignored and `git ls-files .env.example` reports it tracked.

## 4. Documentation

- [x] 4.1 Update `DEVELOPING.md` (local-build caching section) and `AGENTS.md`: the opt-in is `KOOKA_SCCACHE_SHARE=1` sourced from the user-global env file created from `.env.example`, the path, and that no committed file can enable it; verify the commands run as written.

## 5. Integration

- [x] 5.1 Run `bash scripts/check-worktree-hook.sh`, `python3 scripts/check-test-plan.py`, and `openspec validate --all --strict`; verify all pass, then archive this change and confirm `scripts/check-openspec-archived.sh` is green.
