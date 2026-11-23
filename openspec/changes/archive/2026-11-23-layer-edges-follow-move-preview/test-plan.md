# Test plan: layer-edges-follow-move-preview

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `vm_layer_edges` | Make a small pixel layer active on a larger canvas with Layer Edges on. | The outline matches the layer's content rect and not the canvas rect. | `tst_view_menu::layerEdges` |
| `vm_layer_edges_target` | Switch the active layer between pixel layers at different positions. | The outline follows the new active layer's content bounds. | `tst_view_menu::layerEdges` |
| `vm_layer_edges_edit` | Change the active layer's pixels so its content bounds grow. | The outline updates to the new content rect. | `tst_view_menu::layerEdges` |
| `vm_layer_edges_drag` | Create a shape layer, start a Move preview, set the preview delta, and check the dashed outline before releasing. | The outline follows the preview offset during the drag and the committed content bounds after release. | `tst_view_menu::layerEdges` |
