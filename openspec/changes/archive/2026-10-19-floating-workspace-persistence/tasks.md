# Tasks

## 1. Planned tests (author first; run in section 7)

- [x] 1.1 Create `crates/pictura-app/cpp/tests/tst_floating_workspaces.cpp`, register the suite in `crates/pictura-app/cpp/tests/CMakeLists.txt`, and confirm the file and explicit test-list entry are present.
- [x] 1.2 Author `flyoutHostsWholeGroup` (whole group, clicked panel current, active icon, restores on close); verify the slot exists.
- [x] 1.3 Author `flyoutSurvivesOutsideClick`; verify the slot exists.
- [x] 1.4 Author `flyoutTogglesAndSwitches`; verify the slot exists.
- [x] 1.5 Author `escapeDoesNotCloseFlyout` as a regression guard; verify the slot exists.
- [x] 1.6 Author `floatingIconRowOpensFlyout`; verify the slot exists.
- [x] 1.7 Author `floatingIconRowShowsOneHeader`; verify the slot exists.
- [x] 1.8 Author `flyoutGroupLeavesNoStaleOverlay` (drag-out, last panel/group close, and owner-column removal variants); verify the slot exists.
- [x] 1.9 Author `flyoutClosesOnHideAll`; verify the slot exists.
- [x] 1.10 Author `flyoutCloseButtonPlacement` (left of the widget menu, actual-width placement, header-corner width); verify the slot exists.
- [x] 1.11 Author `flyoutCloseButtonHidesFlyout` (group returns to its owner slot); verify the slot exists.
- [x] 1.12 Author `flyoutCloseButtonOnlyInFlyout`; verify the slot exists.
- [x] 1.13 Author `flyoutResizesFromLeftEdge`; verify the slot exists.
- [x] 1.14 Author `flyoutResizesFromRightEdge`; verify the slot exists.
- [x] 1.15 Author `flyoutResizesBesideFloatingRow`; verify the slot exists.
- [x] 1.16 Author `flyoutResizeKeepsMinimum`; verify the slot exists.
- [x] 1.17 Author `flyoutOpensAtMinimumWidth`; verify the slot exists.
- [x] 1.18 Author `flyoutChildModeStaysInFrame` using the forced-child host; verify the slot exists.
- [x] 1.19 Author `floatHeaderMatchesDockedHeader`; verify the slot exists.
- [x] 1.20 Author `floatCornerMenuPainted` (grab the whole float, sample the button rect); verify the slot exists.
- [x] 1.21 Author `iconicColumnFloatKeepsTwoThirdsHeight`; verify the slot exists.
- [x] 1.22 Author `normalColumnFloatKeepsTwoThirdsHeight`; verify the slot exists.
- [x] 1.23 Author `iconicColumnSnapsWhenRailToggled` as a regression guard; verify the slot exists.
- [x] 1.24 Author `workspaceRoundTripsGroupFloat` (including a duplicate group name from a split multi-panel group); verify the slot exists.
- [x] 1.25 Author `workspaceRoundTripsColumnFloat`; verify the slot exists.
- [x] 1.26 Author `workspaceRoundTripsFloatInsideFloat` (group floated out of a floating column); verify the slot exists.
- [x] 1.27 Author `workspaceSwitchRemembersFloats`; verify the slot exists.
- [x] 1.28 Author `workspaceWithoutFloatsDocksEverything`; verify the slot exists.
- [x] 1.29 Author `restartRestoresFloats` (group and whole column); verify the slot exists.
- [x] 1.30 Author `restartRestoresAutoFlags`; verify the slot exists.
- [x] 1.31 Author `restoredFloatIsClamped` (oversized and undersized geometry); verify the slot exists.
- [x] 1.32 Author `toolsFloatRestores` (assert the serialized tools entry carries `floating`, `floatGeometry`, and the group list is empty); verify the slot exists.
- [x] 1.33 Author `toolsFloatSurvivesWorkspaceApply`; verify the slot exists.
- [x] 1.34 Author `legacyStoreDocksEverything`; verify the slot exists.
- [x] 1.35 Author `startupChildFloatUsesFrameGeometry` (forced child overlay, startup order); verify the slot exists.
- [x] 1.36 Update `tst_workspaces::wholeColumnFloatDoesNotPersistPhantom` to read the layout immediately after tear-off and before switching workspaces; verify the slot still exists.
- [x] 1.37 Extend `tst_command_tree` with the flyout close button's header-corner width and update `tst_edit_shortcuts` if the flyout host changes `window()` identity; verify both slots exist.
- [x] 1.38 Reconcile `test-plan.md` rows with every delta scenario and every test name above; verify no scenario key is missing.
- [x] 1.39 Author `tst_floating_workspaces_restart` with `restartRestoresFloatAtRightOfRestoredFrame` (forced child overlay, non-default window geometry, float moved to the right) and map it to `spr_floats_startup_position`; verify the slot exists.

