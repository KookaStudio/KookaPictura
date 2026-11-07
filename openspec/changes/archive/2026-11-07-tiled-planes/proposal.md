# Proposal

## Why

Issue #1213, step 5 of #1209 (bounded memory for large documents). A
fractured `Plane` splits into 64-row bands that span the **full width** of the
canvas. One pixel written on a 17000-wide layer forks 17000 × 64 bytes per
channel, so the cost of an edit grows with the width of the document, not with
the size of the edit. Bands also cannot say "this region is one value", so a
white Background or an opaque alpha channel costs its full area (≈ 1.16 GB at
17000²).

The streaming access of #1212 assumed contiguous full-width rows (`row()`
returns `&[T]`, `write_bands` hands out full-width `&mut [T]`). A square tile
cannot lend such a row, and an `Empty` or `Uniform` tile has no storage to lend
at all. By owner decision this change also finishes the #1212 migration
(paint, select, app, codec), moving every caller to tile access or to row
copies.

## What Changes

- `Plane` gains a tiled form: square tiles of 256 × 256 samples per channel,
  each `Empty`, `Uniform(v)` or a shared copy-on-write `Dense` allocation. A
  plane stays flat until it is tiled; flat planes keep their contiguous
  slice.
- A write forks only the tiles it touches. Written tiles whose samples are all
  equal collapse to `Uniform` (or `Empty` at the plane's default) when the
  write settles. Each tile carries a generation for caches; the whole-plane
  `stamp` and `ident` keep their guarantees.
- The band API (`fracture`, `band`, `bands`, `write_band`, `write_bands`,
  `shares_band`, `row`, `row_mut`, `rows`, `rows_mut`, `for_each_band`,
  `for_each_band_mut`, `BAND_ROWS`) is **BREAKING**ly replaced by `tile`,
  `for_each_tile`, `tiles_mut`, `for_each_tile_mut`, `shares_tile`,
  `read_row_into`, `write_row_from` and `settle`.
- New documents build their Background from `Uniform` tiles; empty layer
  growth, stroke scratch and history reuse tiles.
- The memory gate's `fork_bytes` counts tiles and sample bytes.
- PSD write reads rows across a row of tiles; PSD read fills tiles; the output
  is byte-identical.
- The remaining #1212 callers in paint, select, app and codec stop asking a
  tiled plane for a contiguous slice. Whole-document operations that still do
  are marked `ponytail:` and counted.

## Capabilities

### New Capabilities

- `document/tiled-planes`: tiled plane storage, uniform collapse, tile
  identity, and the memory bounds it buys.

### Modified Capabilities

- `document/plane-access`: the streaming API moves from bands and borrowed
  rows to tiles and row copies.

## Impact

- `pictura-core`: `plane.rs`, `plane/access.rs`, new `tiled_plane.rs`,
  `layer_bounds.rs`, `samples.rs`, `lib.rs`.
- `pictura-paint`: stroke rasterizer, fill, gradient, bucket, eraser, healing.
- `pictura-render`: compositing rows, view pyramid, document composite store,
  Move, Free Transform.
- `pictura-select`, `pictura-codec` (PSD read and write).
- `pictura-app`: history planes and budget, memory gate, new document, level-0
  frames, previews and selection UI, memory guards.
- No new dependency, no document-format change, no golden change.
