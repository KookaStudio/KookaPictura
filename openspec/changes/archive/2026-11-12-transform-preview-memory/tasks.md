# Tasks

## 1. Tests named by the plan

- [x] 1.1 `pictura-app`: `a_large_plain_layer_previews_at_a_reduced_level`, `a_small_or_styled_layer_previews_at_full_size`, `preview_bytes_follow_the_level`, `switching_to_skew_keeps_the_box_and_the_reference_point`, `projective_switch_refusals`.
- [x] 1.2 Qt: `tst_transform_overlay::reducedPreviewDrawsAtDocumentSize`, `tst_transform_distort::projectivePreviewSamplesAReducedImage`, `tst_transform_distort::distortCommandDisabledMidSessionOnType`, `tst_transform_warp::warpPreviewKeepsTheReducedLevel`, `tst_memory_governor::transformBeginRefusedBeyondTheRoom`, `tst_transform_commands::choosingSkewMidSessionKeepsTheTransform`, `tst_transform_commands::distortMidSessionCommitsThePendingScale`.

## 2. Reduced preview

- [x] 2.1 `transform_preview_level`, the threshold and its test seam; the reduced moving image and backdrop patch in `build_transform_preview`.
- [x] 2.2 `transform_preview_level` on the view state; `transform_preview_scale` reports it during a session; the Warp preview rebuilds at it; `transform_leave_warp` scales the image size.
- [x] 2.3 Canvas: `beginTransformPreview` takes the scale; the affine, projective, and CPU-warp draws scale the image; the warp source is converted to premultiplied once.

## 3. Gate

- [x] 3.1 `transform_preview_bytes`; `compute_transform_preview` admits it first.
- [x] 3.2 The C++ begin reports the transform memory message on a memory refusal.

## 4. Mode switch

- [x] 4.1 `can_set_mode` and `switch_mode` for projective target modes.
- [x] 4.2 Skew, Distort, and Perspective menu handlers switch an open modal session; their enabled state follows `transform_can_set_mode` then.

## 5. Close-out

- [x] 5.1 `docs/dev/STATE.md`.
- [x] 5.2 `openspec validate --all --strict`, `check-test-plan.py`, one build, `verify-fast`.
- [x] 5.3 Archive the change.
