# Proposal

## Why

The plain Brush/Pencil/Eraser first dab meets the `ips_first_dab` rule (4–6 ms
at 267 MP). The retouching and source tools never were brought under it: each
stroke start builds a document- or layer-sized surface serially. Dodge, Burn,
Sponge, Smudge, Blur, Sharpen, Mixer, Color Replacement, Background Eraser and
Art History interleave the layer through `layer_rgba`, which re-resolves the
channel list for every pixel; Sample All Layers builds a document-sized
composite and then crops it; Pattern Stamp builds a document-sized tiled image;
Clone Stamp and the History Brush rebuild their source surfaces every stroke.
Dodge/Burn and Clone Stamp are everyday retouching tools, so the stall is felt.

## What Changes

- Rewrite the stroke source builders — `layer_rgba`,
  `surface_from_composite`, and `layer_surface` — to resolve the plane layout
  once and interleave whole rows in parallel, byte-identical to the serial
  builders. Each returns an empty image for a zero-size source. `layer_local` is
  a crop with no channel planes; rewrite it as one parallel crop pass.
- Fold the Sample All Layers crop into the source build, removing the
  document-sized surface build and crop for Smudge, Blur, Sharpen and Mixer. The
  composite pass itself is unchanged and may still be document-sized when the
  picture is not current.
- Make the Pattern Stamp sample its tile per dab instead of building a
  document-sized pattern image at stroke start; a zero-size tile yields no
  source.
- Reuse a History Brush / Art History Brush / Erase to History source surface
  across strokes until the resolved source or layer path changes.
- Add an ignored stroke-start profile for Brush control, Dodge, Smudge with
  Sample All, Clone Stamp All Layers, History Brush, and Pattern Stamp at
  8000×6000, and record before/after numbers. The 8000×6000 size keeps the
  profile cheap; the 267 MP figure is the `ips_first_dab` measurement, not this
  profile.

Deliberately **not** in this change: a tile-backed `PerDab.pixels` and
tile-backed engine state, which would make stroke start dab-sized and actually
meet `ips_first_dab` at 267 MP. The parallel builders are still bounded by
memory bandwidth, so the existing `tools/in-place-stroke` bound remains unmet
for these tools until that follow-up. It is recorded in `design.md` and tracked
by a new issue.

## Capabilities

### New Capabilities

- `tools/stroke-source-surfaces`: how the layer, composite, history, and
  pattern surfaces a stroke reads are built, cropped, sampled and reused.

### Modified Capabilities

- None. `tools/in-place-stroke` keeps its current requirement; this change
  narrows the gap rather than claiming the bound.

## Impact

- `crates/pictura-paint/src/stroke.rs`, new `stroke/surface.rs`,
  `stamp.rs`, `art_history.rs`, `focus.rs`, `smudge.rs`, and `stroke/tests.rs`
  plus new `stroke/tests/surface.rs`.
- `crates/pictura-app/src/cxxqt_object.rs`,
  `cxxqt_object/paint_tools.rs`, `history.rs`, `history/tests.rs`, new
  `history/tests/cache.rs`, and a new `cxxqt_object/tests_profiles_retouch.rs`.
- `StampSource` gains a pattern variant and shares its image through an `Arc`;
  the paint engine's `sampled` surface becomes layer-local.
- No new dependencies; `rayon` is already used by the paint crate.
