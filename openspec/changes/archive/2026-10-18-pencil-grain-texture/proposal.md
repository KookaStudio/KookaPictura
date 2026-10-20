# Proposal: pencil-grain-texture

## Why

Issue #731's `2B Pencil` stroke is still too uniform and solid next to the
owner's Photoshop reference. The current grain (30 %) is a single-pixel hash
that only dims the stroke; Photoshop's pencil reads as a lighter, clumpy paper
texture with 2–3 px grain and visible light patches. The owner asked for the
reference's graininess and randomness.

## What Changes

- `crates/pictura-paint/src/fill.rs`: add `grain_noise`, blending the fixed
  per-pixel hash with a bilinear value noise on a 2 px grid, so the grain
  forms clumps instead of single-pixel static.
- `crates/pictura-paint/src/stroke.rs`: use `grain_noise` for the stroke grain
  modulation.
- `crates/pictura-app/cpp/panels/brush_preset_picker.cpp`: raise the
  `2B Pencil` preset's grain from 30 % to 80 %, which lowers the stroke's mean
  coverage to roughly 60 % and matches the reference's lightness.
- Tests: `pictura-paint` grain tests gain a clump/determinism check and a
  coverage-range assertion; `tst_brush_tip_preview` expects grain 80 for
  `2B Pencil`.

## Capabilities

**Modified:** `ui/brush-preset-picker` (the 2B Pencil grain and texture).

**Added:** `tools/paint-engine` (the paper-grain requirement itself).

## Impact

- The 2B Pencil stroke is lighter and clumpier; the Non-Photo Blue Pencil
  keeps its 30 % grain, and no other preset sets grain.
- No new dependency. The noise is deterministic and stays on the CPU path.
- The exact Photoshop tip texture is unavailable; the clumpy noise is a
  non-parity approximation tuned to the owner's screenshot.
