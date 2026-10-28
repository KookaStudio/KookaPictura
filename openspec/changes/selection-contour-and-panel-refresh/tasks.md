# Tasks

## 1. Tests first

- [x] 1.1 Add the Rust cache tests to `crates/pictura-app/src/cxxqt_object/canvas_view_test.rs`: `selection_contour_cache_reuses_a_revision`, `selection_contour_cache_recomputes_after_a_selection_change`, and `selection_contour_cache_clears_without_a_selection`; verify the names match `test-plan.md` (compilation is verified by 2.2)
- [x] 1.2 Add `tst_selection_preview` cases `anEditKeepsTheCachedContour`, `aSelectionChangeRetracesTheContour`, and `antsTickRepaintsOnlyTheOutline`; register no new suite file. `aSelectionChangeRetracesTheContour` opens a second window of the same document and asserts both canvases show the new contour. `antsTickRepaintsOnlyTheOutline` shows the window/canvas and waits for exposure before expecting a tick. Verify the names match `test-plan.md` (compilation is verified by 2.3 and 5.1)
- [x] 1.3 Add `tst_edit_refresh` cases `anEditRefreshesTheDockInPlace` (one committed edit and one region-preview tick, each with the retarget count flat), `aDocumentSwitchRebindsTheDock` (a document switch, a generation change, and a second-window switch of one document, each with one Layers refresh; open a document with pattern resources and switch back, asserting the pattern controls after the switch; include the deferred bind while a filter commit runs on the new view), `anEditDoesNotRebindTheDock`, `aCommittedEditRefreshesLayersOnce` (including a background-document change), `aStrokeCommitRefreshesPanels`, `aStrokeCommitDoesNotRetraceTheContour`, `previewTicksCoalesceTheRegistry`, `aCommitRefreshIsNotDelayedByPreviews`, and `anInactiveCloseDoesNotRebindTheDock`; verify the names match `test-plan.md` (compilation is verified by 3.2 and 4.1)

## 2. Selection revision and contour cache