## 2. Layout schema and serialization

- [x] 2.1 Add `floatGeometry` to each floating group entry in `PanelColumn::savePanelState()`; verified by `workspaceRoundTripsGroupFloat`.
- [x] 2.2 Add the column-level `floating` and `floatGeometry` fields in `serializeWorkspaceLayout()`, remove the floating-column fold into the primary entry, and use `PanelColumn::side()` for floating columns; verified by `workspaceRoundTripsColumnFloat`.
- [x] 2.3 Handle `isToolsColumn()` before the floating-fold/skip branch so a floating Tools column always emits its tools entry with empty groups and its geometry; verified by `toolsFloatRestores`.
- [x] 2.4 Add geometry accessors and `restoreGroupFloat()` / `restoreColumnFloat()` to `PanelColumn`; the group path must join the owner's `floats_`, install the same `onClose` callback as `createFloat()`, and apply stored geometry last; verified by `workspaceRoundTripsGroupFloat` and `workspaceRoundTripsColumnFloat`.
- [x] 2.5 Clamp restored overlay position and size to the available screen/frame and the overlay minimum; verified by `restoredFloatIsClamped`.
- [x] 2.6 Bump the session schema to 10 in `SessionState`/`saveSession()` and require both the `floating` marker and valid geometry before detaching; verified by `legacyStoreDocksEverything`.
- [x] 2.7 Persist the Tools column's floating flag and top-left position and restore it in the session path; verified by `toolsFloatRestores`.

## 3. Unified layout restore

- [x] 3.1 Add a `persist` parameter to `regroupAndApply()`, save/restore the previous `restoringPanelSession_` value, and only save when `persist` is true and the guard was not already set; verified by `workspaceWithoutFloatsDocksEverything` and the existing workspace tests.
- [x] 3.2 Harvest group floats before whole-column overlays and close every live flyout before harvesting; verified by `workspaceRoundTripsFloatInsideFloat` and `flyoutGroupLeavesNoStaleOverlay`.
- [x] 3.3 Record a per-entry `PanelColumn*` map in `buildInto()` and detach stored floating groups and whole columns after the docked layout is built; verified by `workspaceRoundTripsGroupFloat` and `workspaceRoundTripsColumnFloat`.
- [x] 3.4 Route non-empty session `panelColumns` through `regroupAndApply(..., persist=false)` and normalize missing legacy width/rail-mode fields; verified by `restartRestoresFloats` and `legacyStoreDocksEverything`.
- [x] 3.5 Re-apply `autoCollapseIconic`/`autoShowHidden` to every docked and floating column after the session rebuild; verified by `restartRestoresAutoFlags`.
- [x] 3.6 Keep the empty-layout legacy path (`panelGroups`) unchanged; verified by the existing `applyPanelSessionForTest(SessionState{})` reset behavior.
- [x] 3.7 Restore overlay geometry after the frame's stored window geometry and first show: defer float specs built while the frame is hidden, apply them from `showEvent`, and snapshot the active workspace afterwards; verified by `startupChildFloatUsesFrameGeometry` and `restartRestoresFloatAtRightOfRestoredFrame`.
- [x] 3.8 Verify `workspaceSwitchRemembersFloats` after switching away and back.

## 4. Persistent flyout and lifecycle

