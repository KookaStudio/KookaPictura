# Proposal: psdrect-algebra

## Why

Issue #1162. Rectangles are the shared vocabulary of damage, clipping, filtering,
and compositing, but `PsdRect` (`crates/pictura-core/src/lib.rs`) exposes only
`width`/`height`. Every consumer therefore carries a private copy of the same
min/max arithmetic with subtly different edge behavior: some return an inverted
rectangle, some return `Option`, some normalize to empty, and the grow helpers
split on whether a non-positive pad is a no-op or a shrink.

There are 10 `PsdRect` intersect helpers (including `liquify::overlap`), 7
canvas-clamp helpers (including `liquify::clip`), roughly a dozen rect union
helpers, 5 grow helpers, and one `is_empty` helper across `pictura-core`,
`pictura-render`, `pictura-app`, `pictura-paint`, and `pictura-filters`. The
duplication is the only thing keeping the edge rules inconsistent; each copy is
individually trivial.

## What Changes

- Add the rectangle algebra to `PsdRect`: `is_empty`, `intersect`, `union`,
  `clip_to`, and `grow`, with saturating growth and one documented empty
  representation.
- Standardize the return shape: `intersect` and `clip_to` return `Option`, so a
  miss is explicit and no caller can accidentally index an inverted rectangle.
- Delete the private copies in `pictura-render`, `pictura-app`,
  `pictura-paint`, and `pictura-filters` and call the methods instead. Delete
  the helpers; do not wrap them.
- Add one edge-case unit-test set in `pictura-core` covering empty rects,
  clipping, and saturating growth.

No behavior change: the current bare helpers are already consumed as empty when
their extents are non-positive, so normalizing a miss to `None`/empty preserves
every caller's result. Damage, clipping, and compositing goldens must stay
byte-identical.

## Capabilities

No spec delta: this is an internal refactor with no requirement change
(`skip_specs: true`).

### New Capabilities

None.

### Modified Capabilities

None.

## Impact

- `crates/pictura-core/src/lib.rs` — `PsdRect` methods and `Default`.
- `crates/pictura-core/src/tests.rs` — the edge-case unit tests.
- `crates/pictura-render/src/` — damage, canvas damage, filter, filter proxy,
  crop, slices, clipboard, move content, align, merge, layer effects, GPU clip
  and raster, layer source, fill.
- `crates/pictura-app/src/` — helpers, view preview, path list, crop group,
  layer style, display preview, history, `impl_filters` (`expand` is the fifth
  grow helper), the `impl_paint`/`impl_transform` call sites, damage-policy
  tests.
- `crates/pictura-paint/src/` — healing and stroke helpers.
- `crates/pictura-filters/src/distort/liquify.rs` — the `Option` union body,
  `clip`, and `overlap`.
- No new dependency; no public API removal. `pictura_core::PsdRect` keeps its
  path.

## Provenance

Independent refactor; no Adobe behavior is involved.
