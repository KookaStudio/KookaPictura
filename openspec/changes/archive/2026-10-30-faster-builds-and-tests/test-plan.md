# Test Plan

This change is build and CI configuration. Most scenarios are proven by reading
generated build commands, Ninja rebuild lists, or a CI run, so their Coverage
cell is a manual check with the exact command. Every manual check runs in task
group 8 (after the single compile) or 9 (the PR's CI run). Three invariants the build
relies on are pinned by Rust unit tests that read the manifests and the bridge
sources. Scenarios carried unchanged inside a MODIFIED block are listed so the
plan is complete; the existing runs keep exercising them. The manual rows are
the reviewer checklist in the PR.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `bp_dev_profile` | Configure a fresh build dir with defaults (Ninja, RelWithDebInfo) and no `CARGO_INCREMENTAL`; `ninja -t commands` for the cargo step; build | The cargo command carries `--profile=dev-app`; `build/cargo/build/*/dev-app/incremental` exists after the build. In CI, `CARGO_INCREMENTAL=0` keeps `qt-headless` non-incremental | manual: read the cargo command from `ninja -t commands` and list the incremental directory (task 8.3) |
| `bp_release_profile` | Configure with `-DBUILD_TESTING=OFF`; read the cargo command | The cargo command carries `--release` | manual: `ninja -t commands` in a throwaway `-DBUILD_TESTING=OFF` build dir (task 8.3) |
| `bp_debug_profile` | Configure with `-DCMAKE_BUILD_TYPE=Debug`; read the cargo command | The cargo command carries neither `--release` nor `--profile=dev-app`, so cargo uses `dev` | manual: `ninja -t commands` in a throwaway Debug build dir (task 8.3) |
| `bp_dev_profile_semantics` | Run the unit test that reads the workspace manifest | `dev-app` inherits `release` and sets neither `overflow-checks` nor `debug-assertions` in any of its sections | `pictura-app::dev_app_profile_keeps_release_semantics` |
| `bp_mold` | Configure a fresh build dir on a host with mold; read a link command | The link command carries `-fuse-ld=mold` and the configure prints no linker warning | manual: `ninja -t commands pictura` and search the link line (task 8.7) |
| `bp_mold_fallback` | Configure with mold hidden from `PATH` | Configure succeeds; link uses lld | manual: `PATH` without mold, fresh build dir, read the link line |
| `bp_mold_override` | Configure with `-DCMAKE_EXE_LINKER_FLAGS=-fuse-ld=lld` | lld is used, mold is not substituted | manual: read the link line |
| `bp_mold_masked_warning` | Configure a throwaway dir with `-DCMAKE_EXE_LINKER_FLAGS=-fuse-ld=lld` on a host with mold, then reconfigure it | The first configure prints a warning naming the cached flag and `cmake -U CMAKE_EXE_LINKER_FLAGS -U CMAKE_SHARED_LINKER_FLAGS`; the second prints none; lld stays in both | manual: read both configure outputs and the link line (task 8.7) |
| `bp_mold_platform` | Inspect the macOS and Windows packaging configures | No `-fuse-ld` flag is added | manual: read `.github/actions/macos-bundle/action.yml` and `windows-bundle/action.yml`; the probe is unchanged for these platforms |
| `bp_bridge_same_build` | Add a method to a bridge module; build once, then again | The first build recompiles the C++ that includes the generated header; the second recompiles nothing | manual: `ninja -d explain` or `build/.ninja_log` for both builds (task 8.6) |
| `bp_bridge_no_change` | Build twice with no change | No C++ translation unit is recompiled and the configure step does not re-run | manual: read the Ninja rebuild list and `ninja -d explain` for the second build (task 8.6) |
| `bp_bridge_impl_edit` | Edit a function body outside the bridge module of `cxxqt_object/paint_tools.rs`; build | `build/pictura_bridge.stamp` is unchanged and no C++ object is recompiled | manual: `sha256sum build/pictura_bridge.stamp` before and after, and the `.ninja_log` diff (task 8.6) |
| `bp_bridge_stamp_fallback` | Temporarily add a raw string with a column-0 `}` line inside a bridge module; configure; edit a line after the string; configure again; revert. Repeat with a block comment holding a column-0 `}` line (or run the stamp function on such a file with `cmake -P`) | The stamp changes on the edit after the string or comment, so the file was hashed whole | manual: a configure-time CMake check with no test harness to host it; compare `build/pictura_bridge.stamp` across the two configures (task 8.6). The unit test `bridge_modules_end_at_column_zero` guards the shortcut's invariant on the real sources |
| Push or pull request | — | — | manual: unchanged by this change; carried in the MODIFIED block, and the existing CI/local run exercises it |
| A feature-branch push is covered once | — | — | manual: unchanged by this change; carried in the MODIFIED block, and the existing CI/local run exercises it |
| Tests run under nextest | Read the `test` job in `ci.yml` | nextest runs; no `cargo test --doc` step exists | manual: read `.github/workflows/ci.yml` and the job log of the PR run |
| Lint and test run in parallel | — | — | manual: unchanged by this change; carried in the MODIFIED block, and the existing CI/local run exercises it |
| The local fast gate uses nextest | Run `scripts/verify-fast.sh` | nextest runs; no doctest run follows | manual: read the gate's output |
| The JUnit artifact is uploaded | — | — | manual: unchanged by this change; carried in the MODIFIED block, and the existing CI/local run exercises it |
| The Qt app self-tests headlessly | — | — | manual: unchanged by this change; carried in the MODIFIED block, and the existing CI/local run exercises it |
| External oracles are exercised | Read the PR's `test` job log | `magick -version` succeeds, psd-tools and the pinned ag-psd install, and the oracle tests (including `agpsd_oracle`) report passed rather than skipped; no `oracles` job runs | manual: needs a CI run; read the `test` job log and the nextest JUnit for the oracle tests (task 9.2) |
| `vh_no_doctests` | Run the unit test that reads every workspace member's manifest, `ci.yml` and `scripts/test-report.sh` | Every workspace library sets `doctest = false`; neither file runs `cargo test` with `--doc` | `pictura-app::every_workspace_library_disables_doctests` |
| Compiled artifacts are reused | — | — | manual: unchanged by this change; carried in the MODIFIED block, and the existing CI/local run exercises it |
| A runaway job is bounded | — | — | manual: unchanged by this change; carried in the MODIFIED block, and the existing CI/local run exercises it |
| `vh_ci_ctest_parallel` | — | — | manual: unchanged by this change; carried in the MODIFIED block, and the existing CI/local run exercises it |
| `vh_ci_mold` | — | — | manual: unchanged by this change; carried in the MODIFIED block, and the existing CI/local run exercises it |
| Docs-only change | — | — | manual: unchanged by this change; carried in the MODIFIED block, and the existing CI/local run exercises it |
| Code change | — | — | manual: unchanged by this change; carried in the MODIFIED block, and the existing CI/local run exercises it |
| Guards fail | — | — | manual: unchanged by this change; carried in the MODIFIED block, and the existing CI/local run exercises it |
| The fast full gate runs the worktree-hook check | — | — | manual: unchanged by this change; carried in the MODIFIED block, and the existing CI/local run exercises it |
| The guards workflow runs the archive check | — | — | manual: unchanged by this change; carried in the MODIFIED block, and the existing CI/local run exercises it |
| C++-only change | Read the changed-path gating in `ci.yml` after the edit | `qt-headless` runs; `lint` and `test` are skipped; no `oracles` job is referenced | manual: read the `changes` job and the `if:` conditions in `ci.yml` |
| Rust change runs everything | The PR itself changes Rust and CMake files | `lint`, `test`, and `qt-headless` all run | manual: the PR's own check list |
| Mixed change runs everything | — | — | manual: unchanged by this change; carried in the MODIFIED block, and the existing CI/local run exercises it |
| Unrecognized path runs everything | — | — | manual: unchanged by this change; carried in the MODIFIED block, and the existing CI/local run exercises it |
| Workflow edit re-runs the suite | The PR itself edits `ci.yml` | Every build job runs | manual: the PR's own check list |
| A skipped job still reports | — | — | manual: unchanged by this change; carried in the MODIFIED block, and the existing CI/local run exercises it |
| Every layer is summarised | Run `scripts/test-report.sh never` with a built tree | The report lists the Rust suites, the self-test and Qt Test subtotals, and a TOTAL, with no doctest line; exit 0 | manual: read the report output |
| A failure is listed and fails the run | — | — | manual: unchanged by this change; carried in the MODIFIED block, and the existing CI/local run exercises it |
| Colour output is selectable | — | — | manual: unchanged by this change; carried in the MODIFIED block, and the existing CI/local run exercises it |
| The Qt Test layer is folded in | — | — | manual: unchanged by this change; carried in the MODIFIED block, and the existing CI/local run exercises it |
| `tr_qt_parallel` | — | — | manual: unchanged by this change; carried in the MODIFIED block, and the existing CI/local run exercises it |
| `tr_stale_junit` | — | — | manual: unchanged by this change; carried in the MODIFIED block, and the existing CI/local run exercises it |
| Configure builds the suites | — | — | manual: unchanged by this change; carried in the MODIFIED block, and the existing CI/local run exercises it |
| CTest runs each suite offscreen | — | — | manual: unchanged by this change; carried in the MODIFIED block, and the existing CI/local run exercises it |
| CTest emits per-executable JUnit | — | — | manual: unchanged by this change; carried in the MODIFIED block, and the existing CI/local run exercises it |
| Tests are optional | — | — | manual: unchanged by this change; carried in the MODIFIED block, and the existing CI/local run exercises it |
| CTest does not wrap nextest | — | — | manual: unchanged by this change; carried in the MODIFIED block, and the existing CI/local run exercises it |
| The job exercises the GPU | — | — | manual: unchanged by this change; carried in the MODIFIED block, and the existing CI/local run exercises it |
| A broken driver setup fails the job | — | — | manual: unchanged by this change; carried in the MODIFIED block, and the existing CI/local run exercises it |
| `gpu_clippy_in_lint` | Read the PR's `lint` and `gpu-parity` job logs | `lint` runs the `software-vulkan` clippy; `gpu-parity` has no clippy step | manual: needs a CI run; read `ci.yml` and the job logs of the PR run (task 9.2) |
