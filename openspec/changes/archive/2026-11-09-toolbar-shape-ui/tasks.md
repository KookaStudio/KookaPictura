# Tasks

## 1. Tests

- [x] 1.1 Add `crates/pictura-app/cpp/tests/tst_toolbar_shape.cpp` with
  `compactLayoutHasOneColumnAndNoScrollbar`,
  `doubleArrowExpandsToTwoColumns`, `gripDragFloatsOutsideTheWindow`,
  `bothLayoutsDockLeftAndRight`, `toolbarVisibleOnStartupWhenWorkspaceIsShort`,
  `floatingDragBesideAppWindowStaysFloating`, and
  `closeHidesAndWindowMenuRestores`; register the suite in
  `crates/pictura-app/cpp/tests/CMakeLists.txt`.
- [x] 1.2 Keep the existing suites that assume the toolbar is visible at
  1100×700 on the always-visible, clipped-panel rule.
- [x] 1.3 Add `floatingExpandedToCompactKeepsAllTools` so a floating 2→1 flip
  is checked against the real slot/footer geometry, not just the overlay height.

## 2. Header and grip

- [x] 2.1 Add a `ToolbarDragGrip` widget to `PanelColumn`: the same full-width
  `•••` label the iconic History/Properties panel-group grip uses, move cursor,
  tooltip, and an event filter on the same column-drag path as the title row.
- [x] 2.2 Add a close `QToolButton` to the tools header, place the toggle on
  the left and the close on the right, and emit `closeRequested()`.
- [x] 2.3 Insert the grip below the title row only for the tools column; keep
  widget-column headers unchanged.
- [x] 2.4 Expose test hooks: `toolsGripForTest`, `toolsCloseButtonForTest`, and
  the tools scroll-bar policy; the grip drag test drives real mouse events so
  the event-filter wiring is exercised.

## 3. No scrollbar, short-window clipping, and floating dock zones

- [x] 3.1 Set the tools scroll host's vertical policy to `AlwaysOff` and keep
  the content top-aligned.
- [x] 3.2 Add `PanelColumn::toolsRequiredHeight()` for the current layout; use
  it only to size a floating overlay.
- [x] 3.3 Keep the docked tools column visible when the workspace is too short
  so its content clips at the bottom instead of hiding or scrolling.
- [x] 3.4 Add `PicturaMainWindow::newColumnSideAtInsideWorkspace()` and use it
  for a floating tools column, so a desktop drag never draws a dock indicator
  and only the workspace's left/right edge band re-docks.
- [x] 3.5 Wire the close control to hide the column, refresh the command
  registry, and save the session; keep `Window > Panels > Tools` restoring it.

## 4. Verification

- [x] 4.1 `openspec validate toolbar-shape-ui --strict` and
  `openspec validate --all --strict`.
- [x] 4.2 `python3 scripts/check-test-plan.py`.
- [x] 4.3 `cmake --build build --parallel` (within 80% CPU).
- [x] 4.4 `ctest --test-dir build -R '^tst_toolbar_shape$' --output-on-failure`.
- [x] 4.5 `QT_QPA_PLATFORM=offscreen ./build/pictura --headless --self-test`.
- [x] 4.6 `bash scripts/verify-fast.sh`.
