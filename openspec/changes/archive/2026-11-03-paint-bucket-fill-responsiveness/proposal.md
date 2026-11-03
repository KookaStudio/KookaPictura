# Proposal

## Why

Issue #1149. On a 16507×16196 document, a Paint Bucket click that fills an empty
layer took about 8.4 s, and its undo about 3.4 s. `bucket_fill_profile` (1/16 of
the area on the development laptop) split the click into stages: the flood
(85 ms), the anti-alias pass (146 ms) and the fill (121 ms) were serial or
per-pixel work over the whole canvas. The region composite and the history's
tile copies are shared costs, handled in their own change
(`occluded-bands-and-shared-history`).

## What Changes

- **The flood tests every pixel once, in parallel.** A contiguous flood walks the
  answers, and a surface whose every pixel matches the seed (an empty or uniform
  layer) is returned whole without a walk.
- **The anti-alias box runs in parallel from column sums** and copies a row with
  no edge in its neighbourhood. The output is unchanged.
- **A fill reuses the last blend in a run of equal pixels.** Dissolve still rolls
  per pixel.

## Capabilities

### New Capabilities

- `tools/paint-bucket-fill-responsiveness`

### Modified Capabilities

None.

## Impact

`pictura-paint` (`bucket::flood`, `eraser::antialias_mask`, `fill::fill_planes`,
shared with the Magic Eraser and Edit > Fill) and a new ignored app profile
`bucket_fill_profile`.
