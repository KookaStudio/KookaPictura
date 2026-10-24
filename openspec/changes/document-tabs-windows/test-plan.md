# Test plan

All new GUI checks are Qt Test cases, run on the offscreen platform. No check is
added to the self-test. The new cases are split over three suites so that each
file stays under the 1400-line test cap:

- `tst_document_windows`: targeting, closing, `Ctrl+Tab`, and the tab menu.
- `tst_document_float`: floating windows, tear-out and docking, the header menu,
  keys in floats, and screen modes.
- `tst_document_arrange`: `Window > Arrange`, Tile, Match, New Window For, the
  Interface preferences, recovery, and Close All.
- `tst_document_navigate`: the multi-window Hand and Zoom options.

Each suite builds a fresh window per test function (`init()` / `cleanup()`), so
a suite can be sharded. Geometry checks force in-frame float hosting through
`PanelFloat::setForceChildOverlayForTest`. The checks named "top-level" run
with that hook off and wait for the float with `QTest::qWaitForWindowActive`,
so that `QApplication::activeWindow()` is the `Qt::Tool` window itself.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `Opening a document adds a tab` | One document open, tabs preference on; open a second | Two tabs, each canvas shows its own document | `tst_document_windows::openAddsTab` |
| `Switching tabs changes the canvas` | Two tabs; select the first | `imageView()` and the status bar size are the first document's | `tst_document_windows::switchingTabsChangesCanvas` |
| `Commands target the active document` | Open two documents, activate the second, run Image > Adjustments > Invert | Only the second document's pixels and history change | `tst_document_windows::commandsTargetActiveDocument` |
| `No documents open` | Open one document, close it | Document commands disabled; no crash | `tst_document_windows::noDocumentsDisablesCommands` |
| `dtw_target_float` | One tabbed and one floating document; activate the float | `activeView()` is the float's document; the status bar shows its size | `tst_document_windows::floatingWindowBecomesTarget` |
| `dtw_target_float_loading` | One float; start a held open; activate the float; release the open | The float stays active; the new tab is not current | `tst_document_windows::floatActivatedDuringLoadKeepsFocus` |
| `Close the active tab` | Two clean documents; close the active tab | One tab remains, other document open | `tst_loading_tab::closeButtonClosesTheDocument` |
| `Close with unsaved changes` | Dirty a document; close its tab; answer Cancel to the stubbed prompt | Tab still open | `tst_document_windows::closeDirtyCancelKeepsTab` |
| `dtw_close_extra_window` | Dirty doc, New Window For, close one window with its close control (prompt stub counts) | No prompt; other window open; doc still dirty | `tst_document_arrange::closingExtraWindowDoesNotPrompt` |
| `dtw_file_close_window` | Dirty doc with two windows; activate one; File > Close (prompt stub counts) | No prompt; one window left; doc open and dirty | `tst_document_arrange::fileCloseClosesActiveWindow` |
| `dtw_tooltip_untitled` | New untitled doc whose label is wider than the tab text width | Tab text elided; tooltip == full label; tooltip != tab text | `tst_loading_tab::tooltipShowsFullUntitledLabel` |
| `dtw_tooltip_saved` | Open a saved file with a long name | Tooltip == full label with ` @ N%` and `(Mode/Bits)`; not the path | `tst_loading_tab::tooltipShowsFullSavedLabel` |
| `dtw_cycle_tabs` | Three tabs, first active, focus on tab bar; Ctrl+Tab ×2, then Shift+Ctrl+Tab sent both as `Key_Tab` and as `Key_Backtab` | Active goes 1→2→3→2 | `tst_document_windows::ctrlTabCyclesTabs` |
| `dtw_cycle_wrap` | Last window active; Ctrl+Tab | First window active | `tst_document_windows::ctrlTabWraps` |
| `dtw_cycle_float` | One tab, one float; tab active; Ctrl+Tab | Float active | `tst_document_windows::ctrlTabReachesFloat` |
| `dtw_cycle_panels` | Panels shown; Ctrl+Tab | Panels still shown | `tst_document_windows::ctrlTabLeavesPanels` |
| `dtw_cycle_text_focus` | Focus a panel line edit; Ctrl+Tab | Next window active | `tst_document_windows::ctrlTabFromTextField` |
| `dtw_cycle_modal` | Two docs; open a modal QDialog; Ctrl+Tab sent to it | Active document unchanged | `tst_document_windows::ctrlTabBlockedByModal` |
| `dtw_cycle_type_session` | Two tabs; start a type edit in A; Ctrl+Tab; compare with a second run that clicks B's tab | Session state, layer text, and history are identical in both runs | `tst_document_windows::ctrlTabTypeSessionMatchesTabSwitch` |
| `dtw_cycle_single` | One doc; Ctrl+Tab | Same window active; panels and tool unchanged | `tst_document_windows::ctrlTabSingleWindowNoop` |
| `dtw_cycle_reserved` | Read `workspaceKeys()` | Contains Ctrl+Tab and Ctrl+Shift+Tab | `tst_document_windows::cycleSequencesAreReserved` |
| `dtw_tab_menu_entries` | Request the tab context menu | Entry texts and separators in order | `tst_document_windows::tabMenuEntries` |
| `dtw_tab_menu_target` | Second tab active; Move to New Window on first tab | First floats; second tabbed | `tst_document_windows::tabMenuActsOnClickedTab` |
| `dtw_tab_menu_close_others` | One document; tab menu | Close Others disabled | `tst_document_windows::closeOthersDisabledWhenAlone` |
| `dtw_tab_menu_close_others_run` | Three clean tabs; Close Others on second | Only second remains | `tst_document_windows::closeOthersKeepsClicked` |
| `dtw_tab_menu_reveal` | Untitled doc; tab menu | Reveal in File Manager disabled | `tst_document_windows::revealDisabledWithoutPath` |
| `dtw_tab_menu_reveal_run` | Saved doc; Reveal in File Manager with the URL-opener test hook | Hook receives the file's parent folder URL | `tst_document_windows::revealOpensContainingFolder` |
| `dtw_tab_menu_loading` | Held open; tab menu on the loading tab | Move to New Window disabled | `tst_document_windows::moveDisabledForLoadingTab` |
| `dtw_tab_menu_consolidate` | Two floats + one tab; Consolidate All to Here on the tab | Three tabs; clicked tab active | `tst_document_windows::consolidateAllToHere` |
| `dtw_host_move_keeps_view` | Zoom 200%, scroll; float; dock | Zoom, scroll centre, history count unchanged | `tst_document_float::hostMoveKeepsView` |
| `dtw_status_zoom` | Two windows of one doc at 50% and 200%; activate each | Status magnification 50% then 200% | `tst_document_arrange::statusZoomFollowsActiveWindow` |
| `dtw_inactive_repaint` | Float `A`, tab `B` active; change `A` through its view without activating it | `A`'s float canvas frame revision matches `A`'s | `tst_document_float::inactiveFloatRepaints` |
| `dtw_tear_out` | Two tabs; QTest drag second tab 120 px below the bar | Second floats and is active; first tabbed | `tst_document_float::tearOutFloats` |
| `dtw_tear_out_last` | One tab; drag it below the bar | Document floats; tab count 0 | `tst_document_float::tearOutLastTab` |
| `dtw_tear_reorder` | Drag a tab sideways within the bar | Order changed; no float | `tst_document_float::sidewaysDragReorders` |
| `dtw_dock_back` | Drag a float header over the tab bar and release | Tab at drop index; active | `tst_document_float::dragHeaderDocks` |
| `dtw_float_header_label` | Float a doc; edit it | Header text == label followed by the modified marker | `tst_document_float::floatHeaderShowsMarker` |
| `dtw_float_hosting` | Float with child mode forced, then without | Child overlay of the frame vs. `isWindow()` top-level | `tst_document_float::floatHostingFollowsPlatform` |
| `dtw_float_close` | Only doc floats, clean; header close | Float and doc gone; document count 0; double-click-to-open armed | `tst_document_float::floatCloseClosesDocument` |
| `dtw_header_menu_entries` | Request a float's header menu | Entries in order | `tst_document_float::headerMenuEntries` |
| `dtw_header_menu_target` | Tab active; Image Size from float header menu (dialog auto-rejected) | Float active; dialog opened for its document | `tst_document_float::headerMenuTargetsItsWindow` |
| `dtw_float_shortcut` | Top-level float focused, one edit; Ctrl+Z | Edit undone on the float's document | `tst_document_float::undoShortcutInFloat` |
| `dtw_float_menu_shortcut` | Top-level float active (`qWaitForWindowActive`); Ctrl+A | Float's document has a full selection; the tabbed doc's selection unchanged | `tst_document_float::menuShortcutInFloat` |
| `dtw_float_tool_shortcut` | Top-level float active; press B | Brush tool active | `tst_document_float::toolShortcutInFloat` |
| `dtw_float_frame_key` | Top-level float active; Brush tool; press `]` | Brush size larger than before | `tst_document_float::frameKeyInFloat` |
| `dtw_fullscreen_active` | One tab, two floats, one float active; enter Full Screen With Menu Bar (advisory suppressed) | Only the active float visible | `tst_document_float::fullScreenShowsOnlyActive` |
| `dtw_fullscreen_float_fills` | Float active; enter Full Screen Mode (advisory suppressed) | Float canvas fills the canvas area; header hidden; no other window visible | `tst_document_float::fullScreenFloatFillsArea` |
| `dtw_fullscreen_restore` | Then return to Standard | Every float visible at its old geometry; tab present | `tst_document_float::standardModeRestoresLayout` |
| `dtw_tab_keeps_floats` | One float; Tab | Panels hidden; float visible | `tst_document_float::tabKeepsFloatingDocuments` |
| `dtw_workspace_keeps_floats` | One float; switch workspace | Float visible at the same geometry, not docked | `tst_document_float::workspaceSwitchKeepsFloats` |
| `dtw_arrange_float` | Two tabs; Float in Window | Active floats; other tabbed | `tst_document_arrange::arrangeFloatInWindow` |
| `dtw_arrange_float_all` | Three tabs; Float All; Consolidate All to Tabs | All float; then all tabbed, same active doc | `tst_document_arrange::arrangeFloatAllThenConsolidate` |
| `dtw_arrange_disabled` | No documents | All nine `window.arrange.*` disabled | `tst_document_arrange::arrangeDisabledWithoutDocuments` |
| `dtw_cascade` | Three tabs, child hosting; Cascade | Three equal-size floats; equal positive x/y step | `tst_document_arrange::cascadeStepsWindows` |
| `dtw_cascade_reachable` | Small frame, twelve docs, child hosting; Cascade | Every float header inside the document area | `tst_document_arrange::cascadeKeepsHeadersReachable` |
| `dtw_tile_grid` | Four docs; Tile | 2×2 panes filling the area; tab strip hidden | `tst_document_arrange::tileFourInGrid` |
| `dtw_tile_floats` | One tab, one float; Tile | Two panes; no float | `tst_document_arrange::tileTakesInFloats` |
| `dtw_tile_open` | Two tiled; tabs preference on; New document | Three panes; new one active | `tst_document_arrange::openWhileTiledAddsPane` |
| `dtw_tile_close` | Two clean docs tiled; close one pane, then the other | One pane fills the area; then empty workspace, tile host gone | `tst_document_arrange::closingPanesRetiles` |
| `dtw_tile_consolidate` | Tiled; Consolidate All to Tabs | All tabbed; strip shown | `tst_document_arrange::consolidateEndsTiling` |
| `dtw_match_zoom` | Active 300%, other 50%; Match Zoom | Both 300% | `tst_document_arrange::matchZoom` |
| `dtw_match_location` | Active centre at (0.25, 0.75); Match Location | Other centre at (0.25, 0.75) ±1 px | `tst_document_arrange::matchLocation` |
| `dtw_match_all` | Different zoom and centre; Match All | Both match | `tst_document_arrange::matchAll` |
| `dtw_tile_open_pref_off` | Two tiled; tabs preference off; New document | Three panes; new one active; no float exists | `tst_document_arrange::openWhileTiledPrefOffAddsPane` |
| `dtw_new_window_host` | New Window For with tabs preference on, then off | First new window is a tab; second is a float | `tst_document_arrange::newWindowFollowsPreference` |
| `dtw_new_window` | New Window For; paint, run a filter, toggle a mask overlay in the new window; undo | Both canvases' images equal after each step; label ends `:2`; one undo clears both | `tst_document_arrange::newWindowSharesDocument` |
| `dtw_new_window_viewport` | Zoom second window | First window's zoom unchanged | `tst_document_arrange::newWindowHasOwnViewport` |
| `dtw_new_window_label` | Activate `b.png` | Menu text `New Window For b.png` | `tst_document_arrange::newWindowLabelFollowsActive` |
| `dtw_new_window_renumber` | Three windows of one doc; close the second | Labels: no suffix, `:2` | `tst_document_arrange::newWindowOrdinalsRenumber` |
| `dtw_pref_open_floating` | Pref off; new document | Opens floating | `tst_document_arrange::prefOpensFloating` |
| `dtw_pref_open_floating_loaded` | Pref off; held open of a file | Loading tab while held; floating once released | `tst_document_arrange::prefFloatsOpenedFileAfterLoad` |
| `dtw_pref_no_docking` | Docking off; drop float on bar; then Consolidate All to Tabs | Still floating after the drop; tabbed after the command | `tst_document_arrange::prefDisablesDocking` |
| `dtw_pref_persist` | Turn both off; save the session; load the store again | Both off | `tst_document_arrange::prefsPersist` |
| `dtw_pref_defaults` | Load a store without either key | Both on; other keys kept | `tst_document_arrange::prefsDefaultOn` |
| `dtw_recovery_tabs` | Tabs pref off; float a doc; write a recovery snapshot; recover | Recovered doc is a tab | `tst_document_arrange::recoveryReopensAsTabs` |
| `dtw_close_all_prompts` | Dirty doc with a tab and a float window; Close All with a counting prompt stub | Prompt count == 1 | `tst_document_arrange::closeAllPromptsOncePerDocument` |

