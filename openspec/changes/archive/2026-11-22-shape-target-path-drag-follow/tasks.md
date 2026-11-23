## 1. Drag-follow overlay

- [x] 1.1 Mark a Target Path that is the active layer's outline.
- [x] 1.2 Offset the painted curve by the Move preview delta.
- [x] 1.3 Extend `tst_view_menu::shapeTargetPathFollowsMove` to check the curve
  during the drag and after the commit.

## 2. Verification

- [x] 2.1 `tst_view_menu`, `tst_move_feedback`, and `tst_align_distribute` pass.
- [x] 2.2 `openspec validate --all --strict` passes.
- [x] 2.3 Archive the change.
