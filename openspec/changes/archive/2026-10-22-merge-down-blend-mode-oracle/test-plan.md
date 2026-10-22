# Test Plan

Covers every scenario in the `compositing/layer-management` delta.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `merge_down_source_blend` | Merge Down a source layer whose blend mode is `Multiply` over a target layer; diff the merged node against the committed ImageMagick `Multiply` reference for the `solid` and `ramp` scenes | The merged node equals the ImageMagick reference at tolerance 0 | `pictura-render::merge_down_matches_imagemagick_for_every_supported_mode` |
| `merge_down_all_modes` | Run the merge oracle over the 19 modes ImageMagick implements with the same formula, in the `solid` and `ramp` scenes | Each merged node equals its committed reference within that mode's tolerance (0, except Vivid Light at 1) | `pictura-render::merge_down_matches_imagemagick_for_every_supported_mode` |
| The merged pixels are the blend composite | Merge Down a source of `(120, 130, 140)` with the `Multiply` blend mode over a target of `(40, 60, 80)` with the `Screen` blend mode, and read the merged node | The merged node's pixels are the hand-computed `Multiply` composite `(19, 31, 44)`, and the merged node carries the target's `Screen` blend mode, not the source's `Multiply` | `pictura-render::merge_down_bakes_the_sources_blend_into_the_pixels` |
| The live reference matches | Regenerate the `ramp` scene's `Multiply` composite with a live `magick` and diff the merged node against it | The merged node equals the live composite; the check is skipped when `magick` is absent | `pictura-render::merge_down_matches_live_imagemagick` |
