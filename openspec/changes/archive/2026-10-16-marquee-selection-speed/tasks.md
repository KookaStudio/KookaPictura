# Tasks

## 1. Engine

- [x] 1.1 Run-based `contour` in `crates/pictura-select/src/contour.rs`; verify with a seeded random-mask test that the drawn unit edges equal the brute-force pixel boundary, and that a rectangle is four corners.
- [x] 1.2 Bounded `Selection::rect` / `Selection::ellipse` fills; verify byte-identity against the per-pixel definitions, including off-canvas boxes.
- [x] 1.3 Row-copy `Selection::translate`; verify against the per-pixel shift, including shifts past the canvas.
- [x] 1.4 `feathered` helper at the marquee, lasso, and path-selection call sites; New/Add into no selection uses the shape.
- [x] 1.5 `marquee_selection_profile` ignored profile records the numbers in `design.md`.

## 2. Canvas

- [x] 2.1 `ImageView::setSelectionContourOffset`; the move-selection drag sets it instead of emitting `selectionPreviewChanged` (removed); release and cancel reset it.
- [x] 2.2 Qt suite `tst_selection_move`: the drag offsets the origin contour while the mask moves live; release retraces; cancel drops the offset.
- [x] 2.3 Self-test checks 259, 260, 267, 268 still pass.
