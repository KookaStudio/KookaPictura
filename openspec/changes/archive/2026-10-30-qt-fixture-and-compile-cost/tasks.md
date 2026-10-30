# Tasks

Order follows the AGENTS.md workflow. Group 1 is the baseline, group 2 holds
the tests, groups 3–11 write all code and text, and group 12 compiles once and
runs every measurement. No group from 2 to 11 builds or runs tests.

There are two declared exceptions to "compile once":

- Task 1.3 is a measurement-only Cargo build of the untouched tree, done before
  implementation, because #1043 F asks for it first.
- Task 12.4 has to build each compile-pipeline experiment cold and warm,
  before and after, to apply the measurement rule. Those are measurement
  builds, not fix-and-retry loops.

## 1. Baseline (exploration, 2026-10-30)

- [x] 1.1 Claim #1043: assign it and comment that work has begun on `perf/1043-qt-test-cpp-compile`.
- [x] 1.2 Record the baseline in `design.md` → Context. Steps, all at `-j12` in this worktree:
  - `cmake -S . -B build -G Ninja`, then `SCCACHE_RECACHE=1 cmake --build build --parallel 12` (cold).
  - `find build -path '*CMakeFiles*' -name '*.o' -delete`, then a rebuild (warm, from sccache).
  - `bash scripts/test-report.sh` (Qt layer at `-j4`), with the JUnit summed per suite and per case.
  - Append a dead `fn` to `cxxqt_object/type_tools.rs`, `history/styles.rs`, `pictura-core/src/lib.rs` and `pictura-render/src/lib.rs` in turn. Rebuild after each and take the `.ninja_log` lines that build appended. Revert with `git checkout`.
  - Profile window construction: run `tst_workspaces` under gdb with `ualarm(50000,50000)` and `handle SIGALRM stop print nopass`, and aggregate `thread apply 1 bt`.
