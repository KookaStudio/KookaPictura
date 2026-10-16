# Proposal

## Why

Issue #429: a Lasso, Elliptical Marquee, or Rectangular Marquee with a large
Feather (170 px on a 3000 px document) takes 5-7 s to commit. The selection
shape, history capture, and marching ants were already made fast by
`marquee-selection-speed` (#311); the remaining cost is `Selection::feather`,
which runs a direct separable Gaussian: `O(width * height * radius)` — 13.4 s
for a 3000x3000 selection at radius 170 in release, 33.4 s for the same
5000x3000 shape on an 8000x6000 document.

## What Changes

- **Variance-matched box stack for wide kernels.** At `sigma >= 2` (feather
  radius >= 4 px) the blur becomes three moving averages per axis whose widths
  `w_i` satisfy `sum (w_i^2 - 1) / 12 = sigma^2`. The ramp keeps its width,
  monotonicity, and 50 % crossing, a pixel costs `O(1)` instead of
  `O(sigma)`, and the intermediate stays 8-bit so the memory is
  `2 * width * height` bytes rather than the old `8 * width * height`.
- **Narrow kernels unchanged.** Radius < 4 px keeps the exact direct Gaussian
  byte for byte, so the ImageMagick differential (radius 2) still passes at
  tolerance 0.
- **BREAKING**: none. Feather output for radius >= 4 can differ from the
  previous direct Gaussian by up to 6 coverage levels on a hard edge.

## Capabilities

### Modified Capabilities

- `tools/selection-model`: the Feather requirement gains the box-stack
  approximation for wide kernels.

## Impact

- `crates/pictura-select/src/lib.rs`: `blur` dispatches to `gaussian_blur` or
  `box_stack`; new `box_widths`, `box_horizontal`, `box_vertical`.
- `crates/pictura-app/src/cxxqt_object/tests_profiles.rs`: ignored
  `feather_selection_profile`.
- No new dependencies.
