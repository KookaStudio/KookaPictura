# Test Plan

Every scenario in the change's two deltas maps to an automated Qt Test case. The
ui-scaling cases already exist and stay as they are; only the `document-windows`
cases are new.

## document/document-windows

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `dtw_menu_list_order` | Name the documents with `setDocumentNameForTest`, move `B` before `A` in the strip, float `C`, and emit the Window menu's `aboutToShow`; then trigger `Tile` and emit again | The rows are `fullLabel()` for `B`, `A`, `C` in that order, tabs first; after tiling the same cycle order | `tst_document_windows::windowMenuListOrder` |
| `dtw_menu_list_label` | Open one document named `A&B`, choose `New Window For` so it has two windows, and open the menu | The rows equal `fullLabel()`/`windowLabelForTest()`; the second window is active and its checked row's name part ends in `:2`; the `A&B` row doubles the ampersand | `tst_document_windows::windowMenuFullLabels` |
| `dtw_menu_list_active` | Open two documents, emit `aboutToShow` and inspect, press `Ctrl+Tab`, then emit `aboutToShow` again | Only the active window's row is checked, and the check moves after the switch | `tst_document_windows::windowMenuChecksActive` |
| `dtw_menu_list_activate` | Open a tab window and a floating window, trigger their rows in turn, then trigger `Tile` and trigger a pane's row | The triggered window becomes active in its host | `tst_document_windows::windowMenuActivatesHosts` |
| `dtw_menu_list_updates` | Emit `aboutToShow`, rename one document, open another, float a third, close a fourth, then emit `aboutToShow` again | The renamed row has the new name, the opened document has a row, the floated row follows the tabs, and the closed document has no row | `tst_document_windows::windowMenuFollowsChanges` |
| `dtw_menu_list_last` | Open one document, emit `aboutToShow`, inspect the visible actions after the `uiSizeControl` row | The separator and document rows are visible after the `uiSizeControl` row as the last visible actions, below the UI Size row (which is below Options) | `tst_document_windows::windowMenuListIsLast` |
| `dtw_menu_list_empty` | With no document open, emit `aboutToShow` | No `windowDocumentEntry` exists and every action after `uiSizeControl` is invisible | `tst_document_windows::windowMenuEmptyWithoutDocuments` |

## ui/ui-scaling

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `uis_menu_single_row` | Open the Window menu | One UI Size row with caption, step row, and reset row; no separate Enlarge/Decrease rows | `tst_ui_scale::windowMenuSingleControl` |
| `uis_menu_below_options` | Open the Window menu with no document open | The UI Size row is below Options and separated from it; no document rows or list separator follow | `tst_ui_scale::windowMenuControlSitsBelowOptions` |
| `uis_menu_resize` | Change the scale while the Window menu is open | The menu grows to fit its rows without clipping | `tst_ui_scale::windowMenuResizesWithScale` |
| `uis_menu_buttons` | Press the plus button twice while the menu is open | The scale advances two steps and the menu stays open | `tst_ui_scale::windowMenuSingleControl` |
| `uis_menu_step` | Run Enlarge at 100 percent and Decrease at 90 percent | The scale becomes 110 and 80 percent | `tst_ui_scale::windowMenuStepsAndBounds` |
| `uis_menu_bounds` | Set the scale to 75, 200, and 100 percent | Decrease, Enlarge, and Reset are disabled respectively | `tst_ui_scale::windowMenuStepsAndBounds` |
| `uis_menu_readout` | Set the scale to 130 percent and open the menu | The readout says `UI Size: 130%` and the row's label shows 130 percent | `tst_ui_scale::windowMenuReadout` |
