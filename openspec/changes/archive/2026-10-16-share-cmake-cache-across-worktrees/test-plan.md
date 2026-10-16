# Test Plan

Covers every scenario in the `meta/worktree-hooks` delta. `scripts/check-worktree-hook.sh`
is a shell check; `check-test-plan.py` cannot name a shell script as a coverage
token, so its rows read `manual:` naming the script case and the assertion.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `wtcache_off` | `scripts/check-worktree-hook.sh`: create a worktree with no opt-in and a sandbox `HOME` | No `sccache` config is created under the sandbox `HOME` | manual: scripts/check-worktree-hook.sh opt-off case |
| `wtcache_on` | Same script, opt-in set (`git config kooka.sccache-share true`), `sccache` stubbed on `PATH` | The managed config exists, lists both worktree roots in `basedirs`, and sets `hash_working_directory = false` | manual: scripts/check-worktree-hook.sh opt-on case |
| `wtcache_noclobber` | Same script, pre-seed a sandbox config without the sentinel, opt-in set | The pre-seeded file is byte-identical afterwards | manual: scripts/check-worktree-hook.sh no-clobber case |
| `wtcache_noop` | Same script, run a second checkout after the config exists | Config unchanged and the stubbed `sccache --stop-server` is not called again | manual: scripts/check-worktree-hook.sh no-op case |
| `wtcache_absent` | Same script with `sccache` removed from `PATH`, opt-in set | Worktree add succeeds and no config is written | manual: scripts/check-worktree-hook.sh absent case |
| `wtcache_docs` | Read the local-build caching guidance in `DEVELOPING.md` and the pointer in `AGENTS.md` | Opt-in command, what is written, opt-out, cache bound, and same-source caveat are present; AGENTS.md points at the opt-in | manual: docs review |
