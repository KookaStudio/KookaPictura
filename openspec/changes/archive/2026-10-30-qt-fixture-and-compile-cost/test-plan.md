# Test Plan

Most of this change is build, CI and test-harness work. Those scenarios are
proven by a timing, a Ninja rebuild list or a CI run. Their Coverage cell is a
manual check that names the exact command, run in task group 12 after the
single compile. The rules that can be pinned in code have automated tests:

- the theme rule, by the new Qt Test suite `tst_theme_apply`;
- the first-run layout, by a new `tst_workspaces` case;
- argv handling of the sharded test main, by the new suite `tst_test_support`;
- the declarations-only bridge rule and the fixed-delay rule, by Rust tests in
  `build_config.rs` that read the sources.

The scenarios carried unchanged inside the MODIFIED blocks are listed so the
plan is complete; the existing runs keep exercising them. The manual rows are
the reviewer checklist in the PR.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `las_theme_idempotent` | Construct a window, then a second one with the same session | Neither `Theme::paletteGeneration()` nor the `QApplication::style()` pointer changes for the second window | `tst_theme_apply::secondWindowKeepsTheme` |
| `las_theme_change_applies` | From level 1, step the brightness up on a shown window | The generation advances and the palette changes | `tst_theme_apply::brightnessStepApplies` |
| `las_theme_before_chrome` | Start from the dark theme, construct from a light-family, non-default-level session with an app event filter that snapshots the app style sheet when the layers panel and the options bar first take a child | Both snapshots equal the light theme's style sheet; the finished style sheet and palette equal a fresh application with those settings | `tst_theme_apply::chromeBuiltUnderRestoredTheme` |
| `las_theme_live_state` | Apply a theme, clear `qApp`'s style sheet, apply the same theme | The theme's style sheet is installed again | `tst_theme_apply::reapplyAfterExternalStyleSheet` |
| `las_theme_chrome_fonts` | Construct a window | The tab bar and status bar keep the fonts set when they were built | `tst_theme_apply::chromeFontsSurvive` |
| `wsp_first_run_layout` | Construct from an empty session, record the arrangement, apply Essentials | The arrangement is unchanged | `tst_workspaces::freshLayoutMatchesEssentialsApply` |
| Every layer is summarised | — | — | manual: unchanged by this change; carried in the MODIFIED block, and the existing local run exercises it |
| A failure is listed and fails the run | — | — | manual: unchanged by this change; carried in the MODIFIED block, and the existing local run exercises it |
| Colour output is selectable | — | — | manual: unchanged by this change; carried in the MODIFIED block, and the existing local run exercises it |
| The Qt Test layer is folded in | — | — | manual: unchanged by this change; carried in the MODIFIED block, and the existing local run exercises it |
| `tr_qt_parallel` | Run `bash scripts/test-report.sh` on the 16-CPU reference machine, without an override | The header reads `== qt tests (ctest -j 12) ==` and each suite writes its JUnit file | manual: the level comes from the host's CPU count; read the header line and `ls build/qt-test-results` (task 12.2) |
| `tr_qt_parallel_override` | Run with `CTEST_PARALLEL_LEVEL=2` | The header reads `-j 2` | manual: an end-to-end run of the shell script; read the header line (task 4.1) |
| `tr_qt_parallel_floor` | Run under `taskset -c 0` without an override | The header reads `-j 1` | manual: needs a restricted CPU set (task 4.1) |
| `tr_stale_junit` | — | — | manual: unchanged by this change; carried in the MODIFIED block, and the existing local run exercises it |
| Push or pull request | — | — | manual: unchanged; carried in the MODIFIED verification-harness block, exercised by every CI run |
| A feature-branch push is covered once | — | — | manual: unchanged; carried in the MODIFIED block, exercised by every CI run |
| Tests run under nextest | — | — | manual: unchanged; carried in the MODIFIED block, exercised by every CI run |
| Lint and test run in parallel | — | — | manual: unchanged; carried in the MODIFIED block, exercised by every CI run |
| The local fast gate uses nextest | — | — | manual: unchanged; carried in the MODIFIED block, exercised by `verify-fast.sh` |
| The JUnit artifact is uploaded | — | — | manual: unchanged; carried in the MODIFIED block, exercised by every CI run |
| The Qt app self-tests headlessly | — | — | manual: unchanged; carried in the MODIFIED block, exercised by every CI run |
| External oracles are exercised | — | — | manual: unchanged; carried in the MODIFIED block, exercised by every CI run |
| `vh_no_doctests` | — | — | manual: unchanged; carried in the MODIFIED block, pinned by the existing `pictura-app::every_workspace_library_disables_doctests` |
| Compiled artifacts are reused | Read a CI run that follows a `master` run | Registry, target and compiler caches are restored | manual: needs CI runs (task 11.1) |
| A runaway job is bounded | — | — | manual: unchanged; carried in the MODIFIED block, every job keeps `timeout-minutes` |
| `vh_ci_ctest_parallel` | Read the PR's `qt-headless` CTest step | It prints `ctest -j <nproc>` (4 or more on a hosted runner), and every suite's JUnit is folded | manual: needs a CI run (task 4.2) |
| `vh_ci_no_pch` | Read the PR's `qt-headless` configure step | It passes `-DCMAKE_DISABLE_PRECOMPILE_HEADERS=ON` and the build succeeds | manual: needs a CI run (task 10.4) |
| `vh_ci_mold` | — | — | manual: unchanged; carried in the MODIFIED block, exercised by every CI run |
| `qth_condition_wait` | Run the Rust test that scans the Qt Test sources | Every `qWait` with a non-zero argument has a comment on its line or the line above | `pictura-app::qt_test_waits_are_justified` |
| `qth_negative_wait` | Read each remaining commented delay | Each is a negative window with its comment | manual: a review judgement on what each wait proves; the comment itself is enforced by `pictura-app::qt_test_waits_are_justified` |
| `qth_named_functions` | Build the argument list for argv `{exe, "foo"}` | Only `foo` is passed to QTest; an option value such as `-o` or `-perfcounter` is not a name; an empty share is caught instead of running every function | `tst_test_support::namedFunctionsAreKept`, `tst_test_support::optionValuesAreNotFunctions`, `tst_test_support::perfcounterValueIsNotAFunction`, `tst_test_support::shardAppendsItsShare`, `tst_test_support::emptyShareRunsNothing` |
| `qth_shared_window_isolated` | For each shared-window suite (`tst_document_float`, `tst_document_windows`, `tst_document_arrange`, `tst_import_pdf`, `tst_edit_refresh`, `tst_loading_tab`; also `tst_print`), run `build/crates/pictura-app/cpp/tests/tst_x <fn>` for every function, with no shard variable | Every function passes alone, and the JUnit lists only that function | manual: one process per function across those suites (tasks 7.3, 7.4) |
| `qth_shared_window_shards` | For each suite of the row above, run `PICTURA_TEST_SHARD=k/4` for k = 0..3 | Every entry passes, and together they run every function once | manual: compare the function names in the four JUnit files with the suite's slot list (tasks 7.3, 7.4) |
| `qth_critical_path` | — | — | manual: unchanged; carried in the MODIFIED block, read from CTest's critical-path output (task 12.3) |
| `qth_suite_timeout` | — | — | manual: unchanged; carried in the MODIFIED block, the existing run exercises it |
| `qth_longest_entry` | Run the Qt layer at the default level and read CTest's per-test times and `Total Test time (real)` | No entry exceeds a quarter of the total, or it is named next to the shard registration | manual: a wall-time property of a full run (task 12.3) |
| `bp_bridge_impl_no_rerun` | Append a dead `fn` to the implementation module behind `cxxqt_object/type_tools.rs`; rebuild | The cargo output shows no `pictura_app` build-script run, and the appended `.ninja_log` lines hold no C++ object | manual: needs a real incremental build (task 12.3) |
| `bp_bridge_decl_only` | Run the unit test that scans every bridge source | No bridge file holds a function body or an inline module body outside its bridge module | `pictura-app::bridge_files_hold_declarations_only` |
| `bp_clean_rebuild` | `ninja -C build -t clean && cmake --build build --parallel 12` | The build succeeds and links the shell and every `tst_*` | manual: needs a full rebuild (task 12.6) |
| `bp_ci_target_cache` | Read the PR's `lint` and `test` logs | Each job's rust-cache step uses its own key, restores when a `master` cache exists, and skips saving | manual: needs a CI run (task 11.1); the after-merge wall times and sizes go to the follow-up issue |
| `bp_engine_edit_measured` | `cargo build --timings` after a one-line `pictura-core` and `pictura-render` edit under `dev-app` and `--release` | `design.md` §10 lists the per-crate times and the override decision | manual: timings on the reference machine (tasks 1.3, 12.5) |
| `bp_cheap_tu` | `ninja -C build -t commands` for `filter_commands.cpp.o` in the development tree | Its last `-O` flag is `-O1` | manual: reads generated commands; the requirement only drops the removed `command_tree.cpp` (task 10.3) |
| `bp_test_debug` | — | — | manual: unchanged; carried in the MODIFIED block, checked with `bp_compile_change_per_source` |
| `bp_heavy_tu_cheapened` | — | — | manual: unchanged; carried in the MODIFIED block, decided by an earlier change's recorded measurement |
| `bp_test_tu_cost` | Measure test AUTOMOC and `mocs_compilation` CPU before and after | `design.md` §7 records the numbers and the verdict | manual: timings on the reference machine (task 12.4) |
| `bp_compile_change_measured` | Measure each experiment cold and warm, before and after | `design.md` §7's table lists both numbers and a verdict for each | manual: timings on the reference machine (task 12.4) |
| `bp_compile_change_release` | Configure throwaway `-DBUILD_TESTING=OFF` and `-DCMAKE_BUILD_TYPE=Debug` build dirs; `ninja -t commands` | No `-include`/`.gch`/`cmake_pch`, no `-Og` and no `-gsplit-dwarf` in any compile command | manual: needs fresh configures (task 12.4) |
| `bp_compile_change_per_source` | `ninja -C build -t commands` for `filter_commands.cpp.o` and one `tst_*.cpp.o` | The last `-O` flag is `-O1` for `filter_commands.cpp`, and the last `-g` flag is `-g1` for the test unit | manual: reads generated commands of the development tree (task 12.4) |
| `bp_pch_off_builds` | Configure a throwaway dir with `-DCMAKE_DISABLE_PRECOMPILE_HEADERS=ON` and build it | Every source compiles and every target links | manual: needs a full build; CI's `qt-headless` job repeats it on every PR (tasks 10.4, 12.6) |
