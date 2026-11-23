## 1. Layer Edges selection refresh

- [x] 1.1 Refresh the active document's Layer Edges on a Layers-panel selection
  change.
- [x] 1.2 Extend `tst_view_menu::shapeTargetPathFollowsActiveShape` to check the
  layer edges after selection and during a drag.

## 2. Verification

- [x] 2.1 `tst_view_menu`, `tst_move_feedback`, and `tst_align_distribute` pass.
- [x] 2.2 `openspec validate --all --strict` passes.
- [x] 2.3 Archive the change.
