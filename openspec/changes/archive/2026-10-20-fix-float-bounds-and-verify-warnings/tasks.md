# Tasks

## 1. Self-test determinism

- [x] 1.1 In `crates/pictura-app/cpp/selftest_shell_round4.cpp`, save `frame.pos()` before the `float_screen_bounds` setup, call `frame.move(0, 0)` after `frame.resize(700, 500)`, and restore both the saved position and size after the check; verify the diff shows the position is restored alongside `resize(frameBefore)` and the available right band is used deterministically.
- [x] 1.2 Confirm the `ST_PASS` token still reports the screen and frame bounds and that `torn`, `on_screen`, and `outside` remain the pass conditions; verify at least one band is required and the chosen edge is checked against the frame.

## 2. Build and verification warning causes

- [x] 2.1 Change `[profile.test.package.pictura-app]` to `[profile.test.package.pictura_app]` in `Cargo.toml`; verify no `pictura-app` profile key remains with `rg -n 'profile\.test\.package\.pictura-app' Cargo.toml`.
- [x] 2.2 Set `QT_NO_PRIVATE_MODULE_WARNING ON` before `find_package(Qt6 ...)` in `CMakeLists.txt`, with a comment naming the RHI private-header dependency; verify the variable precedes the `find_package` line.
- [x] 2.3 Remove the redundant `modelReset`/`dataChanged` connections and the `setSourceModel` override from `crates/pictura-app/cpp/panels/layers_filter_proxy.cpp` and `.h`, and call `invalidate()` from `setFilter()`; verify no `invalidateFilter` or `setSourceModel` override remains and the proxy relies on `dynamicSortFilter` for source changes.
- [x] 2.4 Cast the ignored `QFile::open()` result in `crates/pictura-app/cpp/selftest_session.cpp` to `void`; verify the call is explicitly discarded and the surrounding `placesDetails` failure path is unchanged.
- [x] 2.5 Cast the ignored `QTest::qWaitForWindowActive()` result in `crates/pictura-app/cpp/tests/tst_edit_shortcuts.cpp` to `void`; verify the cleanup still closes the document and returns from the lambda.
- [x] 2.6 Replace `QApplication::setActiveWindow(&dialog)` with `dialog.activateWindow()` in `crates/pictura-app/cpp/tests/tst_keyboard_shortcuts.cpp`; verify `rg -n 'setActiveWindow' crates/pictura-app/cpp/tests` returns nothing.
- [x] 2.7 Add `-Wno-sfinae-incomplete` and `-Wno-maybe-uninitialized` with `flag_if_supported` to the cxx-qt `cc_builder` in `crates/pictura-app/build.rs`, with a comment naming the Qt/cxxbridge glue; verify the clean build emits no `-Wsfinae-incomplete` or `Vec<LayerRow>` warning.

## 3. Build, verification, and archive

- [x] 3.1 Remove any existing `build/` directory, then configure and build from scratch with `cmake -S . -B build -G Ninja -DCMAKE_EXE_LINKER_FLAGS=-fuse-ld=lld` and `cmake --build build --parallel`, capturing stderr; verify the output has no project-caused CMake or compiler warning (third-party-only warnings are documented).
- [x] 3.2 Run `./build/pictura --headless --self-test` twenty times; verify every run prints `pictura self-test: PASS core float_screen_bounds` with the available right band present, the float clamped on that axis, and exits 0.
- [x] 3.3 Run `ctest --test-dir build -R 'tst_layers_panel|tst_layers_panel_fixes|tst_layers_smart_filters|tst_edit_shortcuts|tst_keyboard_shortcuts' --output-on-failure`; verify the filter, cleanup, and activation suites pass after the warning fixes.
- [x] 3.4 Run `python3 scripts/check-test-plan.py`, `openspec validate --all --strict`, and `bash scripts/guard.sh`; verify all pass with the change folder present.
- [x] 3.5 Archive the change with `openspec archive fix-float-bounds-and-verify-warnings`; verify `openspec/changes/` contains no active copy and `scripts/check-openspec-archived.sh` passes.
- [x] 3.6 Run `bash scripts/verify-fast.sh` and `bash scripts/verify-full.sh` after archiving, capturing stderr; verify neither prints a warning caused by project code or configuration.
- [x] 3.7 Record the `cargo nextest` and `cargo clippy` durations after the `pictura_app` profile key takes effect, and compare them with the 30-minute CI timeout; verify they remain within budget or record follow-up work. Measured: nextest 29 s, ctest 657 s, verify-full 1784 s; clippy is part of the fast gate and reported no errors.
- [ ] 3.8 Prepare the PR body: list the manual test-plan rows as the reviewer checklist, note the captured warning inventory, and note that release PR #293 needs a re-run after this lands; verify the PR title and every commit end with `(#767)`.
