# Proposal: smart-sharpen-deconvolution

## Why

Smart Sharpen was an unsharp mask with a swappable blur. Next to Photoshop it
looks wrong. Its radius was a 3σ support, so at the same setting the halo was
a third of Photoshop's width. The Lens Blur disc drew a hard, even-width white
outline. It also lifted every scale above the radius equally, which turns
large tones blotchy and leaves fine texture soft. Photoshop does the opposite:
it lifts fine texture hard and broad tones only moderately.

Two Photoshop screenshots of one JPEG were registered against the source
(Gaussian r 30.3 / 378 %, Lens r 9.9 / 405 %). The measured transfer function
has a moderate plateau above `1/radius` and a steep rise to Nyquist. It is the
same rise for both radii, and it amplifies the JPEG's 8×8 blocks. That curve
is the inverse of a sharp-core + halo point-spread function, not an unsharp
mask.

## What Changes

- Smart Sharpen deconvolves `H = 0.62·core + 0.38·halo`. The core is a
  separable 3-tap `[0.17, 0.66, 0.17]`, inverted exactly by a causal and an
  anticausal one-pole pass. The halo follows `remove`, at `0.8·radius`: a
  Gaussian σ, a disc with fractional-span rows, or a bilinear motion line of
  length `1.6·radius`. Relaxed Van Cittert iterations solve the halo part,
  starting from the input so flat areas stay exact.
- `out = original + d × amount/100 × gain`, where `d` is the deconvolved
  image minus the original. The noise floor scale drops from 26 to 10 levels,
  and the tonal fades are unchanged.
- More Accurate runs the halo solve to convergence (8 iterations instead of 3)
  instead of widening the blur estimate. `blur::motion_accurate` and the
  `*_with_support` Gaussian helpers existed only for the old path and go.
- The preview apron for Smart Sharpen grows from `radius` to `3·radius`.

## Capabilities

### Modified Capabilities

- `imaging/sharpen-filters`: Smart Sharpen is a deconvolution of the blur
  model instead of an unsharp mask, so the Gaussian path is no longer
  byte-identical to Unsharp Mask. More Accurate is the converged solve.

## Impact

- `pictura-filters` (`sharpen::smart`, `blur`, `kernel`) and
  `pictura-render`'s `preview_apron`. No `Filter` API or dialog change.
- **Output changes:** every Smart Sharpen result changes. No golden baseline
  covers it. Mean error against the two Photoshop screenshots drops from
  24.1 to 18.0 and from 18.9 to 13.7 levels.
- **Oracle:** Smart Sharpen was classified no-equivalent already. Its
  Gaussian path no longer rides on the Unsharp Mask differential, and the
  mapping note says so.
- **Cost** (1000×652 RGB, release): the screenshot settings take 0.14 s for
  Lens r 9.9 (0.21 s before) and 0.12 s for Gaussian r 30.3 (0.09 s before).
  Radius 64 with More Accurate takes 1.4 s for Lens and 1.7 s for Motion,
  because the disc and the line cost `O(radius)` per pixel per iteration.
- **Docs:** `docs/06-filters/sharpen-filters.md` describes the deconvolution
  model and drops the claim that Gaussian equals Unsharp Mask. It is a
  separate `TASK-ALLOWS-DOCS` commit.
