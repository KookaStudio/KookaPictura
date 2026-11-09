# Tasks

## 1. Tests first

- [x] 1.1 `tst_move_feedback::maskedLayerPreviewsMasked` — a masked blue square
  drags with its hidden half transparent, matching the committed move.
- [x] 1.2 `pictura_app::a_view_preview_applies_the_layer_mask` — the view
  preview's moving image leaves the masked part transparent at level 0.

## 2. Moving image

- [x] 2.1 Add `preview::moving_layer_image`, which renders a layer whose
  channels alone are not its appearance over the preview region (mask, vector
  mask, style, fill, type) and reads raw channels otherwise.
- [x] 2.2 Use it for the full-resolution Move preview (`compute_move_preview`)
  and for the view preview's `moving_image`/`follow`.

## 3. Finish

- [x] 3.1 Run `cargo nextest run -p pictura_app`, `tst_move_feedback`, and the
  adjacent move/linked-layer Qt suites.
- [x] 3.2 Run `scripts/verify-fast.sh`.
- [x] 3.3 Archive the change.
