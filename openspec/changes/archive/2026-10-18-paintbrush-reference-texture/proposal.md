# Proposal: paintbrush-reference-texture

## Why

Issue #731's owner supplied a high-resolution reference of the two pencil
strokes. The current `2B Pencil` grain (80 %) is too fine and too
high-contrast, and the `Non-Photo Blue Pencil` grain (30 %) is too uniform:
the reference strokes are softer-edged and mottled at a 2–3 px clump scale,
and the blue stroke is much lighter than a 30 % grain produces. Both tips are
also hard-edged at 100 % hardness, while the reference edges transition over
several pixels.

## What Changes

- `crates/pictura-paint/src/fill.rs`: rebalance `grain_noise` to mix the
  per-pixel hash with 2 px and 3 px bilinear value noise, keeping the fixed,
  deterministic, clumpy paper texture.
- `crates/pictura-app/cpp/panels/brush_preset_picker.cpp`: set both pencil
  presets to 65 % hardness, `2B Pencil` to 50 % grain, and `Non-Photo Blue
  Pencil` to 75 % grain.
- Tests: `pictura-paint` covers the two preset grain strengths and the
  revised noise; `tst_brush_tip_preview` expects the new preset values.

## Capabilities

**Modified:** `ui/brush-preset-picker` (the two pencils' tip hardness and
grain), `tools/paint-engine` (the paper-grain texture and its clump scale).

## Impact

- The pencils paint softer, lighter, and clumpier strokes that match the
  owner's reference at the same zoom.
- No new dependency or persisted option. The grain stays deterministic and on
  the CPU path.
- The exact Photoshop tip data is unavailable; the values remain a non-parity
  approximation tuned to the reference image.
