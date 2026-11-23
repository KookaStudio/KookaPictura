## 1. Layer Edges preview

- [x] 1.1 Record whether a Move preview moves a layer or selection content.
- [x] 1.2 Offset Layer Edges and the Target Path only for a layer move.
- [x] 1.3 Extend `tst_view_menu::layerEdges` to check the dashed outline during
  the drag and after the commit.

## 2. Verification

- [x] 2.1 `tst_view_menu`, `tst_move_feedback`, and `tst_align_distribute` pass.
- [x] 2.2 `openspec validate --all --strict` passes.
- [x] 2.3 Archive the change.
