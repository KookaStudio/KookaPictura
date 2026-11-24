# Tasks: type-layer-move-locks

## 1. Tests

- [x] 1.1 Add `tst_move_feedback::typeLayerMovesWholeWithASelection`.
- [x] 1.2 Add `tst_move_feedback::positionLockedTypeLayerRefusesCleanly`.
- [x] 1.3 Extend `tst_move_feedback::dragShowsAntsAndReadout` to assert the type
  layer landed at the drag offset.
- [x] 1.4 Add the Rust ink-bounds test
  `a_type_move_previews_and_damages_its_ink_not_the_whole_box`.
- [x] 1.5 Add `tst_move_feedback::pixelSelectionStillContentMoves` so a plain
  pixel layer keeps its selected-pixel content move.

## 2. Implementation

- [x] 2.1 Add `PictureView::selection_content_movable` and its bridge function.
- [x] 2.2 Move the Move tool's content-move preflight before `beginContentMove`
  and fall through to the whole-layer move.
- [x] 2.3 Use `ink_influence` for a type layer's move reach, backdrop patch, and
  moving crop; keep the crop offset across a committed move.

## 3. Verification

- [x] 3.1 `cargo fmt --all --check` and `cargo clippy -p pictura_app
  --all-targets -- -D warnings` pass.
- [x] 3.2 The `pictura_app` Move tests pass (`cargo test --lib move`).
- [x] 3.3 `tst_move_feedback`, `tst_selection_move`, `tst_type_tools`, and
  `tst_linked_layers` pass.
- [x] 3.4 `openspec validate --all --strict` passes.
- [x] 3.5 Archive the change.
