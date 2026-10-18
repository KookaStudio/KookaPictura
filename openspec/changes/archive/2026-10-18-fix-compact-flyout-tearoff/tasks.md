# Tasks

## 1. Regression tests

- [x] 1.1 Add `compactFlyoutClosesWhenGroupTearsOff`, `compactFlyoutStaysOpenForTabTearOff`, and `compactFlyoutClosesWhenTabEmptiesIt` to the `private slots:` block of `PanelsParityTest` in `crates/pictura-app/cpp/tests/tst_panels_parity.cpp`, and include `panels/panel_column.h` and `panels/panel_group.h`.
- [x] 1.2 Implement the group tear-off case: rail mode, open the Layers flyout, `beginGroupDragForTest`, drag to the document area, drop; assert the flyout is hidden, one float holds Layers/Channels/Paths, and the source column no longer owns Layers.
- [x] 1.3 Implement the tab tear-off case: rail mode, open the Layers flyout, `beginTabDragForTest("channelsPanel")`, drag to the document area, drop; assert the flyout is visible, its group keeps Layers and Paths, and the new float holds only Channels.
- [x] 1.4 Implement the emptying tab tear-off case: rail mode, open the Properties flyout (its group has one visible panel), `beginTabDragForTest("propertiesPanel")`, drag to the document area, drop; assert the flyout is hidden and the new float holds only Properties.
- [x] 1.5 Clean every case back to normal mode and close its floats so later suites see the default layout.

## 2. Flyout handoff fix

- [x] 2.1 In `PanelColumn::beginGroupDrag` (`panel_column_drag.cpp`), capture the grab offset from the group's flyout position, and when the visible flyout hosts that group, call `closeIconFlyout()` before the drag state is set; use the captured offset for `dragGrabOffset_`.
- [x] 2.2 In `PanelColumn::createFloat` (`panel_column_float.cpp`), after the group/panel has been rehomed, close the visible flyout when it hosts the source group and the source group has no visible panels; leave it open otherwise.
- [x] 2.3 Confirm via `git diff` that the drag grammar, drop indicators, float geometry, and panel registry are otherwise unchanged.

## 3. Integration verification

- [x] 3.1 Build with `cmake -S . -B build -G Ninja -DCMAKE_EXE_LINKER_FLAGS=-fuse-ld=lld && cmake --build build --parallel 12`.
- [x] 3.2 Verify the slots are registered with `QT_QPA_PLATFORM=offscreen ./build/crates/pictura-app/cpp/tests/tst_panels_parity -functions` and run `ctest --test-dir build -R '^tst_panels_parity$' --output-on-failure`.
- [x] 3.3 Run the neighbouring panel suites: `ctest --test-dir build -R '^(tst_panels_parity|tst_workspaces|tst_layers_panel|tst_layers_panel_fixes)$' --output-on-failure`.
- [x] 3.4 Run `python3 scripts/check-test-plan.py` and `openspec validate --all --strict`.
- [x] 3.5 Archive the change with `openspec archive fix-compact-flyout-tearoff -y` after 3.1-3.4 pass.
- [x] 3.6 Run `bash scripts/verify-fast.sh`; every gate passed except the pre-existing local `imagemagick_fixtures_reproduce` fixture mismatch (`ramp_Divide.rgba`) documented in `docs/dev/STATE.md`.
