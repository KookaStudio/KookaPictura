# Port Edit ▸ Transform ▸ Warp from photocraft

## Why

`Edit > Transform > Warp` did not behave like Photoshop (#751). It was a modal
preset dialog applied in one shot, with preset meshes taken from Patchy's
reading of Adobe's styles. There was no on-canvas mesh, no custom warp, and the
resampler was a fixed 16×16 cell rasterizer that aliased when shrinking and
straightened thin layers. photocraft (MIT OR Apache-2.0, already a credited
source for Liquify) has a Warp that measures its preset shapes against
Photoshop renders, has an interactive bicubic Bezier mesh, and has an adaptive
resampler.

## What Changes

- **BREAKING** Remove the current Warp: `WarpPresetDialog`, the
  `apply_warp_preset` bridge, the `pictura_render` `WarpMesh` / `WarpParams` /
  `style_mesh` / `WarpStyle` API, the in-session preset warp, and the
  `lpr_warp_preset` self-test (exit code 527 is retired).
- Add `pictura_core::warp`, ported from photocraft's `geom::warp`: the fifteen
  preset styles with bend and H/V distortion, and the bicubic Bezier mesh with
  exact subdivision, surface pull, and point drag.
- Replace `transform_layer_warp` with photocraft's resampler: an adaptive,
  forward-mapped triangle mesh that area-averages when shrinking and draws
  later cells on top where the warp folds.
- Warp becomes a Free Transform mode, as in photocraft and CS6. The menu
  command or the options-bar toggle bakes the box into a one-patch custom mesh.
  Anchors and handles drag, a press elsewhere pulls the surface, and the options
  bar sets the style, orientation, Bend, and H/V. Commit applies the warp alone.
- Credit photocraft in `NOTICE.md` and the Move-and-Transform docs.
- Post-CS6 split warps and the Grid menu are not ported; smart-object warps and
  Warp Text keep their current behavior.

## Capabilities

- New: `document/transform-warp-engine`.
- Modified: `document/transform-session-warp`, `document/free-transform`
  (requirements removed), `ui/transform-options-bar`.

## Impact

- Code: `pictura-core/src/warp*`,
  `pictura-render/src/document_ops/layer_ops/warp*.rs`, `kernel.rs`,
  `transform_native.rs`; `pictura-app` `impl_transform/{warp,dispatch,session,bridge}.rs`,
  `state.rs`; C++ `tool_transform.cpp`, `options_bar_transform.cpp`,
  `image_view*`, `api/transform_api.*`, and `frame_menus.cpp`.
- Removed: `warp_preset_dialog.*`, `selftest_warp_preset.*`, `warp_styles*.rs`.
- Tests: new `tst_transform_warp`, plus Rust tests in core, render, and app.