| `dtw_scroll_all` | Two child-hosted floats; Scroll All on; Hand-drag 40 px right in one | Both views' offsets moved +40 px in x | `tst_document_navigate::scrollAllWindows` |
| `dtw_scroll_all_shift` | Option off; Shift+Hand-drag | Both views moved by the drag | `tst_document_navigate::shiftScrollsAllWindows` |
| `dtw_scroll_all_off` | Option off; plain Hand-drag | Only the dragged view moved | `tst_document_navigate::plainDragScrollsOneWindow` |
| `dtw_zoom_all` | Zoom All on; windows at 100% and 50%; Zoom click in the first | Second's zoom ratio new/old equals the first's | `tst_document_navigate::zoomAllWindows` |
| `dtw_zoom_all_shift` | Option off; Shift+click with Zoom | Both zooms change by the same factor | `tst_document_navigate::shiftClickZoomsAllWindows` |
| `dtw_resize_to_fit` | Resize Windows To Fit on; Zoom click in a child-hosted float, then in a tab | Float grows, stays inside the document area; tab size unchanged | `tst_document_navigate::resizeWindowsToFit` |
| `dtw_zoom_resizes_pref` | Zoom Resizes Windows on, Ctrl+= on a float; then off, Ctrl+= | Float resizes the first time only | `tst_document_navigate::zoomResizesWindowsPref` |
| `dtw_hand_zoom_options_persist` | Turn all four on; save and reload the session store | All four on | `tst_document_navigate::handZoomOptionsPersist` |

## Manual checks (reviewer checklist)

These need a real compositor or desktop. Offscreen QPA has no window manager
and no file manager, so the automated rows can't cover them:

- On X11: tear a tab out, move the float outside the frame, drag it back onto
  the tab strip. It docks at the drop position.
- On Wayland: the float is an in-frame overlay. Tear-out, drag, dock-back, and
  Cascade work inside the frame, and floating panels stay above it.
- With a float focused, menu shortcuts (Ctrl+S, Ctrl+Z), tool keys, `Space`
  panning, `[`/`]`, and Ctrl+Tab work on both platforms.
- Reveal in File Manager opens the containing folder in the desktop's file
  manager.
- Minimizing the frame hides its floating document windows, and restoring it
  shows them again.
