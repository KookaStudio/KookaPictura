# Proposal: diffuse-glow

## Why

CS6 lists Diffuse Glow under Distort in both the Filter menu and the Filter
Gallery. Kooka has no kernel for it, so the gallery leaves it out and
`Filter ▸ Distort ▸ Diffuse Glow` is a dead leaf. Issue #286 asks for it.

## What Changes

- Add `Filter::DiffuseGlow { graininess, glow_amount, clear_amount, seed }`
  and its kernel in `distort/diffuse_glow.rs`. Each pixel mixes toward white
  by a fraction driven by its blurred luminance. Glow Amount lowers the
  threshold, steepens the ramp, and widens the halo. Clear Amount thins a veil
  over the whole picture and raises the threshold. Graininess speckles the
  mix. Alpha is untouched.
- Add the `diffuse-glow` kind (arity 4) with CS6's defaults 6 / 10 / 15 and
  a Seed slider, as Kooka's other grain filters have. The gallery lists Diffuse
  Glow first under Distort, as CS6 does, and the menu leaf opens its dialog.

## Capabilities

### New Capabilities

- `imaging/diffuse-glow`: the Diffuse Glow kernel and its parameters.

## Impact

- `pictura-filters` (`distort`, `Filter`) and `pictura-app` (`filter_map.rs`,
  `filter_commands.cpp`). No new dependency.
- **Oracle:** none. CS6's algorithm is closed. The model is fitted to CS6
  renders and covered by property tests.
- Out of scope: the Background swatch as the glow colour (white stands in, as
  for Neon Glow), CS6's bright band along the top edge, and Glass and Ocean
  Ripple, which #286 also lists.
