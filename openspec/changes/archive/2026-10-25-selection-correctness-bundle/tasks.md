# Tasks

## 1. Quick Selection option isolation (#913)

- [x] 1.1 Add `tst_tool_commit::quickSelectionKeepsMarqueeModeIndependent` (set both groups New, complete a New Quick Selection stroke, do not reset the marquee mode, then drag a new rectangle outside the existing selection; it must replace and the marquee group must still read New) and `tst_tool_commit::quickSelectionSecondStrokeAdds`; the isolation case is written to replace, so the shared group would make it add and fail
- [x] 1.2 Add a dedicated `SelectionOptions` instance for Quick Selection in `ToolController` and register it with the Quick Selection handler; the two tool ids now resolve different `SelectionOptions` objects
- [x] 1.3 Make `ToolController::resolveSelectionMode` read the active handler's `SelectionOptions` and fall back to the shared group when the handler has none; `tst_tool_commit::quickSelectionOneStatePerStroke` remains the post-stroke Add check
- [x] 1.4 Resolve the selection group per page in `OptionsBar` (every field, including the Quick Selection Tolerance spin), pass it into `addModeButtons`, and keep the shared group for Patch/Content-Aware Move pages; add `SelectionOptions::combineModeChanged` (notify on change only) and re-check the Quick Selection page's mode button with a `QPointer` guard; the isolation test and the options-page visual check cover it, and `options_bar.cpp` stays under the 1200-line cap
- [x] 1.5 Add a Quick Selection tolerance assertion to `tst_tool_options_persistence::optionsSurviveARestart`; the assertion reads the Quick Selection group, not the shared Marquee value
- [x] 1.6 Persist the Quick Selection group under a new `quickSelection` JSON key with the existing `SelectionFields` helpers; 1.5's assertion is the round-trip check
- [x] 1.8 Commit any numeric field whose text is still being edited before the close-time session snapshot; `tst_tool_options_persistence::quickSelectionToleranceFromOptionsBarPersists` drives the options bar and `quickSelectionToleranceTypedWithoutEnterPersists` is the regression for closing with the tolerance editor focused
- [x] 1.7 Run `cargo nextest run -p pictura_app` plus `ctest --test-dir build -R '^tst_(tool_commit|tool_options_persistence|selection_preview)$'` and verify every case passes (393 nextest cases passed; the CTest suites passed)

## 2. Quick Selection move cost (#914)

- [x] 2.1 Add an ignored Quick Selection profiler (following `tests_profiles.rs`) that times composite, flood, combine, and contour separately at 4000×4000 plus the first-dab, intra-region-move, and new-region totals; running it is task 2.5
- [x] 2.2 Add `tst_tool_commit::quickSelectionMoveInsideRegionIsNoop` (set New, press one dab and keep the stroke open, emit many moves inside the selected region; before release assert the selection, history, and `PictureView::changed` count do not grow; release records one state); it is written to fail on the current per-move recomputation
- [x] 2.3 Add an `is_move` argument to `quick_select_preview` and its bridge; the handler passes `ctx.dragCommitted()`. Return true before `quick_select_shape` when `is_move`, the mode is Add, the dab is in bounds, and the current coverage is 255; make no edit and emit no `PictureView::changed`. New, Subtract, Intersect, the first press, and partial coverage still flood; 2.2 and `quickSelectionCancelRestores` are the regression cases
- [x] 2.4 If the profiler misses first dab ≤ 250 ms, intra-region move ≤ 16 ms, or new-region move ≤ 250 ms, record the numbers, verify no cache code entered the change, and open a follow-up for a cached stroke composite; do not add that cache in this change (all targets met; no follow-up)
- [x] 2.5 Re-run the profiler and `tst_tool_commit`, verify they pass, and record the final numbers in the task result (composite 104.44 ms, flood 1.11 ms, combine 1.51 ms, contour 0.77 ms, first dab 107.83 ms, new region 114.96 ms, intra-region move 0.00 µs; `tst_tool_commit` passed)

## 3. Committed outline at the canvas border (#912)