- [x] 2.1 Add `selection_revision`, `set_selection`, `take_selection`, and `touch_selection` to `DocumentSession`; route every selection write through them, including `impl_selection`, quick selection, refine, selection move, healing, history restore/undo, clipboard, crop group, HDR conversion, image size/ops, transform dispatch/session, and document clears; verify `cargo check -p pictura-app` (no test targets: the 1.1 tests cannot compile before 2.2)
- [x] 2.2 Add the revision-keyed contour cache and compute counter to `PictureViewRust`, factor the encoded-extents helper, and expose `selection_revision` and `selection_contour_compute_count` through the bridge; verify `cargo nextest run -p pictura-app selection_contour` runs the three tests from 1.1
- [x] 2.3 Carry the revision in `selection_api::SelectionContour`, guard `ImageView::setSelectionContour` on an already-shown revision, reset the stored revision in `clearSelectionContour`, and add the contour-set count and last-paint-rect test hooks in the same task; verify the `tst_selection_preview` cases `anEditKeepsTheCachedContour` and `aSelectionChangeRetracesTheContour` (second window included) after `cmake --build build --target tst_selection_preview tst_selection_move tst_selection_api --parallel`, then `ctest --test-dir build -R '^tst_(selection_preview|selection_move|selection_api)$' --output-on-failure` (the same suite's ants case is verified at 5.1)

## 3. Dock bind/refresh split

- [x] 3.1 Remember the bound view, canvas, and document generation in `PicturaMainWindow`; split `retargetDock` into a bind path and a refresh-only path; add `dockRetargetCountForTest`; defer the bind while a filter commit runs on the new view; run `registerTypeFallbacks` unconditionally from the debounced pass; verify `cmake --build build --target pictura_shell --parallel` (the `tst_edit_refresh` cases need the 3.2 hooks)
- [x] 3.2 Add the `changed`/region flags with active-view scoping and the surface-change clear; add `LayersPanel::refreshCountForTest`, `LayersPanel::patternActionEnabledForTest`, and the Histogram bin reader; move the pattern controls' state sync into refresh methods and add `AdjustmentsPanel::refresh`; make `AdjustmentsPanel::setView` delegate to that refresh (or have the bind path call it once), so both bind and refresh-only paths update the pattern button; verify, after `cmake --build build --target tst_edit_refresh --parallel`, the cases `anEditRefreshesTheDockInPlace`, `anEditDoesNotRebindTheDock`, `aCommittedEditRefreshesLayersOnce`, `aStrokeCommitRefreshesPanels`, and `aStrokeCommitDoesNotRetraceTheContour`
- [x] 3.3 Remove the explicit `refresh()` calls after `setView` in the bind path for History, Navigator, Info, Character, Paragraph, ParagraphStyles, CharacterStyles, Paths, and Histogram; verify `aDocumentSwitchRebindsTheDock` asserts one Layers refresh and the correct Adjustments pattern button after `cmake --build build --target tst_edit_refresh --parallel`
- [x] 3.4 Rebind in `removeDocument` only when the removed view is the currently bound or active view; verify `anInactiveCloseDoesNotRebindTheDock` after `cmake --build build --target tst_edit_refresh --parallel`
- [x] 3.5 Fix the stale mid-stroke comment in `frame_canvas.cpp` to name the commit's region presentation rather than `changed`; verify the comment matches the `end_paint` sequence and no behavior changes

## 4. Command registry coalescing

- [x] 4.1 Replace the synchronous `registry_->refresh()` in `afterPresent` with a pending flag consumed by the debounced pass; clear the flag after `refresh()`'s synchronous recomputation; verify `previewTicksCoalesceTheRegistry` and `aCommitRefreshIsNotDelayedByPreviews` after `cmake --build build --target tst_edit_refresh --parallel`
- [x] 4.2 Update `pp_no_registry_refresh` (exit code 394, `selftest_paint_perf.cpp:199-236`): after `end_paint` assert the synchronous count is unchanged, pump the event loop past the 120 ms pass, then assert one refresh; update its stale "releasing the stroke emits changed" comment; verify `cmake --build build --parallel && ./build/pictura --headless --self-test` (or `bash scripts/test-report.sh` after a build)
- [x] 4.3 Keep the existing one-refresh-per-notifying-edit contract; verify `aCommandRefreshesOnce` and `ctest --test-dir build -R '^tst_command_bus$' --output-on-failure`

## 5. Marching ants partial repaint

- [x] 5.1 Add `ImageView::antsUpdateRect()` and make the ants timeout call `update(rect)`; include the committed contour, live previews, `moveOutline_.translated(moveDelta_)`, and the selection move offset; inflate for the cosmetic pen; fall back to a full update when bounds are empty; verify `antsTickRepaintsOnlyTheOutline` after `cmake --build build --target tst_selection_preview --parallel`
- [x] 5.2 Confirm the existing hide/show timer stop still applies; verify the full `tst_selection_preview` suite and no runaway timer in `--headless` self-test

## 6. Integration

- [ ] 6.1 Run `bash scripts/verify-full.sh` (CMake configure/build, fast gate, app self-test) before the ctest task
- [ ] 6.2 Run `ctest --test-dir build -R '^tst_(edit_refresh|selection_preview|selection_api|selection_move|layers_preview_coalescing|histogram_panel)$' --output-on-failure`
- [ ] 6.3 Run `python3 scripts/check-test-plan.py` and `openspec validate --all --strict`
- [ ] 6.4 Manual reviewer check with `PICTURA_METRICS=1` on a 6000×4000 document: a filter commit that does not write the selection leaves the contour compute count unchanged, one Layers refresh is recorded, and region ticks print at most one command refresh per 120 ms; note the before/after numbers in the PR
