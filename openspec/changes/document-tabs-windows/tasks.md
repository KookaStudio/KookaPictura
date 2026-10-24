# Tasks

Each group is one commit, in the order below, and each is meant to be green on
its own. Per the repo workflow, nothing is built or run until group 11: every
"Verify" below names the check that the group 11 run must pass, and a failure
there is fixed in the commit of the group that owns it. Each group writes its
tests before its code.

## 1. Full-label tab tooltip (#917)

- [x] 1.1 Write `tst_loading_tab::tooltipShowsFullUntitledLabel` (tooltip equals the full label and differs from the elided tab text) and `tst_loading_tab::tooltipShowsFullSavedLabel` (tooltip carries the zoom and mode/bits suffix, not the path).
- [x] 1.2 Keep the unelided label in `updateTabTitle` and use it as the tooltip (no path, no marker). Verify the two tests and the existing `tst_loading_tab::longTitlesKeepTheCloseButton`.

## 2. Window registry refactor, with no behavior change

- [x] 2.1 Add `document_window.{h,cpp}` with `DocWindow`, `Host`, and `fullLabel()`. Move the tab-title code out of `frame.cpp`. Register the files in the root `CMakeLists.txt`. Verify `frame.cpp` shrinks and `scripts/check-file-size.sh` passes.
- [x] 2.2 Split `DocEntry` into per-document fields plus `windows`. Add `windows_`, `activeWindow_`, `windowAtTab()`, `tabOf()`, and `onActiveWindowChanged()` (today's `currentChanged` body). Route `activeDocumentIndex`, `setActiveDocumentIndex`, `canvasAt`, the `tabMoved` handler, and every direct `tabs_` index use through them. Verify `grep -n "tabs_->currentIndex\|tabs_->widget(" crates/pictura-app/cpp/frame*.cpp` hits only the helpers.
- [x] 2.3 Add `documentArea_` (a `QStackedWidget` holding only `tabs_` for now) as the splitter's stretch pane. Move the `indexOf(tabs_)` / stretch checks in `frame_columns.cpp`, the empty-workspace filter in `frame_build.cpp`, and the search-bar host in `frame_search.cpp` to it. Verify `tst_floating_workspaces`, `tst_workspaces`, and `tst_loading_tab::firstOpenShowsEmptyWorkspace`.
- [x] 2.4 Keep `reorderDocumentsForTest` aligned with `windows_`. Verify the self-test's `reorderDocumentsForTest` check (`selftest.cpp:880`) and `tst_smoke`, `tst_loading_tab`, `tst_open_save`, and `tst_recovery` pass unchanged.

## 3. Ctrl+Tab cycling (#499)

- [x] 3.1 Create `tst_document_windows.cpp` with a fresh window per test function and add it to `PICTURA_QT_TESTS` in `crates/pictura-app/cpp/tests/CMakeLists.txt`. Write `openAddsTab`, `switchingTabsChangesCanvas`, `commandsTargetActiveDocument`, `noDocumentsDisablesCommands`, `closeDirtyCancelKeepsTab`, `ctrlTabCyclesTabs`, `ctrlTabWraps`, `ctrlTabLeavesPanels`, `ctrlTabFromTextField`, `ctrlTabBlockedByModal`, `ctrlTabSingleWindowNoop`, `ctrlTabTypeSessionMatchesTabSwitch`, and `cycleSequencesAreReserved`.
- [x] 3.2 Add `frame_document_windows.cpp` (registered in the root `CMakeLists.txt`) with the frame-level `Ctrl+Tab` / `Ctrl+Shift+Tab` `WindowShortcut`s stepping `windows_`. Add both sequences to `workspaceKeys()`. Verify the tests above and `tst_keyboard_shortcuts`.

## 4. Float primitive, tear-out/dock, and context menus (#317)

- [x] 4.1 Create `tst_document_float.cpp` and register it like 3.1. Write `hostMoveKeepsView`, `inactiveFloatRepaints`, `tearOutFloats`, `tearOutLastTab`, `sidewaysDragReorders`, `dragHeaderDocks`, `floatHeaderShowsMarker`, `floatHostingFollowsPlatform`, `floatCloseClosesDocument`, `headerMenuEntries`, `headerMenuTargetsItsWindow`, `undoShortcutInFloat`, `menuShortcutInFloat`, `toolShortcutInFloat`, `frameKeyInFloat`, `fullScreenShowsOnlyActive`, `fullScreenFloatFillsArea`, `standardModeRestoresLayout`, `tabKeepsFloatingDocuments`, and `workspaceSwitchKeepsFloats`. In `tst_document_windows`, write `floatingWindowBecomesTarget`, `floatActivatedDuringLoadKeepsFocus`, `ctrlTabReachesFloat`, `tabMenuEntries`, `tabMenuActsOnClickedTab`, `closeOthersDisabledWhenAlone`, `closeOthersKeepsClicked`, `revealDisabledWithoutPath`, `revealOpensContainingFolder`, `moveDisabledForLoadingTab`, and `consolidateAllToHere`.
- [x] 4.2 Add `document_float_window.{h,cpp}` (in `CMakeLists.txt`, with its moc include): platform-adaptive hosting through `PanelFloat::overlayUsesTopLevel()` and its test hook, a header with the label, the marker, the close control, drag-move, the resize grip, and placement clamped to the document area. Add `floatWindow(DocWindow*)` / `dockWindow(DocWindow*, int index)`, which reparent the canvas host and refuse a loading document. Verify `hostMoveKeepsView`, `floatHostingFollowsPlatform`, and `floatCloseClosesDocument`.
- [x] 4.3 Route float activation into `onActiveWindowChanged()`. Make `refresh()` present every visible window whose frame revision moved. Verify `floatingWindowBecomesTarget`, `floatActivatedDuringLoadKeepsFocus`, `ctrlTabReachesFloat`, and `inactiveFloatRepaints`.
- [x] 4.4 Add the tear-out event filter on `documentTabBar` and the header drop-to-dock check. Verify `tearOutFloats`, `tearOutLastTab`, `sidewaysDragReorders`, and `dragHeaderDocks`.
- [x] 4.5 Add the tab context menu and the header context menu, both acting on the clicked window, and the replaceable URL opener for Reveal. Verify the tab and header menu tests.
- [x] 4.6 Make keys reach the floats: rely on the `Qt::Tool` shortcut rule (D4; fall back to `addActions` if needed) and forward unhandled key press/release events from a top-level float to the frame. Check the frame's focus and Space-pan guards with a float active. Verify `undoShortcutInFloat`, `menuShortcutInFloat`, `toolShortcutInFloat`, and `frameKeyInFloat` in top-level mode.
- [x] 4.7 Add `closeWindow()` for the tab, header, and `File > Close` paths; `Close All` and quit iterate documents. Verify `closeDirtyCancelKeepsTab` and `tst_loading_tab::closeButtonClosesTheDocument`.
- [x] 4.8 Hide all but the active float on entering a full-screen mode, show an active float's canvas host in the screen-mode canvas area without its header, and restore them on return to Standard; keep floats out of `Tab` / `Shift+Tab` and workspace switching. Verify `fullScreenShowsOnlyActive`, `fullScreenFloatFillsArea`, `standardModeRestoresLayout`, `tabKeepsFloatingDocuments`, `workspaceSwitchKeepsFloats`, and `tst_screen_modes`.

## 5. Arrange: float, consolidate, cascade, match (#500)

- [x] 5.1 Create `tst_document_arrange.cpp` and register it like 3.1. Write `arrangeFloatInWindow`, `arrangeFloatAllThenConsolidate`, `arrangeDisabledWithoutDocuments`, `cascadeStepsWindows`, `cascadeKeepsHeadersReachable`, `matchZoom`, `matchLocation`, and `matchAll`.
- [x] 5.2 Wire the handlers and enabled-providers for Float in Window, Float All in Windows, Consolidate All to Tabs, Cascade, and Match Zoom/Location/All. Flip those seven catalogue entries to `implemented: true`, update their rows in `crates/pictura-script/tests/golden/catalogue.tsv` and the implemented count in `crates/pictura-script/tests/catalogue.rs`, and add them to `NON_RECORDABLE` in `crates/pictura-script/src/command.rs`. Verify the tests above, `cargo nextest run -p pictura-script`, and `tst_command_tree`.
- [x] 5.3 Implement Match Location with `ImageView::widgetToImage` and `centerOnImageRect`. Verify `matchLocation` and `matchAll`.

## 6. Interface preferences

- [x] 6.1 In `tst_document_arrange`, write `prefOpensFloating`, `prefFloatsOpenedFileAfterLoad`, `prefDisablesDocking`, `prefsPersist`, `prefsDefaultOn`, and `recoveryReopensAsTabs`.
- [x] 6.2 Add the `openDocumentsAsTabs` and `enableFloatingDocumentDocking` checkboxes to the Interface page, their `SessionState` fields (`session.h`, `session.cpp`, missing key loads `true`), and their wiring in `frame_preferences.cpp` and `frame_session.cpp`. Make the frame honor them when a ready document's window is hosted and in the dock-drop check, and make crash recovery always host as a tab. Verify the six tests above and `tst_recovery`.

## 7. Tile

- [x] 7.1 Write `tileFourInGrid`, `tileTakesInFloats`, `openWhileTiledAddsPane`, `openWhileTiledPrefOffAddsPane`, `closingPanesRetiles`, and `consolidateEndsTiling`.
- [x] 7.2 Add `document_tile_host.{h,cpp}` (in `CMakeLists.txt`, with its moc include): pane header plus canvas host in a `ceil(sqrt n)` grid, inside `documentArea_`. Route new ready documents into the tile host while tiled, whatever the tabs preference, and end tiling when the last pane closes. Flip `window.arrange.tile` (catalogue, golden row, count, `NON_RECORDABLE`). Verify the five tests and `cargo nextest run -p pictura-script`.

## 8. New Window For [file]

- [x] 8.1 Write `newWindowSharesDocument`, `newWindowFollowsPreference`, `newWindowHasOwnViewport`, `newWindowLabelFollowsActive`, `newWindowOrdinalsRenumber`, `statusZoomFollowsActiveWindow`, `closingExtraWindowDoesNotPrompt`, `fileCloseClosesActiveWindow`, and `closeAllPromptsOncePerDocument` in `tst_document_arrange`.
- [x] 8.2 Make `setupCanvas` / `wireCanvas` per-window and make the frame-setting loops iterate windows. Classify every `.canvas`, `canvasAt()`, and `imageView()` site in `frame*.cpp` as document-wide or active-window, and route the document-wide ones through `forEachCanvas`. Verify `newWindowSharesDocument` (paint, filter, mask overlay).
- [x] 8.3 Implement the command (a new window on a fresh `ImageView` bound to the document, hosted as a tab, float, or tile pane per the preference and tile state), the `:N` label ordinal, and the `setLabelProvider` menu label. Flip `window.arrange.new.window.for.file` (catalogue, golden row, count, `NON_RECORDABLE`). Verify the remaining 8.1 tests and `cargo nextest run -p pictura-script`.

## 9. Multi-window Hand and Zoom options

- [x] 9.1 Create `tst_document_navigate.cpp` and register it like 3.1. Write `scrollAllWindows`, `shiftScrollsAllWindows`, `plainDragScrollsOneWindow`, `zoomAllWindows`, `shiftClickZoomsAllWindows`, `resizeWindowsToFit`, `zoomResizesWindowsPref`, and `handZoomOptionsPersist`.
- [x] 9.2 Add `options_bar_navigate.cpp` (in `CMakeLists.txt`) with Hand and Zoom pages holding the three checkboxes, their `ToolOptions` fields and persistence, and the `Zoom Resizes Windows` General preference (`SessionState`, missing key reads off). Verify `handZoomOptionsPersist` and `tst_options_bars_parity`.
- [x] 9.3 Fan out Hand pans (with a re-entrancy guard) and Zoom-tool factors to the other windows when the option is on or `Shift` is held. Verify `scrollAllWindows`, `shiftScrollsAllWindows`, `plainDragScrollsOneWindow`, `zoomAllWindows`, and `shiftClickZoomsAllWindows`.
- [x] 9.4 Resize an active floating window to fit after a Zoom-tool zoom, and after a View zoom when `Zoom Resizes Windows` is on, clamped to the document area. Verify `resizeWindowsToFit` and `zoomResizesWindowsPref`.

## 10. Roadmap and docs

- [x] 10.1 In `ROADMAP.md` (repository root, no docs marker needed), tick the `Ctrl+Tab` (#499) and `Window ▸ Arrange` (#500) items and add ticked items for tab tear-out and the tab/header menus (#317) and the full-label tooltip (#917). Verify `bash scripts/guard.sh` rule 13.
- [x] 10.2 In one `docs:` commit carrying `TASK-ALLOWS-DOCS`: remove the closed rows UI-3 and UI-4 from `docs/dev/doc-code-parity-gaps.md` and update its row count; add `DIVERGENT` rows for the Wayland in-frame float (`application-frame.md:241`, `:297`) and for the unsaved float layout (`application-frame.md:221`, `:285`, `:337`), and a `STALE-DOC` row for the "Enable Floating Document Window" label (`preferences.md:108`, `application-frame.md:71`); record the milestone in `docs/dev/STATE.md`. Verify `GUARD_BASE=master bash scripts/guard.sh`.

## 11. Integration

- [ ] 11.1 Build once (`cmake --build build --parallel`, ≤80% CPUs), then run `bash scripts/verify-full.sh`. Verify fmt, clippy, the test report (including the four new suites and the self-test), file-size, the self-test budget, guard, openspec, and `python3 scripts/check-test-plan.py` all pass.
- [ ] 11.2 Archive the change (`openspec archive document-tabs-windows`) and confirm `scripts/check-openspec-archived.sh` passes. Open the PR, listing the test plan's manual checks as the reviewer checklist.