- [x] 3.1 Add `tst_selection_preview::committedOutlineStaysVisibleAtBorderAcrossPanAndZoom` (edge-touching selection, several fractional offsets and zooms with the border inside the viewport, grab the canvas, scan each of the four border runs for a dark dash within one device pixel; capture `f.canvas->selectionContours()` and `selection_api::contour(*f.view).loops` before and after the fractional pan/zoom, and assert the stored loops the painter draws match the traced contour and are unchanged); the case is written to fail on at least one sample against the original clip
- [x] 3.2 Expand the clip of the block that draws the committed ants and the live preview by `1 / zoom` image units around `docClip` in `ImageView::paintEvent`; leave the contour and the content clip unchanged; the run is task 3.3
- [x] 3.3 Run `ctest --test-dir build -R '^tst_(selection_preview|selection_api)$' --output-on-failure` and verify the live-outline cases and the contour encoding are unchanged (both suites passed)

## 4. Selection-tool canvas menu (#884)

- [x] 4.1 Add `tst_canvas_context_menus::selectionMenuOpensForEverySelectionTool` (all seven tool ids), `tst_canvas_context_menus::selectionMenuRunsCommandsOnClickedDocument`, and `tst_canvas_context_menus::selectionMenuNeedsASelection`; the positive menu cases are written to fail against the original handlers
- [x] 4.2 Add `ToolServices::requestCommand`, its `ToolController::commandRequested` signal, the connection in `frame_build.cpp` to `CommandRegistry::dispatch`, and `tst_canvas_context_menus::requestCommandDispatches` (written with the port, since it calls the new method); `ToolController` is the sole `ToolServices` implementer; `requestCommandDispatches` exercises the port through the registered command
- [x] 4.3 Add the shared `selectionCanvasMenu(ToolServices&, const QPointF&)` helper in `tool_selection.cpp`, declare it in `tools.h`, and add the `hasCanvasContextMenu` / `buildContextMenu` overrides to the six handler classes covering all seven tool ids (the marquee handler serves rectangular and elliptical); label the entries `Deselect`, `Inverse`, `Feather…`, and `Refine Edge…`, resolve the Refine Edge id with `commandIdForPath`, and connect each action to `requestCommand`; `selectionMenuOpensForEverySelectionTool` exercises all seven ids
- [x] 4.4 Gate `ToolController::canvasMenuHandler` on an active selection for the seven selection tool ids after clicked-document activation; define `isSelectionMenuTool` beside `isSelectionTool` in `tools_selection_move.cpp` to keep `tools.cpp` under 1200 lines, and return true from `hasCanvasContextMenu` unconditionally so activation happens before the gate; `selectionMenuNeedsASelection` and the pasteboard fallback are the checks
- [x] 4.5 Verify `selectionMenuRunsCommandsOnClickedDocument` passes for Deselect, Inverse, Feather…, and Refine Edge…; drive the Feather value dialog and the Refine Edge dialog from a queued timer to accept, and assert only the clicked document's selection and history change. Then run `ctest --test-dir build -R '^tst_canvas_context_menus$' --output-on-failure` (the suite passed)

## 5. Refine Edge large document (#915)

Split to follow-up. No bounded repro/backtrace was obtained, so the
`tools/refine-edge` delta, the `re_large_document` plan row, and the #915
manual check were removed from this change. The engine work and repro continue
in #915.

## 6. Integration and archive

- [x] 6.1 Build once with `cmake -S . -B build -G Ninja && cmake --build build --parallel $(( $(nproc) > 1 ? $(nproc) * 4 / 5 : 1 ))`; verify the build succeeds (930/930 steps)
- [x] 6.2 Run `ctest --test-dir build -R '^tst_(tool_commit|selection_preview|selection_api|canvas_context_menus|refine_edge|tool_options_persistence)$' --output-on-failure`; verify every suite passes (6/6 suites passed)
- [x] 6.3 Run the non-archive gates individually: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `bash scripts/test-report.sh`, `bash scripts/check-file-size.sh`, `bash scripts/guard.sh`, `openspec validate --all --strict`, and `python3 scripts/check-test-plan.py`; verify every gate passes and fix every finding (all passed; test-report: 5960 passed, 30 skipped, 0 failed)
- [x] 6.4 Confirm the remaining manual checks in `test-plan.md` and list them in the PR body: #914 latency and #913 options page (the #915 check moved with the split)
- [x] 6.5 Archive the change with the archive workflow; verify it moved under `openspec/changes/archive/` (moved to `2026-10-25-selection-correctness-bundle`)
- [x] 6.6 Run `openspec validate --all --strict` and `scripts/check-openspec-archived.sh` on the archived tree; CI runs the build/verify gates (310 passed; archived check OK)
