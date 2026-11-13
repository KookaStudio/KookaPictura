# Tasks

## 1. Tests named by the plan

- [x] 1.1 `pictura-app`: `a_large_shape_previews_from_the_view_at_its_level`, `a_stroked_shape_preview_scales_its_effect`, `a_shape_drag_past_the_margin_rebuilds_the_moving_image`.
- [x] 1.2 `pictura-render`: update `only_a_scale_free_document_reduces_faithfully` for a hidden styled layer.
- [x] 1.3 Qt: `tst_move_feedback::largeShapePreviewsFromTheView`, `tst_large_document::aLargeShapeMoveCommitLandsOffThread`.

## 2. Shape view preview

- [x] 2.1 `view_preview` accepts a shape layer, uses its reach as content, and checks backdrop fidelity with the shape hidden.
- [x] 2.2 `moving_layer_image` scales a shape's effect sizes by the level before the reduced composite.
- [x] 2.3 `follow` uses the reach so a shape drag past the margin rebuilds the moving image around the shifted view.

## 3. Backdrop fidelity

- [x] 3.1 `FilterProxy::reduces_faithfully` skips invisible layers.

## 4. Release regression

- [x] 4.1 Cover a large shape move commit deferring and landing with one history state.

## 5. Close-out

- [x] 5.1 `openspec validate --all --strict`, `check-test-plan.py`, one build, `verify-fast`.
- [x] 5.2 Archive the change.
