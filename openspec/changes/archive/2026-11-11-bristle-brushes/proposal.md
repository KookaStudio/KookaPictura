# Proposal: bristle-brushes

## Why

`pictura-paint` has no bristle tip: `StrokeConfig` carries one smooth
procedural profile, the rasterizer stamps that one profile, and the Mixer Brush
holds one reservoir for the whole tip (issue #607, parity gap `CLR-10`). CS6's
Bristle tips draw a bundle of individual bristle tracks, so their edges are
ragged and their interiors streak, and the Mixer carries paint per bristle.
None of that exists.

## What Changes

- `pictura-paint` gains `BristleTip`, `BristleShape`, `BristleTrack`, and a
  `BristleDab` rasterizer. `StrokeConfig` carries `bristle: Option<BristleTip>`.
- A Paint stroke with a bristle tip draws the union of tapered bristle tracks
  per dab instead of one smooth profile. Shape, Bristles, Length, Thickness,
  and Stiffness change the streak count, length, width, and deformation.
- A Mixer Brush stroke with a bristle tip keeps one reservoir per bristle and
  deposits through the same track coverage, so wet pickup and dry deposit vary
  across the brush.
- `PaintTip` gains the bristle fields; the Brush panel's Brush Tip Shape page
  gains a Bristle Tip toggle and Shape/Bristles/Length/Thickness/Stiffness
  controls. The values persist in the session and the preset picker applies
  them.
- The GPU stroke path declines a bristle tip; the CPU bristle rasterizer is the
  oracle, as it already is for grain, Wet Edges, and dynamics.

## Capabilities

### New Capabilities

- `tools/bristle-brushes`: the bristle tip model, its shape/track geometry, its
  rasterizer, and the per-bristle Mixer reservoir.

### Modified Capabilities

- `ui/brush-panel`: the Brush Tip Shape page exposes the bristle tip options.
- `ui/brush-preset-picker`: the default set gains bristle presets and a preset
  applies the bristle tip fields.
- `ui/tool-options-persistence`: the bristle tip fields are saved and restored.

## Impact

- `crates/pictura-paint`: `lib.rs`, new `bristle.rs`, `tip.rs`, `stroke.rs`,
  `stroke/raster.rs`, `stroke/dynamics.rs`, `mixer.rs`.
- `crates/pictura-app`: `paint_tools` bridge, `paint_tools/imp.rs`,
  `impl_paint`, `tool_options.h`, `tool_options_json`, `paint_tip.h`,
  `panels/brush_panel`, `panels/brush_preset_picker`.
- No new dependency. No document or PSD format change.

## Provenance

`docs/07-color-painting/bristle-brushes.md` (BRU-003). Adobe's bristle
simulation, shape enumeration, defaults, and paint-transfer physics are closed;
this change is behavioural parity only, with the shape families taken from the
documented secondary sources and marked non-parity.
