# Proposal: filter-alpha-family-contracts

## Why

Issue #406, from the review of PR #405. The `filter-alpha-allowlist` fold
(#248) hands every kind `filters_alpha` names a four-plane buffer, but four
of those kernels still capped at three planes — Shape Blur, Smart Blur
(Normal), Lens Blur, and Liquify — so an unlocked layer's edge no longer
moved under them, a regression against the deleted grey pass. (Field and Iris
Blur shared `pyramid_blur`, which #412 already widened to four planes.)
Separately, six family specs still carry an "Alpha channel preservation"
requirement saying every filter in the family leaves channel 4 bit-identical,
which contradicts the `filter-application` contract and the Offset
requirement the same fold rewrote.

## What Changes

- Shape Blur and Liquify run over every plane; Lens Blur defocuses the alpha
  plane without the specular bloom or the noise; Smart Blur's Normal mode
  averages alpha over the same taps the colour admits (the edge modes paint
  colour only and keep alpha).
- A layer-level regression test runs every `filters_alpha` kind over a soft
  alpha edge and requires the edge to move, so a kernel that ignores the
  fourth plane fails CI.
- The six family alpha requirements are replaced by ones that state which variants move channel 4 and
  which leave it bit-identical; `filter-application` names Liquify and the
  Smart Blur mode split.

## Capabilities

### Modified Capabilities

- `imaging/blur-filters`, `imaging/sharpen-filters`, `imaging/noise-filters`,
  `imaging/other-filters`, `imaging/pixelate-filters`,
  `imaging/distort-filters`: the stale "Alpha channel preservation"
  requirement is removed and replaced by "<Family> alpha follows the kernel",
  which follows `filters_alpha`.
- `imaging/filter-application`: the moving-kernel list names Liquify and
  Smart Blur's Normal mode.

## Impact

`crates/pictura-filters/src/blur/{shape,smart,lens}.rs`,
`crates/pictura-filters/src/distort/liquify/resample.rs`, their unit tests,
the kernel digest baseline (alpha bytes only), and
`crates/pictura-render/src/filter/tests.rs`. No new dependencies.
