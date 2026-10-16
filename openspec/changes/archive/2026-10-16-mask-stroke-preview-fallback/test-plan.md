# Test plan — mask-stroke-preview-fallback

One row per spec scenario. Coverage names the automated test, or
`manual: <reason>` when it cannot be automated.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `msp_large_mask_dab_previewed` | GPU compute off; a 600 px mask stroke on 2048²; two dabs and a flush | Preview level 3; no history state and level 0 unchanged until the commit | `tst_mask_edit::largeMaskDabPreviews` |
| `msp_preview_blend` | Blend zero, half and full coverage; Behind; a level wider than the proxy; reduce a mask and its extremes | Snapshot; halfway; hidden composite; unchanged; last proxy column reused; extremes equal the unmasked and hidden renders | `pictura-app::mask_preview_moves_the_snapshot_by_the_coverage_change`, `pictura-app::mask_preview_clamps_to_the_proxy_grid`, `pictura-render::mask_extremes_reduce_the_coverage_and_bracket_the_composite` |
| `msp_commit_exact` | End the 600 px stroke; undo | Hidden under the brush, untouched elsewhere, one `Brush` state; undo restores | `tst_mask_edit::largeMaskDabPreviews` |
| `msp_cancel` | Cancel a 600 px previewed mask stroke | No history state; composite unchanged | `tst_mask_edit::largeMaskDabPreviews` |
| `msp_restore_spacing` | Dabs at x 1024 and 1100 (spacing 150 px); probe the level at x 1376 before, during and after | Changed by the preview; the stroke-start pixel after the commit | `tst_mask_edit::previewOnlyPixelsRestoreAtCommit`, `pictura-app::snapshot_region_crops_the_stroke_start_level` |
| `msp_restore_mask_edge` | A 64² layer whose mask covers its top 32 rows; a preview dab crossing row 32; crop the commit restore | The preview changes a level pixel below the mask, the mask document stops at row 32, and the restore holds the stroke-start pixel (no app command makes a mask smaller than its layer, so this runs below the bridge; the commit restore itself is end-to-end in `msp_restore_spacing`) | `pictura-app::a_dab_past_the_mask_edge_previews_there_and_restores` |
| `msp_stroke_start` | Build the coverage document of a 4×4 mask | One channel sharing the mask's plane; empty composite | `pictura-render::a_coverage_document_is_single_channel` |
| Large-brush mask drag feel | 4000² and 8000² documents, GPU compute off, mask target, 600–1500 px brush, press and drag | The canvas follows the pointer every frame | manual: interactive latency on the reference machine; `PICTURA_PAINT_TIMING=1` prints the split |
