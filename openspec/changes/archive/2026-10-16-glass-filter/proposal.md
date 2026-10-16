# Proposal: glass-filter

## Why

CS6 lists Glass under Distort in both the Filter menu and the Filter Gallery.
Kooka has no kernel for it, so the gallery leaves it out and
`Filter ▸ Distort ▸ Glass` is a dead leaf. Issue #286 asks for it.

## What Changes

- Add `GlassTexture { Blocks, Canvas, Frosted, TinyLens }` and
  `Filter::Glass { distortion, smoothness, texture, scaling, invert }`, with
  its kernel in `distort/glass.rs`. Each surface is a procedural height map.
  Every pixel samples the source displaced along the map's slope. Distortion
  scales the reach, Smoothness blurs the map, Scaling sizes it, and Invert
  flips it.
- Add the `glass` kind (arity 5), with CS6's defaults of 5 / 3 / Frosted /
  100 % / off. The gallery lists Glass between Diffuse Glow and Ocean Ripple,
  and the menu leaf opens its dialog.
- Glass joins the filters whose alpha pass moves with the colour, as Ocean
  Ripple does.
- Diffuse Glow's private plane blur moves to `kernel::blur_plane`, which both
  filters now share.

## Capabilities

### New Capabilities

- `imaging/glass`: the Glass kernel, its surfaces, and its parameters.

## Impact

- `pictura-filters` (`distort`, `kernel`, `Filter`, `GlassTexture`),
  `pictura-render` (alpha allowlist) and `pictura-app` (`filter_map.rs`,
  `filter_commands.cpp`). No new dependency.
- **Oracle:** none. CS6's surfaces are bitmaps and its algorithm is closed.
  Covered by property tests.
- Out of scope: CS6's Load Texture… (a PSD as the surface).
