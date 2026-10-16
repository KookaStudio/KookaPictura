# Proposal: gradient-tool

## Why

The Gradient tool (issue #24) was catalogued but disabled. photorust's
`core/src/gradient.rs` draws a colour ramp along a drag; the port follows
`docs/03-tools/gradient-and-paint-bucket.md` (TOOL-024).

## What Changes

- `pictura_paint::gradient`: `Gradient` (straight-alpha colour stops,
  `sample`, `reversed`, `preview`), the five `GradientStyle`s, `GradientOptions`
  (Mode, Opacity, Reverse, Dither, Transparency), `draw`, and fifteen built-in
  gradients (`PRESET_NAMES`, `preset`), the first two and the last following
  the foreground and background colours.
- `pictura_paint::fill` (private): the per-pixel fill pass the Gradient and the
  Paint Bucket share — a Brush mode through a document-sized selection mask,
  honouring Lock Transparency and refusing locked pixels. The stroke's
  per-pixel blend becomes the shared `blend_pixel` (no behaviour change).
- `cxxqt_object/paint_tools/fills.rs` (a new bridge): `gradient_preset_count`,
  `gradient_preset_name`, `gradient_preset_strip`, and `draw_gradient`, which
  records one `"Gradient"` state.
- `tool_fills.cpp`: the drag shows its axis (Shift snaps to 45°) and draws on
  release. `options_bar_fill.cpp`: the gradient sample and preset menu, the
  five style buttons, Mode (Normal / Dissolve / Behind), Opacity, Reverse,
  Dither, Transparency. `ToolController::colorsChanged` redraws the sample.
  Catalog row enabled.
- Qt Test `tst_fill_tools::gradientTool` (first the self-test `gradient_tool`, code 552, now retired). `shift_plain` (117), `keys_shown` (116),
  and the guard (98) move off the G group.

## Capabilities

### New Capabilities

- `tools/gradient-tool`: the Gradient tool.

## Impact

- `pictura-paint` (`gradient.rs`, `fill.rs`, `stroke.rs`), `pictura-app`
  (bridge, C++).
- No new dependency.

## Provenance

Ported from photorust's `core/src/gradient.rs` and its CanvasView gradient drag
(<https://github.com/perfecto25/photorust>). Behavioural parity only: the
interpolation space, dither, and preset stops are photorust's, not Adobe's.
Ceilings (`ponytail:`): no Gradient Editor or noise gradients; Mode offers the
Brush modes only; 8-bit RGB only; the fill stays within the layer's rectangle.
