# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `lct_move_preview_mask` | Create a blue square on a new layer, add a reveal-all layer mask, fill the square's left half black, clear the mask target, then press and drag the square with the Move tool | The masked half is the backdrop during the drag, the revealed half shows the layer, and the committed move leaves the same result | `tst_move_feedback::maskedLayerPreviewsMasked` |
| `lct_move_preview_mask_view` | Build a view preview at level 0 for a 2400 × 2000 layer whose mask hides the left half, with the presented rect inside the layer | The moving image is transparent over the masked half and shows the layer over the revealed half | `pictura-app::a_view_preview_applies_the_layer_mask` |
