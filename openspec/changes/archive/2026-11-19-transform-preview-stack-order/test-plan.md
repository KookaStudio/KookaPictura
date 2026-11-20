# Test plan

One row per scenario in the change's delta specs. The first cell is the
scenario's tag.

| Scenario | Steps | Expected | Coverage |
| --- | --- | --- | --- |
| `ft_preview_stack` | New 120 × 100 white document; fill a blue square at (20, 20, 40, 40) and an opaque red square above it at (40, 40, 40, 40); select the blue and drag its bottom-right handle from (60, 60) to (100, 100) with the Move tool's controls box | At the overlap (55, 50) the red shows; outside the red but inside the scaled blue (90, 90) the blue shows; the above-layer image is non-null | `tst_transform_controls::lowerLayerStaysUnderTheLayersAbove` |
| `ft_preview_top` | Begin a handle drag on the topmost layer of a document | The above-layer image is null and the preview shows the transformed layer | `tst_transform_controls::topmostLayerBuildsNoAboveImage`, `pictura-app::topmost_layer_has_no_above_patch` |

Manual checks: none.
