# Test plan: layer-edges-follow-active-layer

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `vm_layer_edges` | Make a small pixel layer active on a larger canvas with Layer Edges on. | The outline matches the layer's content rect and not the canvas rect. | `tst_view_menu::layerEdges` |
| `vm_layer_edges_target` | Switch the active layer between pixel layers at different positions. | The outline follows the new active layer's content bounds. | `tst_view_menu::layerEdges` |
| `vm_layer_edges_edit` | Change the active layer's pixels so its content bounds grow. | The outline updates to the new content rect. | `tst_view_menu::layerEdges` |
| `vm_layer_edges_drag` | Create a shape layer, start a Move preview, set the preview delta, and check the dashed outline before releasing. | The outline follows the preview offset during the drag and the committed content bounds after release. | `tst_view_menu::layerEdges` |
| `vm_layer_edges_active` | Select one shape layer, refresh, then select a second shape layer in the Layers panel without another document edit. | The dashed outline shows the second layer's content bounds, and a drag of it follows that outline. | `tst_view_menu::shapeTargetPathFollowsActiveShape` |
