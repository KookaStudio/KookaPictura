# Test plan

Every scenario in the delta specs has a row. The carried-over scenarios are
covered by existing tests; the multi-selection behaviour gets
`tst_linked_layers::moveMultiSelectionMovesEveryLayer`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Drag moves the layer content | Drag the Move tool by a delta | The layer translates and the composite updates | `tst_transform_controls::insideDragMovesAndRecordsMove` |
| No pixel layer | Drag with no movable layer | Nothing changes | `tst_large_document::moveDoesNotWarmTheBackground` |
| lct_move_locked | Drag a position-locked layer | It does not move, no history state | `tst_linked_layers::lockedMemberStaysPut` |
| lct_move_multi | Select two pixel layers and drag | Both translate by the delta in one history state | `tst_linked_layers::moveMultiSelectionMovesEveryLayer` |
| Dragging a linked layer moves the set | Link two layers, move one | Both shift by the delta, one undo restores both | `tst_linked_layers::dragLinkedLayersMovesSet` |
| An unlinked layer moves alone | Move an unlinked layer | Only that layer shifts | `tst_linked_layers::dragUnlinkedLayerAlone` |
| A position-locked member is left in place | Move a set with a locked member | The locked layer stays, the other shifts | `tst_linked_layers::lockedMemberStaysPut` |
| A multi-selection moves with its linked partners | Select one of two linked layers and a third, move | All three shift in one undoable state | `tst_linked_layers::moveMultiSelectionMovesEveryLayer` |
