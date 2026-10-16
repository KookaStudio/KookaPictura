# Proposal: mask-stroke-preview-fallback

## Why

Issue #656 (related: #404): painting a layer mask with a large brush lags the
pointer. `mask-paint-performance` made the mask edit document single-channel and
gave a large mask brush the one-channel GPU stroke, and its requirement says a
large mask brush SHALL use "a reduced-level preview and/or a GPU stroke". When
the GPU stroke is not taken (GPU compute off or unavailable, or a Dissolve or
Behind brush, which only the CPU raster models), the mask stroke still runs the
exact full-resolution CPU raster on every dab, so a large brush rasterizes and
recomposites its whole bounding box per frame. The single-channel document is
also built by copying the whole mask at stroke start, with a composite the
stroke never reads.

## What Changes

- A mask dab over the raster budget that does not take the GPU stroke previews
  at the view-pyramid level the pixel preview uses. The preview interpolates
  the stroke-start level between the reduced composites with the mask hiding
  and revealing everywhere; the commit replays the recorded samples through the
  exact mask stroke and writes the mask back once.
- At the commit, the preview's level returns to its stroke-start snapshot
  before the exact stroke's regions refresh, so pixels only the preview painted
  (a dab crossing the mask's rectangle, a sample the exact spacing skips) do not
  stay on the canvas.
- The single-channel mask paint document shares the mask's coverage plane and
  has no composite, so a stroke start copies nothing.
- **BREAKING**: none.

## Capabilities

- Added `tools/mask-stroke-preview`: the CPU preview fallback for large mask
  dabs, its commit restore, and the copy-free stroke start.

## Impact

- `crates/pictura-app/src/cxxqt_object/impl_paint/mask.rs` (new; the mask paint
  path moves here from `impl_paint.rs`, which is at its size cap).
- `crates/pictura-app/src/cxxqt_object/state.rs` (`MaskPreview`).
- `crates/pictura-render/src/filter_proxy.rs` (`FilterProxy::mask_extremes`).
- `crates/pictura-render/src/document_ops/layer_ops/mask_edit.rs`.
- No `docs/` change, no new dependency.
