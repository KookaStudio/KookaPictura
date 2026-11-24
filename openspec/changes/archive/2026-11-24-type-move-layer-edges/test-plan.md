# Test plan: type-move-layer-edges

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `vm_layer_edges` | Make a small pixel layer active on a larger canvas with Layer Edges on. | The outline matches the layer's content rect and not the canvas rect. | `tst_view_menu::layerEdges` |
| `vm_layer_edges_target` | Switch the active layer between pixel layers at different positions. | The outline follows the new active layer's content bounds. | `tst_view_menu::layerEdges` |
| `vm_layer_edges_edit` | Change the active layer's pixels so its content bounds grow. | The outline updates to the new content rect. | `tst_view_menu::layerEdges` |
| `vm_layer_edges_drag` | Create a shape layer, start a Move preview, set the preview delta, and check the dashed outline before releasing. | The outline follows the preview offset during the drag and the committed content bounds after release. | `tst_view_menu::layerEdges` |
| `vm_layer_edges_active` | Select one shape layer, refresh, then select a second shape layer in the Layers panel without another document edit. | The dashed outline shows the second layer's content bounds. | `tst_view_menu::shapeTargetPathFollowsActiveShape` |
| `vm_layer_edges_content_move` | Start a preview with the content-move default, set a preview delta, and check the overlays. | The Layer Edges outline stays at the committed content bounds. | `tst_view_menu::contentMoveLeavesOverlays` |
| `vm_layer_edges_type_move` | On a 4000 × 4000 document, make a paragraph type layer active and Move-drag it by (40, 20). | The outline follows the text during the drag and lands at the moved text after the commit. | `tst_move_feedback::largeTypeMoveKeepsLayerEdgesOnTheText` |
