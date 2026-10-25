# Test Plan

Every scenario in this change is listed once. The Coverage column names the
automated check that will exist after the apply phase; rows that cannot be
automated yet say so and give the reason.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `qs_union_one_state` | Drag Quick Selection across two disjoint regions; release | Both regions are selected and the history gains exactly one "Selection" state | `tst_tool_commit::quickSelectionOneStatePerStroke` |
| `qs_new_to_add` | Complete a New-mode Quick Selection stroke, then start a second stroke over another region | The tool's mode reads Add and the second stroke adds instead of replacing | `tst_tool_commit::quickSelectionSecondStrokeAdds` |
| `qs_move_inside_noop` | Set New, press one Quick Selection dab and keep the stroke open, emit many moves inside the selected region, then release | During the moves the selection, history, and `PictureView::changed` count stay unchanged; the release records one state | `tst_tool_commit::quickSelectionMoveInsideRegionIsNoop` |
| `qs_mode_isolation` | Complete a New-mode Quick Selection stroke; switch to the Rectangular Marquee; drag a new rectangle outside the existing selection | The marquee replaces the selection and its mode still reads New; the Quick Selection page shows Add (manual check below) | `tst_tool_commit::quickSelectionKeepsMarqueeModeIndependent` |
| `qs_cancel_restores` | Press Quick Selection, move, then press Escape | The pre-stroke selection and history are restored and no state is recorded | `tst_tool_commit::quickSelectionCancelRestores` |
| `committed_outline_border` | Make a selection touching the canvas edge; set several fractional offsets and zooms with the border in view; grab the canvas; scan each of the four border runs | A dark dash pixel is present within one device pixel of each border at every sample; no sample is empty | `tst_selection_preview::committedOutlineStaysVisibleAtBorderAcrossPanAndZoom` |
| `committed_outline_trace` | Capture `f.canvas->selectionContours()` and `selection_api::contour(*f.view).loops`; apply the fractional pan/zoom samples; capture both again | The stored loops the painter draws equal the traced contour at every sample and are unchanged by pan/zoom; the clip adds no segment | `tst_selection_preview::committedOutlineStaysVisibleAtBorderAcrossPanAndZoom` (stored loops vs contour), `tst_selection_api::typedContourMatchesTheLegacyString` (encoding unchanged) |
| `sel_menu_opens` | For each of the seven selection tools with an active selection, right-click over the document | A menu opens at the pointer with Deselect, Inverse, Feather…, Refine Edge… | `tst_canvas_context_menus::selectionMenuOpensForEverySelectionTool` |
| `sel_menu_commands` | Open two documents; right-click the non-active one; trigger each menu entry and accept the Feather and Refine Edge dialogs | The command acts on the clicked document, and the other document is unchanged | `tst_canvas_context_menus::selectionMenuRunsCommandsOnClickedDocument` |
| `sel_menu_no_selection` | With a selection tool and no selection, right-click over the document and over the pasteboard | No selection menu opens over the document; the pasteboard route returns false and the canvas-background menu is available | `tst_canvas_context_menus::selectionMenuNeedsASelection` |

## Manual checks for the PR body

- **#914 latency (measured, not asserted):** run the ignored Quick Selection
  profiler at 4000×4000 and record the composite, flood, combine, and contour
  times, plus the first-dab, intra-region-move, and new-region-move totals.
  Targets: first dab ≤ 250 ms, intra-region move ≤ 16 ms, new-region move
  ≤ 250 ms.
- **#913 options page (visual):** after a New-mode Quick Selection dab, the
  Quick Selection page's Add button is checked and the Marquee page's New button
  is still checked.
