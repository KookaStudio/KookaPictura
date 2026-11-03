# Proposal

## Why

Issue #1149. On a 16507×16196 document, a Paint Bucket click that fills an empty
layer took about 8.4 s, and its undo about 3.4 s. `bucket_fill_profile` (1/16 of
the area on the development laptop) split the click into stages: the flood
(85 ms), the anti-alias pass (146 ms) and the fill (121 ms) were serial or
per-pixel work over the whole canvas; the region composite (285 ms) blended the
background that the now-opaque layer hides; the history capture (77 ms) and the
undo (79 ms) copied every changed tile twice.

## What Changes

- **The flood tests every pixel once, in parallel.** A contiguous flood walks the
  answers, and a surface whose every pixel matches the seed (an empty or uniform
  layer) is returned whole without a walk.
- **The anti-alias box runs in parallel from column sums** and copies a row with
  no edge in its neighbourhood. The output is unchanged.
- **A fill reuses the last blend in a run of equal pixels.** Dissolve still rolls
  per pixel.
- **A band of the CPU composite starts at the topmost plain opaque Normal layer**
  over it when everything below is provably opaque, and **copies that layer's
  bytes** when nothing is shown above it. The output is byte-identical.
- **A whole-plane change moves through the history by sharing.** A capture with
  no hint shares the live plane as the current state's instead of copying the
  changed tiles in, an undo or redo shares the restored plane with the live
  document instead of writing the tiles into it a second time, and the tile swap
  runs one band per core.

## Capabilities

### New Capabilities

- `tools/paint-bucket-fill-responsiveness`
- `compositing/occluded-band-skip`

### Modified Capabilities

- `document/in-place-history`: a capture with no hint shares the changed plane
  with the live document, and an in-place undo shares the restored plane.

## Impact

`pictura-paint` (`bucket::flood`, `eraser::antialias_mask`, `fill::fill_planes`,
shared with the Magic Eraser and Edit > Fill), `pictura-render`'s banded
compositor (`composite_region.rs`), the app history (`history/planes.rs`), and a
new ignored app profile `bucket_fill_profile`.
