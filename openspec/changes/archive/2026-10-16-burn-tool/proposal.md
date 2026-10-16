# Proposal: burn-tool

## Why

The Burn tool (issue #30) was catalogued but disabled. It is photorust's
`core/src/tone.rs` Dodge with its direction flipped; the port follows
`docs/03-tools/dodge-burn-sponge.md`.

## What Changes

- `pictura_paint::tone` gains `Tone::{Dodge, Burn, Sponge}` on `ToneOptions`;
  Exposure becomes `amount`, shared with the Sponge's Flow. Burn darkens by
  the same Range weight and Exposure scale: protected, the luminance moves to
  `l·(1 − k)` with the colour kept, so it cannot clip to black; unprotected,
  the channels are scaled down.
- `cxxqt_object/paint_tools.rs`: `begin_tone` (tool 0 Dodge / 1 Burn /
  2 Sponge) replaces `begin_dodge`; one `"Burn"` state.
- `tool_retouch.cpp`; the Dodge page becomes `buildTonePage`, one per tool
  with its own options (`ToolController::toneOptions(ToolId)`). The Burn bar
  has the tip, Range (Midtones), Exposure 50 %, and Protect Tones (on).
  Catalog row enabled.
- Qt Test `tst_retouch_tools::burnTool`. The guard (98) now probes Pen, and
  `keys_shown` (116) the 3D K group.

## Capabilities

### New Capabilities

- `tools/burn-tool`: the Burn tool.

## Impact

- `pictura-paint` (`tone.rs`), `pictura-app` (bridge, C++).
- No new dependency.

## Provenance

Ported from photorust's `core/src/tone.rs`
(<https://github.com/perfecto25/photorust>). Behavioural parity only: Adobe's
Exposure curve is closed. Ceiling (`ponytail:`): no Airbrush toggle or
pressure-driven Exposure.
