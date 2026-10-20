# Proposal: paintbrush-oval-wet-edges

## Why

Issue #731 follow-up: `Paintbrush Oval 45 pixels Multiply` does not look like
the owner's Photoshop strokes. Resampling the owner's ~770 % screenshot onto
its pixel grid shows a hard-edged oval dab at roughly 50 % roundness, tilted
about 50° with the major axis up and to the right. A single dab paints a flat
26/255 of coverage on white. A stroke's interior paints lighter (20/255) than
a single dab and has a 1–3 px darker rim: Photoshop's Wet Edges. Our preset
uses a soft (60 %) horizontal tip at 35 % roundness and has no Wet Edges, so it
paints a uniform grey about three times darker (64/255). The Brush's Multiply
mode also falls back to Normal, because `paint_mode_from` has no `multiply`
arm.

## What Changes

- `crates/pictura-paint`: `StrokeConfig::wet_edges` remaps the accumulated
  dab coverage through a wet-edge transfer curve, and `PaintMode::Multiply`
  multiplies the paint colour into the backdrop.
- `crates/pictura-app`: `PaintTip` and `PaintOptions` carry the Wet Edges
  flag, which the options JSON persists. `"multiply"` maps to
  `PaintMode::Multiply`. Wet-edge strokes stay off the GPU dab path, and the
  large-dab preview applies the same curve.
- `crates/pictura-app/cpp/panels/brush_preset_picker.cpp`: the oval preset
  becomes 45 px, 100 % hardness, 50 % roundness, −50° angle (the engine's
  sign for an up-right tilt), and 5 % spacing, with Wet Edges on. Every other
  preset clears Wet Edges.
- Tests in `crates/pictura-paint/src/stroke/tests.rs`,
  `crates/pictura-paint/src/tip.rs`, `tst_brush_tip_preview`, and
  `tst_tool_options_persistence`.

## Capabilities

**Modified:** `tools/paint-engine`, `ui/brush-preset-picker`

## Impact

- The oval preset paints a light grey stroke (about 20/255 coverage on white)
  with a darker rim, which matches the reference's tones.
- Brush strokes in Multiply mode now multiply instead of painting as Normal.
  The other blend modes the options bar lists still paint as Normal.
- No new dependency. Wet Edges has no Brush-panel toggle yet; only the preset
  sets it.
- Ceiling: the transfer curve is fitted to two measured tones (a single dab
  and a stroke interior), not to Adobe's algorithm. It is non-parity.