- [x] 1.3 Measure engine-crate edits before any implementation (#1043 F). Append a dead `fn` to `crates/pictura-core/src/lib.rs`, run `cargo build --timings --profile dev-app -p pictura_app`, revert, and repeat with `--release`. Do the same for `crates/pictura-render/src/lib.rs`. Record the per-crate times in `design.md` §10 and the baseline table, replacing the "not measured" release row. If the dependents dominate, note in `design.md` §10 that the profile decision moves ahead of group 10. Verify that the table has four filled rows.

## 2. Tests first

- [x] 2.1 Write `test-plan.md` with one row per spec scenario. Check it with `python3 scripts/check-test-plan.py`: only the "no such test" lines for tests that group 2 has not written yet may remain.
- [x] 2.2 Add the Qt Test suite `tst_theme_apply`. Register it in `PICTURA_QT_TESTS` in `crates/pictura-app/cpp/tests/CMakeLists.txt` (one entry, not sharded, because the cases depend on process-wide theme state). Add five cases that run in group 12:
  - `secondWindowKeepsTheme`: construct a window, record `Theme::paletteGeneration()` and the `QApplication::style()` pointer, construct a second window with the same session, and assert both are unchanged. *Fails on today's tree*: every construction installs a new style.
  - `brightnessStepApplies`: start at level 1 (below the highest), step the brightness up, and assert the generation advances and the palette changes. *Guard*: passes today.
  - `chromeBuiltUnderRestoredTheme`: start from a light-family session at a non-default level. Start from the dark theme. Install an application event filter that snapshots `qApp->styleSheet()` the first time a `LayersPanel` and the `OptionsBar` receive `QEvent::ChildAdded` (while they build their contents), and assert both snapshots equal the light sheet. Then assert `qApp->styleSheet()` equals `Theme::styleSheet(family, level)` and that `qApp->palette()` matches what a fresh `Theme::apply` with those settings produces. *Fails on today's tree*: the theme is applied after the panels and the options bar are built.
  - `reapplyAfterExternalStyleSheet`: apply a theme, call `qApp->setStyleSheet(QString())`, apply the same theme again, and assert `qApp->styleSheet()` equals the theme's sheet. *Guard*: passes today, and fails if the short-circuit trusts a private record.
  - `chromeFontsSurvive`: construct a window. Assert the document tab bar font is bold, with a pixel size two below the application font's. Assert the status bar font equals the footer font that `buildStatusBar` sets. *Guard*: records today's result, and a difference is noted in `design.md` §1.
- [x] 2.3 Add `bridge_files_hold_declarations_only` to `crates/pictura-app/src/build_config.rs`. For every path in build.rs's `.files([...])` list, it asserts that the file holds a bridge and that no `fn` with a body and no `mod` with an inline body sits outside the bridge module. `mod x;` declarations, with or without `#[cfg(test)]`, are allowed, as are `use`/`pub use` items and `impl … for … {}` (or `unsafe impl`) with an empty body. It reuses the module-extraction helper. *Fails on today's tree* (for example `type_tools.rs:427`, `decode_bridge.rs:16`) and passes on `src/cxxqt_object.rs`.
- [x] 2.4 Add `qt_test_waits_are_justified` to `crates/pictura-app/src/build_config.rs`. It scans `crates/pictura-app/cpp/tests/*.cpp` and `*.h` and fails on every `qWait(…)` whose argument is not the literal `0` and that has neither a `//` comment after the call on its line nor a comment-only line right above it. It lists file:line for each. *Fails on today's tree* (183 calls, mostly uncommented).
- [x] 2.5 Add `freshLayoutMatchesEssentialsApply` to `tst_workspaces`. Construct a window from an empty session and record its arrangement: per column its groups in order, each group's panel names in order, the active tab, iconic and visible state. Apply the Essentials workspace and assert the arrangement is unchanged. *Guard*: passes today, because today's path ends in the same regroup, and it pins the result of task 6.1.
- [x] 2.6 Add the Qt Test suite `tst_test_support` (registered in `PICTURA_QT_TESTS`, no window) with these cases:
  - `namedFunctionsAreKept`: the argument list built for argv `{exe, "foo"}` with shard `0/1` is `{exe, "foo"}`.
  - `optionValuesAreNotFunctions` and `perfcounterValueIsNotAFunction`: the value of `-o` or `-perfcounter` is not read as a function name.
  - `shardAppendsItsShare`: with argv `{exe}` and shard `1/4`, the list is `{exe}` plus that share's functions.
  - `emptyShareRunsNothing`: a share with no functions is detected, so `shardedMain` fails it with a message instead of letting QTest run every function.

  Factor the argument-list construction in `qt_test_support.h` into a pure function `shardArguments(...)` for this. *First case fails on today's tree*.

## 3. Theme applied once (design §1)

- [x] 3.1 In `Theme::apply`, build the palette and style sheet. When `QApplication::style()` is a `SpinArrowStyle`, `qApp->palette()` equals the built palette and `qApp->styleSheet()` equals the built sheet, record the family and level and return before `setStyle`/`setPalette`/`setStyleSheet`, the pixmap-cache clear and the generation bump. Verify with `tst_theme_apply::secondWindowKeepsTheme` and `reapplyAfterExternalStyleSheet` (group 12).
- [x] 3.2 In the `PicturaMainWindow` constructor, call `Theme::apply` for the restored family and `state.brightnessLevel` right after `setFont(QApplication::font())` (`frame.cpp:144`) and before `buildMenus` (`:156`). Set `brightnessLevel_` there. Keep the later `applyBrightness` for `applyWorkspaceCanvasColor`. Verify with `tst_theme_apply::constructionPolishesChromeOnce` and `chromeFontsSurvive`.
- [x] 3.3 Grep `crates/pictura-app/cpp/tests/` and `selftest*.cpp` for direct `qApp->setPalette`, `setStyleSheet` and `setStyle` calls. Today the only one is `tst_keyboard_shortcuts.cpp:465`. Confirm that each one is handled by the live comparison and record the list in `design.md` §1. Verify with `tst_keyboard_shortcuts` passing in group 12.

## 4. CTest parallel level and the slowest entry (design §3)

- [x] 4.1 Make `scripts/test-report.sh` default `qt_parallel` to `max(1, nproc*8/10)` and keep `CTEST_PARALLEL_LEVEL` as the override. In group 12, verify by running it with `CTEST_PARALLEL_LEVEL=2` and under `taskset -c 0` and reading the `== qt tests (ctest -j N) ==` line.
- [x] 4.2 In `.github/workflows/ci.yml` (`qt-headless`, the `level=` line), default to `$(nproc)` and keep the override. Verify that the PR's CI log prints `ctest -j 4` or more.
- [x] 4.3 Switch `tst_ui_scale` from `QTEST_MAIN` to `pictura::test::shardedMain` and add it to `PICTURA_QT_TEST_SHARDS`. Its `init()` builds a fresh window per function. In group 12, verify by running `PICTURA_TEST_SHARD=k/8` for each k (it has eight shares, design §3) and checking that the eight JUnit files together hold every function once.
- [x] 4.4 Update the CTest parallel default where it is stated as four: `AGENTS.md:62` and `DEVELOPING.md:271` and `:415`. `docs/dev/testing-conventions.md` states no default and stays. Verify with `grep -n 'four' AGENTS.md DEVELOPING.md` that no stale default remains.

## 5. Condition waits (design §5)

- [x] 5.1 Inventory every `QTest::qWait(` in `crates/pictura-app/cpp/tests/` (183 today) and classify each one as condition, event flush or negative window. Keep the per-class counts and the negative-window list in `design.md` §5. Verify that the counts sum to the grep count.
- [x] 5.2 Replace the condition waits with `QTRY_VERIFY`/`QTRY_COMPARE`/`QTest::qWaitFor`. Replace the event flushes with `QCoreApplication::sendPostedEvents` or `qWait(0)`. Give each remaining delay a reason comment on its line or the line above. Verify with `build_config::qt_test_waits_are_justified`, and in group 12 run the full Qt layer three times.

## 6. First-run layout (design §2)

- [x] 6.1 On a fresh session (no stored groups or columns), have `buildPanels` stage its default groups instead of attaching them to the column, and have `regroupAndApply` harvest the staged groups (design §2). The fresh-session test moves to `sessionHasPanelLayout` in `session.cpp`. Workspace switching keeps `regroupAndApply`. Verify with `tst_workspaces::freshLayoutMatchesEssentialsApply`, `freshSessionUsesEssentialsDefault`, `tst_panels_parity` and `tst_initial_view`.

## 7. Shared windows, suite by suite (design §4)

- [x] 7.1 Use `shardArguments` in `shardedMain` so argv's named functions are kept. Verify with `tst_test_support::namedFunctionsAreKept`.
- [x] 7.2 For each of `tst_document_float`, `tst_document_windows`, `tst_document_arrange`, `tst_floating_workspaces`, `tst_workspaces` and `tst_import_pdf`, list the state its functions read and decide whether an `init()` reset can restore it. Record the decision per suite in a table in `design.md` §4. Verify that the table has six rows.
- [x] 7.3 Convert the suites marked "share": build one window in `initTestCase`, reset in `init()`, close in `cleanupTestCase`. Update the registration comment at `tests/CMakeLists.txt:191-193` to the spec's rule. Verify each converted suite in group 12 by running every function alone (`tst_x <fn>`, with no shard variable) and every shard (`PICTURA_TEST_SHARD=k/4`).
- [x] 7.4 Apply the same review to the suites that joined the shards in group 12: `tst_edit_refresh` (one window since `initTestCase`), `tst_print` (no shared window) and `tst_loading_tab` (one window since `initTestCase`). Add their rows to the `design.md` §4 table and any missing reset to `cleanup()`. Verify every function alone and every shard, as in 7.3.

## 8. Clean rebuild (design §8)

- [x] 8.1 Make the C++ build regenerate the cxx-qt export directory after `ninja -t clean`, by declaring the export outputs as cargo-step byproducts or by forcing the cargo step to rerun when they are missing. Record the mechanism in `design.md` §8. Verify with a clean followed by a build in group 12.

## 9. Declarations-only bridge files (design §6)

- [x] 9.1 Move the function bodies, helpers and inline `mod tests { … }` of the 76 bridge files that hold implementation into implementation modules: the 75 under `crates/pictura-app/src/cxxqt_object/` and `crates/pictura-app/src/decode_bridge.rs`. Use a sibling implementation module or an existing `impl_*` module. Leave the bridge module, the `mod` declarations, the `use`/`pub use` items and the empty trait impls. Pure moves only, a few files per commit, and build.rs's file list unchanged. Verify with `build_config::bridge_files_hold_declarations_only` and `cargo clippy -p pictura_app --all-targets -- -D warnings` in group 12, and check that `git diff crates/pictura-app/build.rs` is empty.
- [x] 9.2 Keep each new implementation module under the file-size caps, splitting along its seam where needed. Update `.serena/memories/*` where they describe the bridge layout. Verify with `bash scripts/check-file-size.sh`.

## 10. Compile-pipeline experiments (design §7, kept or reverted in group 12)

- [x] 10.1 Add a precompiled header for `pictura_shell` (Qt Core/Gui/Widgets and common std headers; the generated cxx-qt headers stay out, design §7), plus a separate `pictura_test_pch` target that the Qt Test targets `REUSE_FROM`. The test target is separate because of PIC vs PIE and `-g1`. Use both only in the optimized development configuration with GCC or Clang. Verify in group 12 that the compile commands for `-DBUILD_TESTING=OFF` and for a Debug configure carry no PCH.
- [x] 10.2 Measure removing the per-test `mocs_compilation` TU and the AUTOMOC scan cost for the Qt Test targets. Measured; skipped (design §7): no measurable gain, so the change was reverted.
- [x] 10.3 Add `-Og -g1` for the optimized development configuration as target-level options, so the per-source `-O1` (`filter_commands.cpp`) and the tests' `-g1` still come last. Verify from `ninja -t commands` in group 12. `-gsplit-dwarf` was measured; skipped (design §7). `command_tree.cpp` no longer exists.
- [ ] 10.4 In the `qt-headless` job of `.github/workflows/ci.yml`, configure with `-DCMAKE_DISABLE_PRECOMPILE_HEADERS=ON`, so CI compiles every source without the PCH. Verify from the PR's CI configure log and a green build.

## 11. CI target cache (design §9)

- [ ] 11.1 Split the shared `*rustcache` anchor in `.github/workflows/ci.yml`:
  - `lint` and `test` get `cache-targets: "true"`, a per-job `shared-key` and `save-if: ${{ github.ref == 'refs/heads/master' }}`;
  - `qt-headless` keeps `cache-targets: "false"` (Corrosion builds outside `target/`).

  Verify from the PR's CI log that both jobs restore with their key and that the save step is skipped on the pull request. The wall times and cache sizes after the first `master` run are recorded under the follow-up issue (design §9).

## 12. Compile once, measure, decide

- [x] 12.1 Build once at `-j12` (`cmake --build build --parallel 12`), fix errors, and rebuild only if the code changed.
- [x] 12.2 Run `bash scripts/test-report.sh` and confirm that `tst_theme_apply`, `tst_test_support`, `tst_workspaces`, both new `build_config` tests and the whole Qt layer pass. Run the Qt layer three times at the new default to check for flakiness.
- [x] 12.3 Repeat the baseline steps (task 1.2) and fill the after-columns in `design.md`: Qt layer wall time, `initTestCase` median, per-window profile shares, the longest entry against CTest's total time, and the bridge-edit wall time with confirmation that the build script did not rerun.
- [x] 12.4 Measure each group 10 experiment cold and warm, before and after (declared exception to compile-once). Keep or revert each one by the rule, and fill the experiment table in `design.md` §7.
- [x] 12.5 Decide on engine profile overrides from task 1.3's numbers and the Qt layer's wall time (design §10), and record the decision.
- [x] 12.6 Run the clean-rebuild check (`ninja -C build -t clean && cmake --build build --parallel 12`), a configure with `-DCMAKE_DISABLE_PRECOMPILE_HEADERS=ON` into a throwaway build dir, and the remaining manual rows of `test-plan.md`.
- [x] 12.7 Update `docs/dev/STATE.md` with the new numbers in a commit of its own whose message carries `TASK-ALLOWS-DOCS`. Verify with `GUARD_BASE=master bash scripts/guard.sh`.
- [ ] 12.8 Run `bash scripts/verify-fast.sh`, archive the change with `openspec archive qt-fixture-and-compile-cost`, and list the plan's manual rows in the PR body. Verify that `scripts/check-openspec-archived.sh` passes.
