# Proposal: hue-saturation-replace-color-shift

## Why

Issue #211, the follow-up to #200 / #210. The Hue/Saturation and Replace Color
requirements specify a pure HSL shift: Lightness moves HSL `L` and (for
Replace Color) Saturation adds toward 1. Both turn near-neutral pixels into
saturated colour. A dark pixel with JPEG noise has a high HSL `S` at a low `L`,
so lightening it in HSL, or pushing its `S` toward 1, makes vivid patches that
CS6 does not produce. CS6 Help also says Replace Color cannot replace pure gray,
black, or white with a colour (`docs/04-image-ops/adjustments/replace-color.md`),
but the specified HSL shift can.

## What Changes

- **Hue/Saturation:** the hue rotation and saturation scale stay in HSL. After
  converting back, Lightness blends each RGB channel toward white (positive) or
  black (negative), instead of moving HSL `L`. The CPU kernels (8-bit and native
  depth) and the GPU shader change together.
- **Replace Color:** the hue still turns in HSL. Saturation scales the pixel's
  chroma about its HSL lightness, by `1 / (1 − s)` when raising and `1 + s` when
  lowering, so a gray stays gray. Lightness then blends toward white or black,
  as Hue/Saturation does.
- **Replace Color dialog:** the Result swatch and the result-colour picker use
  the engine's model, through `replace_color_result` and
  `replace_color_shift_for`, instead of a C++ copy of the old HSL shift.
  Picking a colour for a gray sample now moves only Lightness.

## Capabilities

### Modified Capabilities

- `imaging/image-adjustments`: the Hue/Saturation and Replace Color
  requirements specify the new shift.

## Impact

- `pictura-adjust` (`color.rs`, `replace_color.rs`), `pictura-render` (GPU
  `adj_hs`), and `pictura-app` (Replace Color bridge and dialog). No new
  dependency.
- Output changes for any non-zero Lightness (Hue/Saturation and Replace Color)
  and any non-zero Replace Color Saturation. Zero parameters remain an exact
  identity. No golden baseline covers these paths.
- Both models are approximations of closed Adobe kernels (`ponytail:` in code);
  no parity is claimed.
