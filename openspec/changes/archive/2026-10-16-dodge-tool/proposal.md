# Proposal: dodge-tool

## Why

The Dodge tool (issue #29) was catalogued but disabled. photorust's
`core/src/tone.rs` lightens under a brush by tonal range; the port follows
`docs/03-tools/dodge-burn-sponge.md`.

## What Changes

- `pictura_paint::tone`: `ToneOptions` (Range — Shadows / Midtones /
  Highlights —, Exposure, Protect Tones) and the per-dab `ToneBrush`: a pixel's
  lift is Exposure × half the distance to white × a Gaussian weight of its
  luminance about the Range's centre. Protect Tones shifts the luminance and
  keeps the colour (so it cannot clip); off, the channels are scaled. The
  brush records the coverage applied per pixel, so a stroke tones a pixel once
  however many dabs cover it; a new stroke deepens it. Coverage never changes.
  `Stroke::begin_tone` runs it; 16/32-bit documents are refused.
- `cxxqt_object/paint_tools.rs`: `begin_dodge` (since generalised to `begin_tone` by `burn-tool`); one `"Dodge"` state.
- `tool_retouch.cpp`; the bar (`options_bar_retouch.cpp`) has the tip, Range
  (Midtones), Exposure 50 %, and Protect Tones (on). Catalog row enabled.
- Qt Test `tst_retouch_tools::dodgeTool`. `shift_plain` (117) now presses P
  as the unimplemented key.

## Capabilities

### New Capabilities

- `tools/dodge-tool`: the Dodge tool.

## Impact

- `pictura-paint` (`tone.rs`, `stroke.rs`), `pictura-app` (bridge, C++).
- No new dependency.

## Provenance

Ported from photorust's `core/src/tone.rs`
(<https://github.com/perfecto25/photorust>); Burn and the Sponge are left to
their own issues. Behavioural parity only: Adobe's Exposure curve is closed.
Ceiling (`ponytail:`): no Airbrush toggle or pressure-driven Exposure.
