# Proposal: mixer-brush-tool

## Why

The Mixer Brush (issue #16) was catalogued but disabled. photorust
(perfecto25/photorust) ships it as `mixer::MixerBrush`, a reservoir/pickup
wet-paint model. This change ports it onto Kooka's paint engine and tool
framework (`docs/03-tools/mixer-brush.md`), completing the `B` group.

## What Changes

- `pictura_paint::mixer`: `MixerOptions` (Wet, Load, Mix, Flow) and the per-dab
  `MixerBrush`: a tip-weighted premultiplied pickup, a deposit that mixes the
  reservoir with the pickup by Mix and slides to pure pickup as Load runs out,
  a reservoir that absorbs what a wet brush crosses, and a transparency lock
  that keeps alpha.
- A `StrokeKind::Mixer` stroke (see `color-replacement-tool`) and
  `Stroke::mixer_reservoir`, so the brush's paint carries to the next stroke.
- `cxxqt_object/paint_tools.rs`: `begin_mixer_brush`, `mixer_reservoir`; one
  `"Mixer Brush Tool"` state per stroke through `end_paint`.
- `tool_mixerbrush.cpp`: the reservoir lives on the controller; Alt-click loads
  it from the image; choosing a foreground loads it; Load / Clean after each
  stroke (Clean wins).
- Options-bar row: Size, Hardness, the Current Brush Load swatch (Load Brush /
  Clean Brush), the Load / Clean toggles, the preset menu, Wet, Load, Mix, Flow
  (Load and Mix greyed while Wet is 0), and a disabled Sample All Layers.
  Catalog row enabled.
- C++ self-test `mixer_brush_tool` (541); `shift_plain` (117) asserts the B cycle.

## Capabilities

### New Capabilities

- `tools/mixer-brush-tool`: the Mixer Brush and its engine.

## Impact

- `pictura-paint` (`mixer.rs`, `stroke.rs`), `pictura-app` bridge and C++.
- No new dependency.

## Provenance

Ported from photorust's `core/src/mixer.rs`, `core/src/bridge.rs` (the
reservoir after-stroke rules), and `shell/src/MainWindow.cpp`
(`addMixerOptions`, `mixerPresets`) (<https://github.com/perfecto25/photorust>).
Behavioural parity only: the mixing, pickup, and dry-out rates and the preset
values are photorust's (the spec leaves CS6's open). Sample All Layers, Load
Solid Colors Only (sampled-variation loads), and bristle tips are `ponytail:`
ceilings.
