# Test Plan

Covers every scenario in the `meta/worktree-hooks` delta. `scripts/check-worktree-hook.sh`
is a shell check; `check-test-plan.py` cannot name a shell script as a coverage
token, so its rows read `manual:` naming the script case and the assertion.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `wtcache_off` | `scripts/check-worktree-hook.sh`: create a worktree with a sandbox `HOME` and no opt-in anywhere | No `sccache` config is created; hook exits 0 | manual: scripts/check-worktree-hook.sh opt-off case |
| `wtcache_on` | Same script, `KOOKA_SCCACHE_SHARE=1` in the process env, stubbed `sccache` on `PATH` | Managed config lists both worktree roots and sets `hash_working_directory = false` | manual: scripts/check-worktree-hook.sh opt-on case |
| `wtcache_envfile` | Same script, process env unset, sandbox `HOME/.config/kooka-pictura/env` sets `KOOKA_SCCACHE_SHARE=1` | Hook treats the opt-in as enabled and writes the config | manual: scripts/check-worktree-hook.sh env-file case |
| `wtcache_notracked` | Same script, a repo-local env file exports the opt-in but the user-global file and process env do not | No config is written | manual: scripts/check-worktree-hook.sh not-tracked case |
| `wtcache_noclobber` | Same script, pre-seed a sandbox sccache config without the sentinel, opt-in set | The pre-seeded file is byte-identical afterwards | manual: scripts/check-worktree-hook.sh no-clobber case |
| `wtcache_noop` | Same script, run a second checkout after the config exists | Config unchanged and the stubbed `sccache --stop-server` is not called again | manual: scripts/check-worktree-hook.sh no-op case |
| `wtcache_absent` | Same script with `sccache` removed from `PATH`, opt-in set | Worktree add succeeds and no config is written | manual: scripts/check-worktree-hook.sh absent case |
| `wtcache_docs` | Read `DEVELOPING.md` and `AGENTS.md` | Opt-in variable, user-global file path, `cp .env.example` step, opt-out, bounds, and caveats are present | manual: docs review |
| `env_example` | `git ls-files .env.example` and `git check-ignore .env .envrc` | `.env.example` is tracked and a no-op; `.env`/`.envrc` are ignored | manual: git ls-files / check-ignore review |
