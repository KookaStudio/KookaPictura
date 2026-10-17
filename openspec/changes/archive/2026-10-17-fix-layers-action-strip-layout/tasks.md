# Tasks

## 1. Regression test

- [x] 1.1 Add the `actionStripStaysFixedAndCentred` declaration to the `private slots:` block of `LayersPanelFixesTest` and include `<QtCore/QList>`, `<QtWidgets/QLayout>`, and `<QtWidgets/QToolButton>` in `crates/pictura-app/cpp/tests/tst_layers_panel_fixes.cpp`; verify the declaration sits in the slots block that moc registers.
- [x] 1.2 Implement the case: construct a standalone `pictura::LayersPanel`, use `minimumSizeHint().width() + 80` for the narrow width and +240 more for the wide width, show and wait 50 ms, assert the panel reached the narrow width, record the buttons, resize, activate the layout, wait 50 ms, assert the panel reached the wide width, and assert 2 px gaps, unchanged widths, and margins equal within 1 px at both widths.
- [x] 1.3 Keep the test's gap arithmetic edge-based (`next->x() - (previous->x() + previous->width())`) and its margin arithmetic inclusive (`panel.width() - (last->x() + last->width())`), so the assertions do not depend on `QRect::right()` off-by-one behaviour.

## 2. Strip layout fix

- [x] 2.1 In `crates/pictura-app/cpp/panels/layers_panel.cpp`, set `buttons->setSpacing(2)`, add `buttons->addStretch(1)` before the first button and after the last, and leave the buttons' existing `QToolButton` size policy untouched; the build and test verification is 3.2.
- [x] 2.2 Confirm via `git diff` that the seven object names, icons, tooltips, click handlers, `layerDropAction` properties, and drop event filters are unchanged.

## 3. Integration verification

- [x] 3.1 Build with the documented sequence: `cmake -S . -B build -G Ninja -DCMAKE_EXE_LINKER_FLAGS=-fuse-ld=lld && cmake --build build --parallel 12` (12 of this machine's 16 CPUs, per the 80% cap); `BUILD_TESTING` defaults on, so the Qt suites are built.
- [x] 3.2 Verify the slot is registered and the focused suite passes: `QT_QPA_PLATFORM=offscreen ./build/crates/pictura-app/cpp/tests/tst_layers_panel_fixes -functions` lists `actionStripStaysFixedAndCentred`, then `ctest --test-dir build -R '^tst_layers_panel_fixes$' --output-on-failure` passes.
- [x] 3.3 Verify the existing Layers-panel, panel, and icon tests still pass: `ctest --test-dir build -R '^(tst_layers_panel|tst_layers_panel_fixes|tst_panels_parity|tst_icon_assets)$' --output-on-failure`.
- [x] 3.4 Run the pre-archive guards while the change is still unarchived: `python3 scripts/check-test-plan.py` and `openspec validate --all --strict`. This is the only test-plan check, because `check-test-plan.py` excludes `openspec/changes/archive/`.
- [x] 3.5 Archive the completed change after 3.1-3.4 are checked: `openspec archive fix-layers-action-strip-layout -y`. The CLI warns that 3.6 is still unchecked; `-y` proceeds.
- [x] 3.6 Run `bash scripts/verify-fast.sh`; verify fmt, clippy, the unified test report, file-size, archive, guards, and OpenSpec validation pass. After it passes, tick 3.5 and 3.6 in the archived `openspec/changes/archive/<date>-fix-layers-action-strip-layout/tasks.md`.
