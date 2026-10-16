# Proposal: port-photorust-blur-sharpen

## Why

Issue #222 (part of #183, after #221). The Blur and Sharpen family is the hard
part of the photorust port. Five of its thirteen filters have verified
ImageMagick parity, and most have GPU plans held within 1 LSB of the CPU. Each
filter was run through the oracle and the GPU parity suite. photorust's code
was taken where it held both, and Kooka's kept where it would regress either.

## What Changes

- **Ported to the photorust engine:** Surface Blur (a per-channel mean of the
  neighbours within the threshold, on a sliding histogram) and Sharpen Edges
  (an unsharp mask gated by a smoothstep of the Sobel edge strength).
- **photorust's models, Kooka's verified arithmetic:** Blur and Blur More are
  Gaussians at σ 0.7 and σ 2.0. Sharpen and Sharpen More are Unsharp Masks at
  50 % and 100 %, σ 1. Each runs through Kooka's oracle-exact Gaussian or
  Unsharp Mask, so all four gain an ImageMagick differential (Blur exact).
- **Kept on Kooka's code** (photorust diverges; recorded in `design.md`):
  Gaussian Blur, Box Blur, Median, Custom, Unsharp Mask, High Pass, Motion Blur.
- **photorust's speed, Kooka's semantics:** Median slides photorust's
  histogram with a clamped window, so it stays exact against ImageMagick at
  6× the speed. The shared Gaussian runs its rows on rayon.
- **GPU:** Blur, Blur More, Sharpen, and Sharpen More take the Gaussian and
  Unsharp Mask plans. The Surface Blur shader is rewritten to the new model
  and runs to radius 16; past that the CPU histogram is faster.

## Capabilities

### Modified Capabilities

- `imaging/blur-filters`: Blur and Blur More are fixed Gaussians; Surface Blur
  is a thresholded mean over the window clipped to the image; Blur and Blur
  More are diffed against ImageMagick.
- `imaging/sharpen-filters`: Sharpen and Sharpen More are fixed Unsharp Masks
  diffed against ImageMagick; Sharpen Edges fades in with the edge strength.
- `compositing/gpu-filter-acceleration`: Surface Blur's GPU radius cap and its
  performance evidence against the new CPU baseline.

## Impact

- `pictura-filters` (`blur`, `sharpen`, `noise`, `kernel`, `photorust`) and
  `pictura-render`'s `gpu_filter`. No `Filter` API change. `blur` and
  `sharpen` gain public constants for the fixed radii and amounts, which the
  GPU plans use.
- **Docs:** `docs/06-filters/blur-filters.md` and `sharpen-filters.md`
  describe the adopted models, marked as photorust's CS6-tuned approximations.
- **Output changes:** Blur, Blur More, Sharpen, Sharpen More, Sharpen Edges,
  and Surface Blur change. No golden baseline covers them. Median, Gaussian,
  Unsharp Mask, and High Pass are byte-identical.
- **Profile** (1024² RGB, release, `filter_profile_1024`): Surface Blur
  5.2 s → 0.03 s, Median 0.98 s → 0.16 s, Gaussian 1.48 s → 0.09 s, Unsharp
  Mask 0.09 s → 0.02 s, High Pass 0.29 s → 0.03 s. Blur, Blur More, Sharpen,
  and Sharpen More are faster than their 3×3 kernels were. Sharpen Edges is
  the one slowdown, 6 ms → 24 ms: its σ 1 blur and Sobel do more work than
  the old 4-neighbour gradient gate.
