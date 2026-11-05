# Design

## Work box

`fills::apply` computes `FillTarget::bounds`: the selection mask's cached
bounds (`selection_mask_bounds`) clipped to the canvas, or the canvas without a
selection. A mask target clips them to the mask's rectangle and moves them into
the mask document. A selection that covers nothing returns before any work.

Each fill hands the box to the engine's bounded path, `fill_layer_within`, which
forks only the 64-row bands the box crosses and visits only their rows:

- Paint Bucket: `bucket::flood` builds a box-sized match plane read row by row
  from `BucketSource` (a layer's planes, or the composite's), then
  `mask_ops::flood_from` floods from the seed alone; `bucket::fill_within`
  writes the box.
- Fill: `bucket::fill_within` with no mask (full coverage in the box).
- Gradient: `gradient::draw` takes `within` and answers no colour outside it.
- Stroke: the window is the selection's box grown by the stroke's reach and one
  pixel, clipped to the canvas. That ring holds no selected pixel, so the
  clamped morphology border at the window's edge sees zeros exactly where the
  whole-document filter did: the band is bit-identical.

## Selection wall

The match plane treats an unselected pixel as a non-match, so the contiguous
flood stops at the selection edge. This changes the result only where a
matching path left the selection and came back in. Before anti-aliasing,
unselected pixels in the box are set to filled: they are walls, not edges of
the colour region, and the fill's selection keeps them unpainted. A uniform
layer in a rectangular selection therefore fills solid, as before.

## Gate

`fills/budget.rs`: `admit(bytes)` compares an estimate to three quarters of
`pictura_codec::available_memory()` (no limit when unreadable) and records a
refusal in a thread-local the bridge reads back (`fill_refused_for_memory`).
`apply` clears it first. The estimates are:

| Fill | Peak |
|---|---|
| Bucket | 2 × box (match + flood) + max(box, forked bands) |
| Fill, Gradient | forked bands |
| Stroke | 4 × window (crop, dilate, erode, band) + forked bands |

Forked bands: planes × band-aligned rows the box crosses × layer width.
ponytail: 8-bit planes only; a deep document's native store is not charged.

A test hook, `set_fill_available_memory_for_test`, stands in for the host's
available memory.
