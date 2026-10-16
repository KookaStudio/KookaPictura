# Proposal

## Why

`VectorMask::inside` builds a per-row edge-crossing table spanning every pixel
row between the path's top and bottom (`Rows::build`,
`crates/pictura-core/src/vector.rs`). Both bounds come straight from the file's
`vmsk` points, which are 8.24 fractions of the canvas and so can reach about
±128× the document height. A crafted path on a large canvas therefore allocates
one empty `Vec` header per row and walks every row — hundreds of MB and roughly
a second — the first time any vector-mask coverage is sampled, with no feather,
through ordinary compositing (`vector_mask::coverage`). The feathered path is
now clipped (#446) but still calls `inside`, so it builds the same table.

## What Changes

- Bound the row table to the rows that are actually sampled: the caller's clip
  plus one row each side, intersected with the path's own row span. An edge
  spanning past the clip contributes crossings only to the in-range rows.
- Keep the table a cache, not a correctness boundary: a row outside it is
  answered by scanning the edges directly, so a later caller with a different
  clip (a partial-region composite, or the feather plane grown past the canvas)
  still samples correctly.
- Thread the clip through `VectorMask::inside`, `vector_mask::coverage`, and
  `shape_coverage`. The compositor passes the document canvas; the feather path
  passes its plane bounds (the exact region it samples); the thumbnails pass the
  canvas.
- Decouple the mask's coverage `ident` from the row-table build, so a cache key
  can be taken without forcing an unbounded build, and retire the row table when
  the feather radius changes (which grows the feather plane's clip).

## Capabilities

### New Capabilities

- `compositing/vector-mask-sampling`: the per-row crossing table is bounded by
  the sampled clip, so file-controlled path points cannot size it.

### Modified Capabilities

<!-- none -->

## Impact

- `crates/pictura-core/src/vector.rs` — `Rows::build` takes a `PsdRect` clip;
  `inside` takes it; `ident` no longer builds the table; `set_feather` retires
  it.
- `crates/pictura-render/src/vector_mask.rs`,
  `crates/pictura-render/src/mask_refine.rs`, and
  `crates/pictura-render/src/document_ops/layer_ops/shape_layer.rs` — `coverage`
  / `shape_coverage` take the clip, and the feather path passes its plane
  bounds.
- `crates/pictura-app/src/cxxqt_object/helpers_composite.rs` and `shapes.rs` —
  the mask and shape thumbnails pass the canvas.
- No new dependencies, no document or codec behavior change; coverage inside the
  clip is unchanged.
