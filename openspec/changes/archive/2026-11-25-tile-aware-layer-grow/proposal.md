# Proposal

## Why

Long brush strokes on a 21,000² document take seconds each (#1609). The
profile in the issue paints 15 strokes of about 15,500 px with a 400 px hard
brush. Timed phase by phase in a release build, each stroke spends 1.4–6.7 s
growing the layer, about 175 ms on dabs and at most 17 ms recording history.
The 15 strokes take 60 s, 93 % of it in the grow.

A stroke grows the layer whenever a sample passes its edge: 121 times for the
first stroke and 32 times for each later one. Each grow rebuilds six planes
(the layer's four channels and the stroke's scratch and applied masks) through
`padded_plane`, which copies every pixel of the old layer row by row, empty
tiles included. A grow therefore costs the whole layer's area, which by the
last stroke is about 14,600 × 15,900 px.

## What Changes

- `padded_plane` moves a tiled plane tile by tile through the new
  `Plane::padded`. An empty tile costs nothing. A tile that lands on a whole
  tile of the new grid is shared, not copied; this covers every full tile when
  the layer grows right or down. Other tiles copy only their own samples.
- A flat plane keeps the old row copy.
- The layer's rect stays tight, so the pixels, the composite and the saved
  files do not change.
- A new ignored profile, `long_stroke_phases_profile`, times one stroke's
  phases apart.

## Capabilities

### New Capabilities

- `document/layer-grow-cost`: what a layer grow may copy.

### Modified Capabilities

None. `document/layer-content-bounds` still says when a layer grows; this change
says what the grow costs.

## Impact

- `crates/pictura-core/src/tiled_plane/pad.rs` (new), `plane/access.rs`
  (`Plane::padded`), `layer_bounds.rs` (`padded_plane`).
- `crates/pictura-app/src/cxxqt_object/tests_profiles/long_stroke.rs` (new
  profile); `restore.rs` shares its document setup.
- No new dependency, no UI change.
