# Tasks

## 1. Hook test first

- [x] 1.1 Extend `scripts/check-worktree-hook.sh`: set `HOME` to a sandbox, stub an `sccache` executable on `PATH` that logs its arguments, and add cases for opt-off (no config), opt-on (config lists both roots + `hash_working_directory = false`), no-clobber (pre-seeded unmarked config untouched), no-op (second checkout does not re-run `--stop-server`), and sccache-absent (no config, checkout succeeds). Verify `bash scripts/check-worktree-hook.sh` passes.

## 2. Hook implementation

- [x] 2.1 Add the opt-in cache-sharing step to `.githooks/post-checkout`: skip unless `git config kooka.sccache-share` is `true` or `KOOKA_SCCACHE_SHARE=1`, and unless `sccache` is on `PATH`; verify the opt-off and sccache-absent cases from 1.1 pass.
- [x] 2.2 Generate the managed config (sentinel first line, `basedirs` from `git worktree list`, `[cache.disk.preprocessor_cache_mode] hash_working_directory = false`) at `SCCACHE_CONF` or the platform default, write only on change, and `sccache --stop-server` only then; verify the opt-on and no-op cases pass.
- [x] 2.3 Refuse to touch a config without the sentinel (stderr note, exit 0); verify the no-clobber case passes.

## 3. Documentation

- [x] 3.1 Add ~1–3 lines to `AGENTS.md` pointing at the opt-in (`git config kooka.sccache-share true`) and to `DEVELOPING.md`; verify the pointer names the exact command.
- [x] 3.2 Extend the local-build caching guidance in `DEVELOPING.md`: the opt-in command and that it is per-clone via the shared git config, what file the hook writes, how to opt out (delete the file and `sccache --stop-server`), the 10 GiB cache bound, and the identical-source caveat; verify every command and path matches what the hook does.

## 4. Integration

- [x] 4.1 Run `bash scripts/check-worktree-hook.sh`, `python3 scripts/check-test-plan.py`, and `openspec validate --all --strict`; verify all pass.
- [ ] 4.2 Manual end-to-end: create two linked worktrees from one commit with the opt-in set, build the same target in each, and confirm `sccache --show-stats` reports hits on the second. Deferred — the effectiveness spike already proved `basedirs` yields a cross-directory hit, and 4.1 proves the hook writes the config.
