# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `scp_layer_stroke` | Select a rectangle, then drag the Brush across its edge on a pixel layer. | Pixels outside the selection keep their pre-stroke values; pixels inside take the brush colour. | `tst_tool_commit::aSelectionClipsABrushStroke`, `pictura-paint::a_selection_clip_stops_a_stroke_at_its_edge` |
| `scp_growth` | Add an empty layer, select part of the canvas, then stroke across it. | The layer grows to the dab extent; only selected pixels show paint, the rest stay transparent. | `tst_tool_commit::aSelectionClipsANewLayerStroke`, `pictura-paint::a_grown_layer_keeps_the_document_space_clip` |
| `scp_per_dab` | Select a rectangle, then drag Dodge across its edge on a pixel layer. | Pixels outside the selection keep their pre-stroke values; pixels inside are dodged. | `pictura-paint::a_per_dab_tool_is_clipped_too` |
| `scp_soft_clip` | Make a feathered selection and paint a full-coverage dab under partial coverage. | The changed pixel moves toward the brush colour by that coverage instead of taking it fully. | `pictura-paint::a_partial_selection_scales_the_paint` |
| `scp_no_selection` | Deselect, then paint across the layer. | Every dab pixel takes the brush colour as before. | `pictura-paint::a_stroke_without_a_selection_paints_everywhere` |
| `scp_mask_stroke` | Select a rectangle, target a reveal-all layer mask, and paint black across its edge. | Mask coverage inside the selection becomes black; coverage outside stays white; layer pixels are untouched. | `tst_tool_commit::aSelectionClipsAMaskStroke` |
| `scp_preview` | Stroke with a brush large enough to preview across a selection. | The preview shows paint only inside the selection and the committed pixels outside are unchanged. | `pictura-app::preview_blend_honours_the_selection_clip`, `tst_tool_commit::aSelectionClipsABrushStroke` |
