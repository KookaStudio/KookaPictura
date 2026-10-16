# Proposal: sharpen-tool

## Why

The Sharpen tool (issue #27) was catalogued but disabled. It is photorust's
`core/src/focus.rs` with its sign flipped; the port follows
`docs/03-tools/smudge-blur-sharpen.md`.

## What Changes

- `pictura_paint::focus` gains Sharpen: `Focus { Blur, Sharpen }` and Protect
  Detail on `FocusOptions` (was `BlurOptions`); the per-dab `FocusBrush` (was
  `BlurBrush`) reflects a pixel through its 3×3 Gaussian average
  (`2·dst − average`, a 3×3 unsharp mask at amount 1), clamped with Protect
  Detail to the neighbourhood's per-channel range; alpha is kept.
  `Stroke::begin_focus` replaces `begin_blur`; the Mode enum becomes
  `RetouchMode`, shared with Smudge.
- `cxxqt_object/paint_tools.rs`: `begin_focus` (Blur or Sharpen) replaces
  `begin_blur`; one `"Sharpen"` state.
- The retouch tools share `tool_retouch.cpp` (replacing `tool_blur.cpp`) and
  `options_bar_retouch.cpp`; each tool keeps its own options
  (`ToolController::retouchOptions`). The Sharpen bar has the tip, Mode,
  Strength 50 %, Sample All Layers, and Protect Detail (on). Catalog row
  enabled.
- Qt Test `tst_retouch_tools::sharpenTool`. The guard (98) now probes Burn.

## Capabilities

### New Capabilities

- `tools/sharpen-tool`: the Sharpen tool.

## Impact

- `pictura-paint` (`focus.rs`, `stroke.rs`), `pictura-app` (bridge, C++).
- No new dependency.

## Provenance

Ported from photorust's `core/src/focus.rs`
(<https://github.com/perfecto25/photorust>). Behavioural parity only: Adobe's
kernel is closed. Ceiling (`ponytail:`): no pressure-driven Strength.
