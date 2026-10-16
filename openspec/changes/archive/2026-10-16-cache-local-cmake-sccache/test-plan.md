# Test Plan

Covers every scenario in this change's specs. `scripts/check-local-build-caching.sh`
is a self-skipping shell check (it runs a real CMake configure, so it needs Qt
and is wired into `verify-fast.sh` and the guards workflow, skipping where Qt is
absent). `scripts/check-test-plan.py` cannot name a shell script as a coverage
token, so its rows read `manual:` naming the script and the assertion.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `lbc_default` | Run `scripts/check-local-build-caching.sh` case 1: fresh configure, no launcher override, `sccache` on `PATH` | The generated `build.ninja` carries `LAUNCHER = …sccache` | manual: scripts/check-local-build-caching.sh case 1 greps the Ninja LAUNCHER rule |
| `lbc_override` | Script case 2: configure with `-DCMAKE_CXX_COMPILER_LAUNCHER=ccache` while `sccache` is on `PATH` | The Ninja `LAUNCHER` rule names `ccache`, not `sccache` | manual: scripts/check-local-build-caching.sh case 2 greps the override |
| `lbc_absent` | Script case 3: configure with `sccache` removed from `PATH` | Configure succeeds; no `LAUNCHER` rule and no `-NOTFOUND` text is generated | manual: scripts/check-local-build-caching.sh case 3 |
| `lbc_rebuild_hits` | Script `--rebuild` (run by `verify-full.sh`): build, record `sccache --show-stats`, remove objects, rebuild | Cache hit count increases; rebuild reuses cached objects | manual: scripts/check-local-build-caching.sh --rebuild rebuilds twice and reads sccache --show-stats |
| `lbc_docs` | Read the "Speeding up local builds" section of `DEVELOPING.md` | Names `~/.cache/sccache`, the 10 GiB default, `SCCACHE_DIR`/`SCCACHE_CACHE_SIZE`, and `SCCACHE_IGNORE_SERVER_IO_ERROR` | manual: docs review of the documented cache variables |
| Docs-only working diff | Run `scripts/verify-fast.sh` with only `docs/`, `*.md`, `openspec/`, or `.serena/` changes | Guard + OpenSpec validation run; fmt, clippy, tests skipped | manual: scripts/verify-fast.sh unchanged behaviour |
| Clean tree or code change | Run `scripts/verify-fast.sh` with a code change | Full gate runs | manual: scripts/verify-fast.sh unchanged behaviour |
