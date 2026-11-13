# Tasks

## 1. Tests first

- [x] 1.1 Add `tst_canvas_mask_domain::maskThumbCompanions`, covering `vmk_row_vector_selects_both`, `vmk_row_vector_toggle_off`, `lmk_row_raster_keeps_vector_state`, `vmk_row_vector_companion_content`, and `vmk_row_vector_companion_mask`; the case includes the content-thumb click with an active vector (`vmk_row_vector_companion_content`), which is also the vector-preserving content click for `lmk_row_deactivate`; verify the slot is declared in the class and the case asserts the bit state, target routing, and bracket pixels.
- [x] 1.2 Add `tst_canvas_mask_domain::vectorOnlyMaskThumbCompanion`, covering `vmk_row_vector_only_companion`; verify it builds a layer with no raster mask and asserts content+vector with a clear target.
- [x] 1.3 Add `tst_canvas_mask_domain::inactiveLayerThumbClicks`, covering `vmk_row_inactive_content`, `vmk_row_inactive_mask`, and `vmk_row_inactive_vector`; re-activate the other layer before EACH of the three thumb clicks, then assert content-only, raster-mask-only, and content+vector on the first row.
- [x] 1.4 Add `tst_canvas_mask_domain::otherLayerClearsThumbSelection`, covering `vmk_row_other_layer_clears`; select raster+vector, change layer through `window_->selectLayerPath`/`set_active_layer` (the `refresh`/`projectTarget` path), refresh, and assert the old row has no mask/vector bits BEFORE any row-body click; then the row-body click selects content.
- [x] 1.5 Update the existing active-thumb expectations in `tst_canvas_mask_domain.cpp`, `tst_panels_parity.cpp`, and `tst_layers_panel.cpp` to the three-bit values (content 1, raster 2, vector combinations 5/6); assert the white/black mask pair after the vector click (the raster target stays) and the image pair after the content-thumb click, keeping both assertions; `maskThumbCompanions` includes the content-click-with-active-vector case; verify every changed assertion names a declared slot.
- [x] 1.6 Extend `tst_canvas_mask_domain::maskModifierIndicatorsRemain` and `tst_layers_panel::vectorMaskRowIndicators` with the companion/vector selection assertions for `lmk_row_modifiers_keep_selection`; verify a `Shift`-click on either mask, a `Ctrl`-click on the raster mask, and an `Alt`+`Shift`-click on the raster mask leave the selection unchanged (an inactive row may default to content), while an `Alt`-click on the raster mask still selects it as the companion and target; keep `tst_layers_panel.cpp` at or below its 1408-line allowlist ceiling.
- [x] 1.7 Extend `tst_canvas_mask_domain::maskModifierIndicatorsRemain` with the `Alt`-click target and raster bracket assertions for `lmk_row_alt_solo` and `lmk_row_alt_restore`.

## 2. Selection model and target routing

- [x] 2.1 Add the `ActiveThumbBit` values (`Content = 1`, `Mask = 2`, `Vector = 4`) near `ActiveThumbRole` in `layers_panel_model.h`; update the `activeThumbByPath_` comment in `layers_panel.h:585` and the `LayerRowNode::activeThumb` comment in `layers_panel_model.h:129-131` to the bit model; verify both the panel and delegate compile against the names.
- [x] 2.2 Normalize the row state in `LayersPanel::refresh` to `(hash & Vector) | (row == maskTarget ? Mask : Content)`; verify a both-mask row with an active vector shows 6 after `refresh()` and a vector-only row shows 5.
- [x] 2.3 Prune `activeThumbByPath_` to `layer_selection_anchor(*view_)` at the top of `LayersPanel::refresh`, before the snapshot and row loop, and clear the hash in the `newDocument` (document-generation) block; verify `otherLayerClearsThumbSelection` and the row-change paths in the mask suites; `projectTarget` bypasses `rememberTarget`, so do not rely on a prune there.
- [x] 2.4 Update the content, raster-mask, and vector-mask click handlers in `layers_panel_events.cpp`: gate the selection writes so `Shift`, `Ctrl`, and `Alt`+`Shift` perform only their existing effects (the raster branch moves `setActiveThumb` after those early returns; the vector branch handles `Shift` before the plain toggle), keep `Alt`-raster selecting the raster companion and target, start inactive rows from content, preserve the companion and vector, write the content companion as `(bits & Vector) | Content`, and set the raster target exactly when the raster mask is the companion; verify `maskThumbCompanions`, `vectorOnlyMaskThumbCompanion`, `inactiveLayerThumbClicks`, and the modifier assertions.
- [x] 2.5 Write `ActiveThumbContent` (not `0`) in the row-body default, guard it with `!rowControlAt(index, pos)`, and use `ActiveThumbMask` in the right-click mask menu; verify `tst_canvas_mask_domain::layerRowClickDefaultsToContent`, `tst_layers_panel::maskEditTargetActivation`, and the mask menu suite still pass.

## 3. Delegate

- [x] 3.1 Paint the delegate brackets per selected bit: content (bit 1, no group icon), raster mask (bit 2), vector (bit 4); update the stale "active edit target's thumbnail only" comment in `layers_panel_internal.h:1027`; verify the bracket-pixel assertions in `maskThumbCompanions`, `inactiveLayerThumbClicks`, and `maskThumbnailUsesOnlyBrackets`, and keep `layers_panel_internal.h` at or below 1200 lines.

## 4. Integration

- [x] 4.1 Run `openspec validate --all --strict`; verify the change and the archive-ready delta pass.
- [x] 4.2 Build and run `tst_canvas_mask_domain`, `tst_panels_parity`, `tst_layers_panel`, `tst_mask_edit`, `tst_quick_mask`, `tst_layers_panel_fixes`, and `tst_layers_snapshot` offscreen; verify all pass.
- [x] 4.3 Run `python3 scripts/check-test-plan.py`, `scripts/check-file-size.sh`, and `bash scripts/verify-fast.sh`; verify all pass.
- [x] 4.4 Run `openspec archive fix-vector-mask-thumb-selection` and verify `openspec/changes/` contains no non-archived change.
