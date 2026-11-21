# Test plan

Rows are keyed by the delta's scenario tags, or by the scenario title where the
delta has no tag; the last row is the non-spec Layers panel brush path. Engine
rows run under `cargo nextest run -p pictura-select`; Qt Test rows run offscreen
through `ctest` after building `tst_refine_edge` and `tst_layers_mask_commands`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| The commands are enabled with a document and a selection | Open a document, check the `Select ▸ Refine Edge…` command and the options-bar button; add a selection and re-check. | Both are disabled without a selection and enabled with one; the button opens the dialog. | `tst_refine_edge::optionsBarButtonOpensIt` |
| OK records one state and refines the selection | Open the dialog on a selection, set Radius and click OK. | The selection changes and the history gains exactly one `Refine Edge` state. | `tst_refine_edge::previewsAndApplies` |
| Cancel is byte-identical | Open the dialog, drag the refinement brush across the edge, then Cancel. | Selection coverage, layer masks, pixels, and history are unchanged; the preview overlay and ring are gone. | `tst_refine_edge::cancelDiscardsBrushStrokes` |
| Decontaminate forbids in-place output | Check Decontaminate Colors and read the Output To selection. | The in-place targets are disabled and a colour target is selected. | `tst_refine_edge::decontaminateForbidsInPlaceOutput` |
| Layer Mask output writes the active layer's mask | Choose Layer Mask output and click OK. | The active layer gains a raster mask holding the refined coverage. | `tst_refine_edge::layerMaskOutputWritesTheMask` |
| `re_brush_canvas` | Open the dialog on a soft edge, drag the brush across the edge on the canvas, inspect the preview, then click OK. | Dabs are recorded; the refined preview changes; the committed coverage under the dab drops while a far pixel is unchanged; the document is untouched before OK and OK records one state. | `tst_refine_edge::canvasDragAddsBrushDabs` |
| `re_brush_keys` | Press `Shift+E`, hover the canvas, press `[` / `]`, then `J`; inspect the mode, size, ring diameter, and overlay. | Restore latches and unlatches; the hover ring diameter equals the brush size; `J` paints and clears a green Show Edge band on the canvas overlay. | `tst_refine_edge::brushKeysToggleResizeAndShowEdge` |
| `re_brush_alt` | Open the dialog; drag with Alt held and release; drag again without Alt; then latch `Shift+E` and drag without Alt. | Alt-held dabs restore; the following drag refines; a latched `Shift+E` makes a drag without Alt restore; the document is unchanged. | `tst_refine_edge::brushAltRestoresMomentarily` |
| `re_brush_refine` | Refine a soft boundary with Radius 0 and one refine dab; compare with the no-dab run. | Coverage changes inside the dab disk and is byte-identical outside; a refine dab's radius floor is the slider radius (checked by the bounded row). | `pictura-select::brush_stroke_changes_only_its_disk` |
| `re_brush_restore` | Apply a refine dab, an overlapping restore dab, then a refine dab over part of the same area, with Smooth, Feather, Contrast, and Shift Edge at their defaults. | Restore-only area equals the input; the final refine area is refined again. | `pictura-select::brush_restore_undoes_refinement_in_order` |
| `re_brush_order` | Apply a large refine dab, then a smaller overlapping refine dab, then a restore dab over part of that area. | The smaller refine dab replaces the earlier radius in its disk; the restore clears refinement including band expansion; the band does not mark a last-restored pixel. | `pictura-select::brush_later_refine_dab_replaces_the_radius` |
| `re_brush_invalid` | Run dabs with non-finite coordinates or radius, a negative radius, an outside centre, and an over-long radius. | Those dabs are ignored; the result matches a run without them. | `pictura-select::brush_invalid_dabs_are_ignored` |
| `re_brush_bounded` | Apply a refine dab of radius R with slider radius S on a straight-edge fixture; measure the changed distance with the engine's `boundary_distance`, as `refine_distance_is_pixels` does; also check the radius helper and that a small dab cannot narrow the slider band. | No changed pixel lies farther than the larger of S and R; a refine dab's floor is `max(S, R)`. | `pictura-select::brush_radius_bounds_changed_pixels`, `pictura-select::effective_band_radius_matches_the_brush_rule`, `pictura-select::slider_radius_still_refines_outside_a_small_dab` |
| `re_brush_scale` | Preview the same dabs at the proxy size with the per-axis x/y scaling and apply at document size with a dab radius of at least 8 document pixels. | The same pixel at the image edge is `0` without the dab and nonzero in the full-size preview and the commit; the proxy's brushed edge matches the commit within two proxy pixels, allowing for `scale_preview_params`' clamp to at least one proxy pixel. | `tst_refine_edge::brushDabsApplyThroughTheBridge` |
| `re_brush_band` | Request the refinement band with a slider radius plus a refine dab at the engine and through the preview extern. | The engine test proves the radius rule; the preview extern marks `(30,20)` with the dab and leaves it clear without one. | `pictura-select::edge_band_matches_the_brush_expanded_radius`, `tst_refine_edge::brushBandPreviewMatchesTheEngine` |
| Layers panel Refine Mask accepts brush dabs (non-spec, proposal promise) | On a layer with a mask, open Refine Mask through the Layers panel with a driver that presses once on the canvas at `(32,32)`. | The dialog saw one dab; the entry point applies it, records one `Refine Mask` state, changes `(32,32)`, leaves `(4,32)` unchanged, and keeps the mask linked. | `tst_layers_mask_commands::refineMaskBrushes` |

Existing regression coverage to run in addition: `defaults_are_identity`,
`larger_radius_reaches_farther_but_stays_bounded`,
`refine_distance_is_pixels`, `previewMatchesCommitWithinProxyTolerance`,
`cancelChangesNothing`, and the Layers panel `refineMask` case. None of their
baselines should change.

The parity acceptance criteria about recovering hair strands are deliberately
behavioral: the engine's coverage estimate is an approximation, so the plan
asserts geometry (the disk, the restore, the order, the invalid-dab filtering,
the bound, the scale, the band) rather than a photographic result. The
geometry/approximation ceiling is recorded in the docs and the parity register.
