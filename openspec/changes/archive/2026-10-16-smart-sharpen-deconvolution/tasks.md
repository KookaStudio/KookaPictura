# Tasks: smart-sharpen-deconvolution

## 1. Measure

- [x] 1.1 Register the Photoshop screenshots against the source; estimate and fit the radial transfer function through clipping.
- [x] 1.2 Fit the core weight, core spread, and halo scale on both screenshots; record them in `design.md`.

## 2. Engine

- [x] 2.1 `sharpen::smart`: exact core inverse (one-pole IIR), halo solved by relaxed Van Cittert, Gaussian / fractional disc / bilinear line halos.
- [x] 2.2 More Accurate runs 8 iterations; remove `blur::motion_accurate` and the `*_with_support` Gaussian helpers.
- [x] 2.3 `preview_apron` reserves `3·radius` for Smart Sharpen.

## 3. Verification

- [x] 3.1 Unit tests: the core inverse undoes the core, the halo solve converges for every removal, the disc equals the naive fractional-span kernel, fine detail gains beyond Unsharp Mask, radius widens the halo, motion follows the Photoshop angle, uniform input is bit-identical at the extremes.
- [x] 3.2 The oracle mapping note and `tests/README.md` no longer claim Unsharp Mask equality.
- [x] 3.3 Mean error against the two screenshots recorded in `proposal.md`.
- [x] 3.4 `docs/06-filters/sharpen-filters.md` updated (`TASK-ALLOWS-DOCS`).
- [x] 3.5 `bash scripts/verify-full.sh`; `openspec validate --all --strict`.
