# Test plan

One row per scenario in the change's delta specs. The first cell is the
scenario's tag.

| Scenario | Steps | Expected | Coverage |
| --- | --- | --- | --- |
| `lct_move_preview_stack` | New 200 × 160 document; fill a blue square at (20, 20) and an opaque red square above it at (40, 40); select the blue, press at (30, 30), drag to (70, 50) with the Move tool, grab the canvas | At the overlap (70, 50) the red shows; outside it (90, 45) the blue shows; the old place (25, 25) is white | `tst_move_feedback::movedLayerStaysUnderTheLayersAbove`, `pictura-app::above_patch_is_the_layers_above_the_group` |
| `lct_move_preview_top` | Prepare a Move drag on the topmost layer | `move_preview_above` is null and the moving image is unchanged | `pictura-app::topmost_layer_has_no_above_patch` |
| `lct_move_preview_link_stack` | Two blue squares linked, under a red square; drag the set under the red square | The red still shows at the overlap and the second member shows outside it | `tst_move_feedback::linkedSetStaysUnderTheLayersAbove` |
| `cvda_move_previous_frame` | Press a blue square, move it, grab the canvas, move it again, then keep only what a repaint limited to `antsUpdateRectForTest()` would repaint | The previous frame's pixel (45, 30) is white; the current frame (75, 30) is blue | `tst_move_feedback::antsRepaintClearsThePreviousFrame` |

Manual checks: none.
