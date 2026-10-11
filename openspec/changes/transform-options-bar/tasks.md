# Tasks: transform-options-bar

## 1. Engine — interpolation

- [x] 1.1 Add `pictura_render::Interpolation` and the sampling kernels
  (`layer_ops/kernel.rs`): nearest, bilinear, Mitchell–Netravali bicubic
  (Bicubic/Smoother/Sharper), and the automatic area-ratio resolver; `from_id`
  over the six ids. Unit tests: nearest replicates, bilinear unchanged,
  integer translations are byte-exact for every interpolating kernel, automatic
  picks sharper down / smoother up.
- [x] 1.2 Add `transform_layer_with`, `transform_layer_quad_with`, and
  `transform_layer_warp_with`; thread the interpolation through
  `resample_plane`, `transform_native::resample_store`, and the warp sampler;
  the existing ops delegate with bilinear. Tests: a 400% nearest scale has no
  new colours, a 200% bicubic scale differs from bilinear, refusals still leave
  the document bit-identical.

## 2. App — session state and bridge

- [x] 2.1 Session state: `pivot`, `pivot_origin`, `show_pivot`, `skew_x/y`,
  `interpolation`, `warp`; default pivot = source-rect centre, show on,
  Bicubic.
- [x] 2.2 Pivot geometry (`geometry.rs`): preset anchors of the live quad,
  drag hit-test (a new handle id), snap to the 9 anchors within 6 screen px,
  `set_pivot` translation compensation; rotate-about-pivot gesture; shear in
  `gesture_scale`. Unit tests.
- [x] 2.3 Bridge setters/getters (`session.rs`): pivot set/preset, show,
  scale (link), angle, skew, interpolation, warp set/clear; percentage and
  degree readouts (stored in Free, derived in projective); `transform_quad`
  and `transform_preview_matrix` for every mode; the warp preview image.
- [x] 2.4 Commit: shear via the affine quad, interpolation on all ops,
  transform-then-warp, one `"Free Transform"` state; cancel/identity still
  restore bit-identically. Tests where reachable from Rust.

## 3. App — C++ surface

- [x] 3.1 `ToolController::transformChanged` (begin/commit/cancel, gesture
  move, pivot press) and the transform-page switching in the frame.
- [x] 3.2 `OptionsBar::buildTransformPage` (`options_bar_transform.cpp`):
  tool icon, show-pivot toggle, 3×3 locator, X / relative / Y, W / link / H,
  angle, H/V, interpolation combo, warp button (`WarpPresetDialog`), Cancel,
  Apply; refresh from the session on `transformChanged`.
- [x] 3.3 Canvas pivot: `ImageView` draws the reference point (target marker)
  when visible; `ToolController` hit-tests and routes the drag, cursor; snap
  feedback.
- [x] 3.4 New `options.warp` custom icon + `lucide-map.json` provenance entry.

## 4. Verification

- [x] 4.1 `crates/pictura-app/cpp/tests/tst_transform_options.cpp`:
  page shown on begin and restored on end; fields seeded; pivot locator and
  X/Y move the pivot; W/H + link change the scale; angle/H/V setters; the
  interpolation combo reaches the session; canvas pivot drag snaps to an
  anchor; warp set/clear updates the session; Cancel/Apply buttons end the
  session and Apply records one state.
- [x] 4.2 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D
  warnings`, `cargo nextest run --workspace`, `ctest --test-dir build -R
  '^tst_'`, `bash scripts/verify-fast.sh`, `openspec validate --all --strict`.
