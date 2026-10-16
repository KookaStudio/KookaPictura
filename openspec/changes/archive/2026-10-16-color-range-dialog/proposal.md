# Proposal: color-range-dialog

## Why

Issue #67: import the Color Range dialog from photorust
(`shell/src/dialogs/ColorRangeDialog.cpp`, engine `core/src/wand.rs`).
Select > Color Range… was an inert placeholder, and `pictura_select::color_range`
(a sampled-colour Chebyshev ramp) had no caller outside its own tests.

## What Changes

- `pictura-select` `color_range.rs` (new) replaces the old function:
  `ColorRangeSelect` (Sampled Colors, Reds … Magentas, Highlights, Midtones,
  Shadows) and a graded `color_range(img, select, target, fuzziness 0–200,
  invert)` — RGB distance for a sampled colour, hue windows (greys excluded)
  for the colour bands, lightness windows for the tonal bands.
- `cxxqt_object/color_range.rs` (new bridge): `color_range_available` (not
  32 bpc), `color_range_preview` (the mask of the composite scaled to fit the
  preview), and `color_range_apply` — a new selection, or the intersection
  with a live one (CS6's refine), as one "Color Range" state.
- `color_range_dialog.*` (new): Select, Fuzziness (spin + slider, default 40),
  the sampled-colour swatch and eyedropper, Invert, and the greyscale mask
  preview. It is non-modal: while its eyedropper is down,
  `ToolController::setCanvasSampler` routes a canvas press to the dialog under
  the eyedropper cursor instead of the active tool.
- Select > Color Range… opens it (enabled when available); switching
  documents cancels it.
- Tests: `color_range` unit tests (ported from photorust's) and the Qt Test
  `tst_color_range`.

## Capabilities

### New Capabilities

- `tools/color-range`: Select > Color Range.

## Impact

- `pictura-select`, `pictura-app` (bridge, C++). No new dependency.

## Provenance

Band model and dialog layout from photorust; behaviour from `SEL-005`
(`docs/08-selection/color-range.md`). The distance metric and band widths are
approximations (behavioural parity only). Ceiling (`ponytail:`): no Skin Tones,
Detect Faces, Localized Color Clusters, Out Of Gamut, plus / minus
eyedroppers, Image preview, Selection Preview modes, or Save / Load.
