# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `tpr_layer_region` | Prepare a masked layer in a larger document | Moving image equals the cropped full render | `pictura-app::rendered_layer_preview_equals_the_cropped_full_render` |
| `tpr_patch_exact` | Prepare a small layer over another layer | Patch of the layer bounds; pixels match the full backdrop | `pictura-app::a_small_layer_gets_a_backdrop_patch` |
| `tpr_full_backdrop` | Prepare a layer with effects, and a lifted session | Whole-document backdrop | `pictura-app::effects_and_lifts_keep_the_whole_backdrop` |
| `tpr_canvas_patch` | Press and drag a handle of a small layer over transparency | Old place clears; layer drawn; canvas elsewhere unchanged | `tst_transform_controls::handlePressPatchesTheBackdrop` |
| `tpr_sample_banded` | Read a pixel of a composite with a forked band | Forked value; no materialization | `pictura-app::sample_planar_argb_reads_a_diverged_composite_without_materializing` |
| `tpr_canvas_kept` | Drag a small layer and a handle at 58 % zoom | Canvas outside the layer unchanged | `tst_transform_controls::previewsKeepTheCanvasPixels` |
| Feel on the 16507×16196 world map | Add text, drag a corner handle with the Move tool | The handle responds at once | manual: perceived lag needs a real display and pointer; `scripts/profile-tools.py` records the timings |
