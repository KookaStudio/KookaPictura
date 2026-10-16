# Design

## Decisions

- **Clip, not point clamping.** The alternative — clamp the decoded `vmsk`
  points to the canvas at decode — fills the same pixels inside the canvas but
  changes the path that `sync_vector_mask_block` writes back on save and that
  rasterize bakes, so it is rejected. Bounding the row table leaves the
  document model untouched.
- **The caller's clip is the sampled region.** The compositor samples the
  document canvas; the feather path samples a plane cut to the canvas grown by
  the feather's reach; `rasterize_vector_mask` samples the layer rect. Each
  passes the exact region it queries, so the table covers it plus one row each
  side. The feather path passes its plane bounds, not the canvas, so a ramp at
  the canvas edge still sees the coverage just outside it.
- **The table is a cache, not a correctness boundary.** The compositor samples
  the same mask over different partial regions across calls (dirty-rect
  refresh), so a table built for the first region's clip must not make a later
  region wrong. A row outside the cached table is answered by scanning the
  mask's edges directly — `O(edges)`, not `O(rows)` — so the clip bounds only
  the allocation, never the result. The feather radius grows the feather
  plane's clip, so `set_feather` retires the table to keep that plane on the
  indexed path; the `ident` moves off the table so a cache key can be taken
  before the first build without forcing an unbounded one.
- **`i64` arithmetic for the bounds.** `clip.top - 1` and `clip.bottom + 1`
  are computed in `i64` and clamped to the `i32` range, so neither an extreme
  clip nor an extreme path point can overflow.

## Sources

- The 8.24 point scale is in `crates/pictura-codec/src/vector_mask.rs`
  (`FIXED_ONE`, `scale`): a raw `i32` maps to `raw / 2^24 * extent * 256`
  1/256-px units, so a crafted raw point reaches about ±128× the extent.
- The sampled-row rule (`row y` samples the centre line `y * 256 + 128`) is
  unchanged from the existing `Rows::build`.
