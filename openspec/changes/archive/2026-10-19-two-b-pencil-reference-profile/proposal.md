# Proposal: two-b-pencil-reference-profile

## Why

Issue #731 follow-up: the `2B Pencil` stroke is wider, flatter, and noisier
than the owner's Photoshop strokes. Resampling the owner's ~670 % pixel-grid
screenshot onto document pixels shows a stroke about 7 px across. Its
cross-section is a soft hump (`.12 .41 .72 .76 .74 .57 .25` coverage) that
integrates to 3.7 px. Its grain is clumpy: the residual from a 5×5 mean has
sd 0.064, and horizontal neighbours correlate at 0.38. Ours paints a
flat-topped 9 px band (6.0 px effective width) because flow 100 at 8 %
spacing saturates every overlapped pixel. Our grain is more single-pixel
static (residual sd 0.081, neighbour correlation 0.24).

## What Changes

- `crates/pictura-app/cpp/panels/brush_preset_picker.cpp`: `2B Pencil`
  becomes 8 px at 25 % hardness and 25 % spacing (it was 10 px, 65 %, 8 %).
- `crates/pictura-paint/src/fill.rs`: `grain_noise` weights the 2 px and
  3 px clumps more and the per-pixel hash less, at lower contrast. This
  changes both pencils' texture.
- Tests: a stroke-profile test in `crates/pictura-paint/src/stroke/tests.rs`,
  a neighbour-correlation bound on the grain noise, and the 2B values in
  `tst_brush_tip_preview`.

## Capabilities

**Modified:** `ui/brush-preset-picker`, `tools/paint-engine`

## Impact

- The 2B Pencil paints a thinner, soft-shouldered, clumpy stroke that matches
  the reference's measurements. The Non-Photo Blue Pencil keeps its tip, but
  its grain becomes slightly softer and clumpier too.
- No new dependency. The size and values are fitted to one screenshot and
  are non-parity.
