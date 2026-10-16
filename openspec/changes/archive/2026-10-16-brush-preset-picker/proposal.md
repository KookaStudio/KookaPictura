# Proposal: brush-preset-picker

## Why

The brush bars showed plain Size and Hardness fields. CS6 opens its Brush
Preset picker from a tip button there instead: Size and Hardness with sliders,
and the default brush set (issue #77). The Size slider is uneven, so the
common small sizes get most of its travel. photorust has the picker, but its
presets need dab dynamics (scatter, count, jitter) that Kooka's stroke did not
have.

## What Changes

- `StrokeConfig` gains Scattering and Shape Dynamics: `count` dabs per step,
  each scattered up to `scatter` % of the diameter and shrunk / turned /
  flattened by the jitters, from the stroke's fixed seed (ported from
  photorust's `Brush::stamp`). No dynamics paints exactly as before. The
  Paint stroke only; per-dab tools ignore them.
- `PaintTip` carries the dynamics; `begin_brush` begins Brush / Pencil strokes
  from a `PaintTip`; `brush_dab_preview` paints one step of a tip, scaled to fit,
  for thumbnails.
- `BrushPresetPicker` (`panels/brush_preset_picker.*`): a popup with the tip
  preview, Size (1–5000 px; the slider's first half covers 1–100 px, the second
  climbs geometrically to 5000 px) and Hardness, the preset name, and photorust's
  44-entry CS6 default set with engine-drawn thumbnails. A preset writes the
  whole tip into the controller.
- Every brush bar (Brush, Pencil, the healing brushes, Color Replacement, Mixer
  Brush, the stamps, the History brushes, and the Eraser) shows a tip button
  (the tip with its size) in place of the Size / Hardness fields; a click opens
  the picker.
- `stroke.rs`'s tests move to `stroke/tests.rs` (pure move; the file passed its
  size cap).
- C++ self-test `brush_preset_picker` (549); `lpn_brush_resync` (307) and
  `lpn_paint_percent` (325) now read the picker.

## Capabilities

### New Capabilities

- `tools/brush-dab-dynamics`: scatter, count, and shape jitter per step.
- `ui/brush-preset-picker`: the options bar's tip button and picker.

## Impact

- `pictura-paint` (`StrokeConfig`, `Stroke`), `pictura-app` (bridge, options
  bar, new panel file).
- No new dependency.

## Provenance

Ported from photorust's `shell/src/panels/BrushPresetPicker.cpp` and the dab
dynamics of `core/src/brush.rs` (<https://github.com/perfecto25/photorust>).
The sampled-image tips (chalk, charcoal, spatter, grass) are approximated with
scatter and jitter, as in photorust. No cog menu, Use Sample Size, or preset
libraries — `ponytail:` ceilings.
