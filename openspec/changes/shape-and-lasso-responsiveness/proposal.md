# Proposal

## Why

Issue #1185 (also #1096): on a 17000×17000 document the lasso and shape tools
are very slow. `scripts/profile-tools.py` now drives every tool in the toolbox,
and the 17000×17000 baseline (headless, GPU compute on, idle machine) measured:

- **Lasso commit 7.2 s, 12.7 s feathered, 12.8 s Add or Subtract; Polygonal
  Lasso 7.8 s; the Magnetic Lasso never finished.** `Selection::polygon` tested
  every document pixel against every edge, so the Magnetic Lasso's wire (one
  vertex per pixel) ran for hours. A feather blurred the whole mask; Add and
  Subtract combined it whole; the outline's bounds scan read all 289 MB.
- **The Magnetic Lasso press built the whole edge field**: three
  document-sized `f32` planes (3.5 GB) on one thread.
- **A shape layer took 6.0–8.6 s to create.** Its `rect` is the canvas, so the
  GPU fill source rasterized and uploaded a 1.16 GB canvas-sized buffer one
  sample at a time, and the CPU fill blended every canvas pixel.
- **Move on a shape layer took 15–22 s to press and 12–15 s to commit.** The
  moving image and the effects backdrop were rendered over the canvas, and the
  commit recomposited and diffed the whole canvas.
- **A Path Selection drag on a shape layer cost 8 s per pointer move**: every
  move recomposited the document.
- **A Fill Pixels shape took 2.1–2.4 s.** The coverage was a document-sized mask
  and the fill walked and copied every plane of the layer.

## What Changes

- **The lasso fill is a bounded scanline fill.** Each row of the polygon's
  bounding box computes its edge crossings once and fills the spans between
  them. The mask is unchanged: the crossing formula is the per-pixel test's.
  The fill reports its tight bounds.
- **A lasso or marquee commit works inside the shape's box.** The feather
  blurs the box grown by the kernel's reach, and Add, Subtract and Intersect
  combine only the box. A lasso whose bounds follow from its own (New, Add)
  seeds the selection-bounds cache, so the outline skips the full scan; the
  marquee keeps its one bounds read per selection (`mq_bounds_once`).
- **The Magnetic Lasso computes its edge field lazily.** The press finds the
  gradient peak in one parallel pass that keeps nothing; a trace computes the
  128-pixel tiles it reads, on first read.
- **A fill or shape layer composites, sources its effects and uploads its GPU
  fill only within its mask's box** (`fill_content_box`). Outside the box its
  coverage is zero, so the composite is unchanged.
- **Move on a shape layer renders and refreshes the shape's reach**: the moving
  image covers the shape's paint bounds, the backdrop patch covers its
  influence, and the commit's damage is its influence before and after.
- **A Path or Direct Selection drag on a shape layer refreshes the shape's
  influence before and after each step**, not the document.
- **A Fill Pixels shape rasterizes and fills its own box**, writing only the
  plane bands the box crosses.
- `profile-tools.py` gains a scenario for every toolbox tool.

## Capabilities

### New Capabilities

- `tools/shape-lasso-responsiveness`

### Modified Capabilities

None.

## Impact

`pictura-select` (`Selection::{polygon, polygon_bounded, feather_within,
combine_within}`, `EdgeMap`), `pictura-core` (`shape::coverage_box`),
`pictura-paint` (`bucket::fill_within`, banded `fill_layer_within`),
`pictura-render` (`fill_content_box` in the solid-fill composite, the effects'
source rect, the GPU fill source and coverage), and `pictura-app` (lasso
commit, bounds-cache seeding, the shape Move preview and commit, the path drag
refresh, Fill Pixels), plus `scripts/profile-tools.py`.
