# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| lct_move_preview_fresh | Move a linked set, then begin another drag and commit a delta | The moved set recomputes its preview at the new union rect, previews and lands at the pointer's offset, and an unmoved group still reuses its preview | `tst_linked_layers::linkedMovePreviewOriginMatchesTheCrop` |
