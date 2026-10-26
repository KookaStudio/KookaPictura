# Test plan

Every scenario in the delta specs has a row. The carried-over scenarios are
covered by existing tests; the two fixes get new `tst_linked_layers` cases.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Drag moves the layer content | Drag the Move tool by a delta | The layer translates and the composite updates | `tst_transform_controls::insideDragMovesAndRecordsMove` |
| No pixel layer | Drag with no movable layer | Nothing changes | `tst_large_document::moveDoesNotWarmTheBackground` |
| lct_move_locked | Drag a position-locked layer | It does not move, no history state | `tst_linked_layers::lockedMemberStaysPut` |
| lct_move_multi | Select two pixel layers and drag | Both translate by the delta in one history state | `tst_linked_layers::moveMultiSelectionMovesEveryLayer` |
| lct_move_preview_origin | Move a linked set off the canvas edge, preview again, commit a delta | The cached preview origin is the clamped crop origin and the set lands at the drop | `tst_linked_layers::linkedMovePreviewOriginMatchesTheCrop`, `tst_linked_layers::canvasDragLandsAtTheDrop` |
| sg_document_centre | Drag a content centre near the document centre | A guide is drawn and the centre snaps | `tst_smart_guides::documentCentre` |
| sg_layer_edge | Drag one layer's edge near another's | A guide is drawn and the edges coincide | `tst_smart_guides::layerEdge` |
| sg_threshold | Drag just outside the threshold | No guide, the offset is kept | `tst_smart_guides::threshold` |
| sg_clears | Release the drag | No smart guides remain | `tst_smart_guides::documentCentre` |
| sg_type_move | Drag live text near the document centre | A guide is drawn and the ink box is centred | `tst_smart_guides::typeMove` |
| sg_no_self | Select both linked layers and read the references | Neither moving layer is a reference | `tst_linked_layers::multiSelectionIsNotItsOwnReference` |
