# Proposal: native-depth-edit-preserve

## Why

The depth epic retains each layer's native samples (`Layer.source_channels`)
and the document composite (`Document.source_planes`) so an unchanged save
re-emits 16/32-bit samples instead of widening the 8-bit working model. But
every destructive geometry op drops or invalidates that store: a move leaves
`source_channels.rect` stale (silently disabling both the live native composite
and the save gate), a scale/rotate/projective/warp clears it, and a
canvas-resize/crop leaves `source_planes` sized to the old canvas. A 16/32-bit
document's layer channels are therefore re-widened from 8 bits after any edit.

## What Changes

- Move/translate (including the Move tool, crop, and canvas-resize) SHALL
  re-anchor `Layer.source_channels.rect` with the layer bounds instead of
  dropping the store, and canvas-resize/crop SHALL offset-blit
  `Document.source_planes` so its dimensions still match the document.
- `transform_layer`, `transform_layer_quad`, and `transform_layer_warp` SHALL
  resample every retained native plane through the same plane map as the 8-bit
  channels (dispatch on `u8`/`u16`/`f32`), set the store rect to the
  destination, and derive each stored 8-bit channel from the resampled plane's
  narrowing so the writer's equality gate holds.
- `Image > Image Size` SHALL resample the retained layer and composite stores at
  native precision; `Image Rotation`/flips SHALL remap them exactly (they are
  index permutations) and stop clearing them.
- Unmodeled raw on-disk channel streams (`Layer.raw_channels`), merge/flatten,
  via-copy, rasterize, Preserve Transparency, and GPU paths keep their existing
  behavior and remain documented ceilings.

## Capabilities

### New Capabilities

- `native-depth-edit-preserve`: preserving retained native samples through
  destructive geometry edits (move, transform, warp, resize, orient,
  canvas-resize, crop).

### Modified Capabilities

<!-- None: the writer's `narrow(retained) == current` rule and the read-time
     retention model are unchanged; this changes what the ops hand the writer. -->

## Impact

- `crates/pictura-render/src/document_ops/native_store.rs` (new): sample-typed
  offset blit, resize, and exact orientation remap of a flat plane store.
- `crates/pictura-render/src/document_ops/layer_ops/transform_native.rs` (new):
  sample-typed inverse-map resample and mesh warp; `transform.rs`/`warp.rs`
  resample the retained store and derive the 8-bit channel from it.
- `orientation.rs`, `resize.rs`, `canvas.rs`, `crop.rs`: keep/remap/resample the
  retained stores.
- Tests: transform/resize/orient/warp/canvas/crop unit checks plus an
  end-to-end 16-bit fixture round-trip in `tests/document/native_depth_roundtrip.rs`.
- No new dependency; `crates/pictura-codec/src/write.rs` is untouched and does
  not grow.
