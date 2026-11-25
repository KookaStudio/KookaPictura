# Tasks: type-move-layer-edges

## 1. Tests

- [x] 1.1 Add `tst_move_feedback::largeTypeMoveKeepsLayerEdgesOnTheText`.
- [x] 1.2 Extend the Rust ink-bounds test to a paragraph box that is large but
  not near-canvas.

## 2. Implementation

- [x] 2.1 Add `damage::type_ink_influence` and use it in the Move reach,
  backdrop patch, and moving-image crop.
- [x] 2.2 Key the Layer Edges cache on the active layer's rect.
- [x] 2.3 Stop re-adding the drag delta when a deferred commit's outline refresh
  lands at the committed place.

## 3. Verification

- [x] 3.1 `tst_move_feedback`, `tst_view_menu`, and `tst_large_document` pass.
- [x] 3.2 The `pictura_app` move tests and clippy pass.
- [x] 3.3 `openspec validate --all --strict` passes.
- [x] 3.4 Archive the change.
