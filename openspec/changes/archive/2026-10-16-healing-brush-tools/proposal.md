# Proposal: healing-brush-tools

## Why

The Spot Healing Brush (issue #10) and Healing Brush (issue #11) were
catalogued but disabled. photorust (perfecto25/photorust) ships both in
`core/src/healing.rs`: a region is rebuilt from the pixels around it (Spot
Healing) or from an offset source with the destination's own lighting (Healing
Brush). This change ports them onto Kooka's paint engine, document model, and
tool framework.

## What Changes

- `pictura_paint::healing`: `RgbaImage`, `heal_region` (Spot Healing's three
  CS6 types — Proximity Match, Create Texture, Content-Aware patch synthesis),
  `clone_region` (Healing Brush's Poisson solve, `Transfer::Full` /
  `TextureOnly`), `heal_layer` (the layer wrapper), and `HealStroke` (the live
  coverage-mask gesture). Single-threaded (`ponytail:`).
- A `cxxqt_object/healing.rs` bridge: `healing_begin` / `healing_dab` /
  `healing_commit` / `healing_cancel`, one `"Spot Healing Brush"` / `"Healing
  Brush"` history state per committed gesture.
- `tool_healing.cpp` (Spot Healing and Healing handlers), options-bar rows
  (Spot Type; Healing Aligned), catalog rows enabled, toolbox J-group flyout.
- C++ self-test `healing_tools` (code 536).

## Capabilities

### New Capabilities

- `tools/healing-brushes`: the Spot Healing Brush and Healing Brush tools and
  their engine.

## Impact

- `pictura-paint` (`healing/`), `pictura-app` bridge and C++ as above.
- No new dependency.

## Provenance

Ported from photorust's `core/src/healing.rs`
(<https://github.com/perfecto25/photorust>). Behavioral parity only: Adobe's
biharmonic solver is closed (`docs/03-tools/healing-brushes.md`); the Laplace /
Poisson approximations are marked `ponytail:`.
