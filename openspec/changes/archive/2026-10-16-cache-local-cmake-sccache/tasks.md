# Tasks

## 1. Verification check

- [x] 1.1 Create `scripts/check-local-build-caching.sh`: three throwaway configures (default with `sccache` on `PATH`, `-DCMAKE_CXX_COMPILER_LAUNCHER=ccache` override, `PATH` without `sccache`), each asserting the generated `build.ninja` `LAUNCHER` rule (present/`sccache`, `ccache`, or absent with no `-NOTFOUND`); save and restore the repo-root `compile_commands.json` symlink via a `trap`; self-skip when `cmake` or `qmake6` is absent. Verify `bash -n scripts/check-local-build-caching.sh` passes and it prints a skip on a `PATH` lacking `cmake`.
- [x] 1.2 Add a `--rebuild` mode to the script: build once, record `sccache --show-stats`, delete the build outputs, rebuild, and assert the hit count increased. Verify it reports a non-zero hit delta on a checkout with `sccache`.

## 2. CMake default launcher

- [x] 2.1 Add the guarded block to `CMakeLists.txt` immediately after `project()`: `if(NOT CMAKE_CXX_COMPILER_LAUNCHER)` → `find_program(SCCACHE_EXECUTABLE sccache)` → nested `if(SCCACHE_EXECUTABLE)` → `set(CMAKE_CXX_COMPILER_LAUNCHER "${SCCACHE_EXECUTABLE}")`. Verify `scripts/check-local-build-caching.sh` case 1 finds `sccache` in the Ninja `LAUNCHER` rule.
- [x] 2.2 Confirm the override and absent cases pass: an explicit `-D` yields a `ccache` `LAUNCHER` rule, and with `sccache` off `PATH` the configure succeeds with no `LAUNCHER` rule and no `SCCACHE_EXECUTABLE-NOTFOUND` text.

## 3. Verify script simplification

- [x] 3.1 Remove the conditional `-DCMAKE_CXX_COMPILER_LAUNCHER=sccache` block from `scripts/verify-full.sh`; verify `bash -n scripts/verify-full.sh` passes and the configure line no longer names a launcher.

## 4. Gate the check

- [x] 4.1 Add `bash scripts/check-local-build-caching.sh` to the full gate of `scripts/verify-fast.sh` (self-skips without Qt); verify `bash -n scripts/verify-fast.sh` passes and the docs-only fast path still exits before it.
- [x] 4.2 Add a `Local build caching` step running the script to `.github/workflows/guards.yml`; verify the workflow YAML parses and the step self-skips on the Qt-less runner.
- [x] 4.3 Add `bash scripts/check-local-build-caching.sh --rebuild` to `scripts/verify-full.sh` after the build; verify it runs end to end and reports the cache-hit delta.

## 5. Documentation

- [x] 5.1 Update the "Speeding up local builds" section of `DEVELOPING.md`: a plain configure now picks up `sccache`, default cache dir `~/.cache/sccache` with a 10 GiB cap, `SCCACHE_DIR`/`SCCACHE_CACHE_SIZE` overrides, and `SCCACHE_IGNORE_SERVER_IO_ERROR=1`; verify every documented variable name and the default path match `sccache --show-stats` output.

## 6. Integration

- [x] 6.1 Run `scripts/check-local-build-caching.sh` and `--rebuild` end to end (all cases pass), then `python3 scripts/check-test-plan.py` and `openspec validate --all --strict`; verify the change validates clean and every check reports a pass.
