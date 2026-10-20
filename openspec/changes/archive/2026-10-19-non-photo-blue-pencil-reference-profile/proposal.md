# Proposal: non-photo-blue-pencil-reference-profile

## Why

Issue #731 follow-up: the `Non-Photo Blue Pencil` stroke is far wider than
the owner's Photoshop strokes. Resampling the owner's ~670 % pixel-grid
screenshot onto document pixels shows a line about 3.5 px across. Its
cross-section is `.09 .44 .68 .44 .04` coverage of `#73d6ff`; it integrates to
1.8 px and peaks at 0.74 (10th/90th percentile 0.59/0.84). Ours, a 9 px tip at
65 % hardness, paints a flat 8.6 px band (4.8 px effective width).

## What Changes

- `crates/pictura-app/cpp/panels/brush_preset_picker.cpp`: `Non-Photo Blue
  Pencil` becomes 4 px at 0 % hardness and 20 % spacing with a 55 % grain (it
  was 9 px, 65 %, 8 %, 75 %).
- Tests: a stroke-profile test, the grain-55 strength in
  `grain_strength_matches_the_pencil_presets`, and the preset values in
  `tst_brush_tip_preview`.

## Capabilities

**Modified:** `ui/brush-preset-picker`, `tools/paint-engine`

## Impact

- The blue pencil paints a thin, grainy sketch line that matches the
  reference's measurements. The grain noise is unchanged.
- No new dependency. The values are fitted to one screenshot and are
  non-parity.
