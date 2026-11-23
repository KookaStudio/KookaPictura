## 1. Target path refresh

- [x] 1.1 Refresh the active window's Target Path overlay on a Layers-panel
  selection change.
- [x] 1.2 Skip a window without a canvas in `refreshTargetPathOverlay`.
- [x] 1.3 Add `tst_view_menu::shapeTargetPathFollowsActiveShape`.

## 2. Verification

- [x] 2.1 `tst_view_menu`, `tst_move_feedback`, and `tst_align_distribute` pass.
- [x] 2.2 `openspec validate --all --strict` passes.
- [x] 2.3 Archive the change.