- [x] 4.1 Change `PanelFlyout` to the platform-adaptive frameless overlay host, activate it after showing, and branch `placeFlyout()` between global and frame-relative coordinates; verified by `flyoutSurvivesOutsideClick`, `flyoutChildModeStaysInFrame`, and `ctrlZWorksWhileHistoryIconFlyoutIsOpen`.
- [x] 4.2 Add same-icon toggle and different-icon replacement in `openIconFlyout()`; verified by `flyoutTogglesAndSwitches`.
- [x] 4.3 Add the flyout-only chevrons-right close button to `PanelGroup`'s header corner, the `flyoutCloseRequested` signal, and the column wiring; give it a matching icon size, tooltip, and auto-raise; call `setFlyout(true)` on entering the flyout and `setFlyout(false)` on every exit; update `headerCornerWidthForTest()`; verified by `flyoutCloseButtonPlacement`, `flyoutCloseButtonHidesFlyout`, and `flyoutCloseButtonOnlyInFlyout`.
- [x] 4.4 Hide the floating top bar and float close control while a group from a floating icon row is in the flyout, and restore them on return; verified by `floatingIconRowShowsOneHeader`.
- [x] 4.5 Close the flyout when its hosted group is dragged out, its last visible panel/group is closed, its owning column is removed, or all panels are hidden; delete the frame-parented flyout in `PanelColumn::~PanelColumn()`; verified by `flyoutGroupLeavesNoStaleOverlay` and `flyoutClosesOnHideAll`.
- [x] 4.6 Verify `escapeDoesNotCloseFlyout` and `floatingIconRowOpensFlyout` pass.
- [x] 4.7 Update the stale comments that call the flyout a popup host and say the float close no longer lives in the corner; verified by review of `panel_column_iconic.cpp` and `panel_group_test.cpp`.

## 5. Flyout width resize

- [x] 5.1 Add the canvas-facing edge grip to `PanelFlyout`, reusing the float grip pattern, position it on resize, and lay the flyout out using its actual size; verified by `flyoutResizesFromLeftEdge`, `flyoutResizesFromRightEdge`, and `flyoutCloseButtonPlacement`.
- [x] 5.2 Anchor the opposite edge, enforce the shared minimum width, apply it before the first resize, and clamp child-mode geometry to the frame; verified by `flyoutResizeKeepsMinimum`, `flyoutOpensAtMinimumWidth`, `flyoutResizesBesideFloatingRow`, and `flyoutChildModeStaysInFrame`.

## 6. Chrome and height fixes

- [x] 6.1 Remove the corner menu button's `QGraphicsOpacityEffect` and verify `floatCornerMenuPainted` samples the whole float, not the button alone.
- [x] 6.2 Add the `QWidget#panelFloatHeader` theme rule and `WA_StyledBackground` on the float top bar; verified by `floatHeaderMatchesDockedHeader`.
- [x] 6.3 Skip `syncToContent()` for an already-iconic widget column in `floatColumn()`; verified by `iconicColumnFloatKeepsTwoThirdsHeight`.
- [x] 6.4 Keep the rail toggle's `syncToContent()` and verify `iconicColumnSnapsWhenRailToggled` passes.
- [x] 6.5 Verify `normalColumnFloatKeepsTwoThirdsHeight` passes.

## 7. Integration and manual checks

- [x] 7.1 After all code is written, run `cmake --build build --parallel` once and fix any compile errors.
- [x] 7.2 Run `ctest --test-dir build -R '^tst_(floating_workspaces|workspaces|edit_shortcuts|command_tree|session_store|panels_parity|icon_assets)$' --output-on-failure`; verify all affected suites pass.
- [x] 7.3 Run `./build/pictura --headless --self-test` and verify no existing shell self-test regresses.
- [x] 7.4 Run `python3 scripts/check-test-plan.py` and verify every referenced test case exists and every scenario has a row.
- [x] 7.5 Run `openspec validate --all --strict` and verify the change and every capability validate.
- [ ] 7.6 Manual: tear off a normal column and an already-iconic column, confirm the initial height feels like roughly two thirds, then toggle the rail while floating and confirm the snap; record the result in the PR.
- [ ] 7.7 Manual: open a flyout from a docked icon strip and from a floating icon row, click the workspace, press Escape, use the close control, re-click the icon, drag the hosted group out, and press `Tab`; confirm the documented close paths, one header, and one-flyout-per-column behavior.
- [ ] 7.8 Manual: float the Tools column, quit, relaunch, and confirm its position and 1/2-column state; then apply a workspace and confirm Tools is untouched.
