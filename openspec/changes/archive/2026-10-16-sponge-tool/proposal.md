# Proposal: sponge-tool

## Why

The Sponge tool (issue #31) was catalogued but disabled. photorust's
`core/src/tone.rs` moves colour toward or away from grey under a brush; the
port follows `docs/03-tools/dodge-burn-sponge.md`.

## What Changes

- `pictura_paint::tone`: `Tone::Sponge` with `SpongeMode::{Desaturate,
  Saturate}` and Vibrance on `ToneOptions`. A pixel's distance from grey (its
  luminance) is scaled by `1 ∓ Flow·coverage`; Vibrance eases off by the
  pixel's saturation (saturating what is already vivid, or draining what is
  nearly grey). Flow is not damped by the Exposure scale, so one stroke at the
  default is visible. Toning is once per pixel per stroke; alpha is kept.
- `cxxqt_object/paint_tools.rs`: `begin_tone`; one `"Sponge"` state.
- `tool_retouch.cpp`; the bar (`buildTonePage`) has the tip, Mode
  (Desaturate), Flow 50 %, and Vibrance (on). Catalog row enabled; the O group
  is complete.
- Qt Test `tst_retouch_tools::spongeTool`.

## Capabilities

### New Capabilities

- `tools/sponge-tool`: the Sponge tool.

## Impact

- `pictura-paint` (`tone.rs`), `pictura-app` (bridge, C++).
- No new dependency.

## Provenance

Ported from photorust's `core/src/tone.rs`
(<https://github.com/perfecto25/photorust>). Behavioural parity only.
Ceiling (`ponytail:`): no Airbrush toggle or pressure-driven Flow.
