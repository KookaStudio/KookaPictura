# Tasks: type-move-responsiveness

## 1. Tests

- [x] 1.1 Add `pictura-app::a_channel_less_type_layer_materializes_for_a_move`.
- [x] 1.2 Add `pictura-render::type_ink_reach_keeps_the_shadow_pad`.
- [x] 1.3 Extend the ink-bounds test to a large, non-near-canvas box and an
  empty effects block.

## 2. Implementation

- [x] 2.1 Add `pictura_render::materialize_type_layer` and call it on the Move
  press for a channel-less type layer.
- [x] 2.2 Make `type_ink_influence` the ink plus the effect pad for a type layer
  with no knockout, clipping, or vector mask.

## 3. Verification

- [x] 3.1 The `pictura_app` and `pictura-render` tests pass.
- [x] 3.2 `tst_move_feedback`, `tst_view_menu`, and `tst_large_document` pass.
- [x] 3.3 `openspec validate --all --strict` passes.
- [x] 3.4 Archive the change.
