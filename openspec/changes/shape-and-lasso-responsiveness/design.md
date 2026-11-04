# Design

## Measurement

`scripts/profile-tools.py` gains one scenario per toolbox tool (selection,
crop and measure, retouch, painting, pen and type, every shape kind and mode,
navigation) and setup-based ones: a shape moved with the Move tool (plain and
stroked) and dragged with Path and Direct Selection, lassos that add to and
subtract from a rectangle, a patch from a selection, and the healing and clone
tools with an Alt-click source. A scenario may hold modifiers, finish with a key
(Crop's Enter), or leave no history. The Polygonal Lasso and Pen click a ring of
eight vertices closed on the first; the Magnetic Lasso clicks, hovers along the
ring and closes.

The baseline ran on a new 17000×17000 white document, headless, GPU compute on,
30 motion events. The GUI thread was sampled with `gdb` where the phases did not
add up. Times are milliseconds; "up" is the release, or the closing click.

| Scenario | Before up | After up | Before move p95 | After move p95 |
|---|---|---|---|---|
| Lasso | 7185 | 128 | 1.9 | 1.8 |
| Lasso, feather 20 | 12654 | 353 | 2.3 | 2.3 |
| Lasso Add | 12821 | 482 | 1.6 | 4.1 |
| Lasso Subtract | 12760 | 462 | 4.1 | 2.9 |
| Polygonal Lasso | 7829 | 454 | 2.3 | 1.6 |
| Magnetic Lasso (press / close) | press 7800, close never ended (killed after 15 min) | 232 / 399 | – | 42 |
| Rectangle shape | 7512 | 1121 | 2.5 | 2.3 |
| Ellipse shape | 6557 | 935 | 2.8 | 1.4 |
| Rounded rectangle shape | 6678 | 1135 | 3.2 | 1.9 |
| Polygon shape | 8562 | 2727 | 2.2 | 1.6 |
| Line shape | 5984 | 22 | 1.7 | 1.8 |
| Custom shape | 6519 | 905 | 2.1 | 1.9 |
| Rectangle, Fill Pixels | 2406 | 878 | 1.9 | 2.2 |
| Ellipse, Fill Pixels | 2051 | 673 | 1.8 | 1.8 |
| Move a shape (press / up) | 15346 / 12603 | 1873 / 1180 | 6.3 | 4.8 |
| Move a stroked shape (press / up) | 22349 / 15527 | 3738 / 2877 | 8.2 | 4.6 |
| Path Selection drag on a shape | 58 | 1321 | 12831 | 5.6 |

The path drag composites on its release (owner decision): the release pays the
one region composite the steps used to pay each.

### The Photocraft bar

Photocraft (`storytold/photocraft` 6a2aaebe, release build) was driven with the
same gestures through its control port (`ui.pointer`, then `ui.inspect` to catch
the frame) on a 17000×17000 new document under Xvfb. There it renders on
llvmpipe (its CPU path), so every event carries a frame floor of 47–90 ms;
Pictura ran headless with GPU compute. Read the rows as an order of magnitude.

| Scenario | Photocraft | Pictura after |
|---|---|---|
| Rectangle / Ellipse shape (up) | 1081 / 1073 | 1121 / 935 |
| Lasso (up) | 220 | 128 |
| Polygonal Lasso (close) | 390 | 454 |
| Magnetic Lasso (press / move p50 / close) | 220 / 112 / 402 | 232 / 39 / 399 |
| Move a shape (press / move p50 / up) | 140 / 432 / 590 | 1615 / 3.5 / 1303 |
| Path Selection drag (move p50 / up) | 92 / 1285 | 3.9 / 1321 |

The shape Move press is the one row still well behind: it renders the shape and
its backdrop at full resolution over the shape's box, where the large-layer view
preview (#1148) would build them at the shown level. It is a follow-up.

The polygon covers nearly the canvas (the tool draws from its centre), so its
region is the document. The shape undo costs 330–1700 ms on both builds: the
part-2 baseline's ~10 ms undos came from history left by earlier scenarios.

What the time was, before:

- **`Selection::polygon`** tested each pixel against each edge, O(W·H·N). At 30
  vertices it took about 7 s; the Magnetic Lasso's wire has one vertex per
  pixel, tens of thousands, so its close ran for hours.
- **The feather** blurred the whole 289 MB mask on one thread, and **Add,
  Subtract and Intersect** combined every pixel.
- **`EdgeMap::from_buffer`** built luma, magnitude and cost as full-document
  `f32` planes on one thread.
- **`fill_source`** (the GPU fill upload) allocated a canvas-sized buffer and
  wrote it one sample at a time through `Plane` indexing. Sampling showed it on
  every shape composite, every Move press and every Move commit.
- **The Move tool** rendered a shape's moving image over its `rect` (the
  canvas), refused a backdrop patch for any fill layer, and committed with a
  dirty rect of `rect` before and after: the canvas.
- **`paths::edit_target`** called `recomposite()` on every drag step.
- **Fill Pixels** built a document-sized coverage and `fill_eight_bit` took
  every plane with `as_mut_slice`, copying each history-shared 289 MB plane.

## Lasso fill

`polygon::fill` lists the polygon's non-horizontal edges with their y range,
then fills 64-row bands in parallel. Each band keeps the edges its rows can
meet; each row computes `xi + (cy - yi) / (yj - yi) * (xj - xi)` for the edges
that cross its centre (the per-pixel test's own expression and half-open rule),
sorts them, and fills between pairs. A pixel is inside under even-odd when an
odd number of crossings lie at or left of its centre. Each span starts at the
first pixel whose centre is at or right of the entering crossing, found from a
rounded guess and settled by the direct comparison, so the mask is
byte-identical to the per-pixel test's. The rows' spans give the tight bounds.

## Bounded commit

`Selection::feather_within(radius, bounds)` blurs only `bounds` grown by
`blur::reach(sigma) + 1`, then writes the result into zeroed pages. A pixel past
the reach reads only zeros, and the window's own clamped edge repeats a zero, as
the neighbour it stands in for would. At the canvas edges the window is the
canvas, so the clamping is the same. The blurs sum whole numbers in `f32` far
below 2²⁴, so the result is bit-identical. Both blur passes now run rows (and,
for the vertical box, 64-row bands seeded at their first row) in parallel.

`Selection::combine_within(other, op, bounds)` combines only `bounds` (in
parallel rows) when `other` is zero outside it; Intersect clears the rest.

`combine_selection_boxed` takes the shape's `ShapeBox` (zero-outside box,
tight or not). A New lasso seeds the bounds cache with its own bounds, and an
Add seeds the union with the old ones. The seed is keyed by the stored plane's
ident, so a later write rescans as before. The Marquee passes its box untight:
it keeps its one read per selection (`mq_bounds_once`). The control server's
selected-pixel count reads only the bounds.

## Magnetic Lasso field

`EdgeMap` keeps a clone of the picture (a refcount), the peak, the threshold
and a `OnceLock` per 128-pixel tile. The press finds the peak in parallel
64-row bands that roll three luma rows and keep nothing. A trace computes a
tile's luma with a one-pixel apron and its Sobel costs on first read, in the
eager field's operation order, so every cost has the eager bits.

## Shape content box

`layer_effects::fill_content_box(layer)` is a fill or shape layer's rect
narrowed to the box its mask confines it to (`confined_fill_box`, the same rule
as the layer's damage reach, without the effects pad). It reads the fill kind
from the block key rather than decoding it, so a composite still decodes each
fill once. Outside the box the mask is zero, and a zero mask leaves the
backdrop untouched, so:

- `composite_solid_fill` blends only the box, unless the pass ignores the mask;
- every effect's source rect (`clip_rect`) is the box;
- `fill_source` uploads only the box, built with `Plane::build` and one fill per
  plane, and keys the cache on it;
- the GPU coverage's influence rect is the box.

A test-only switch restores the whole rect, and the composite with and without
a drop shadow is compared byte for byte.

## Shape Move and path drag

`rendered_layer_image` renders a shape over its paint bounds. The backdrop
patch accepts a shape layer and covers its `content_influence` (outline box,
effects, clipped run). `translate_move_group` dirties a shape's influence
before and after instead of its `rect`. `edit_target` computes the union of
the target's influence before and after an edit. A live drag step (subpath,
anchor or handle) adds it to `path_drag_damage` and composites nothing, while
the overlay draws the moving outline. Every path commit and the gesture's end
(`path_gesture_end`, on every release) refresh the pending region with
`refresh_regions` and `changed`, or recomposite when the reach is unprovable.
Any other edit flushes it at once.

## Fill Pixels

`shape::coverage_box` rasterizes into the box of the contours' rows and columns,
in parallel rows. `coverage` is that box expanded, so it is unchanged.
`bucket::fill_within` hands `fill_layer_within` the box. For an 8-bit layer it
fractures each plane, forks only the bands the box crosses with `write_bands`,
and runs `fill_planes` per band, clipped to the box. A mask target keeps the
document-space path.

## What remains

On this machine the CPU region compositor runs at about 10 ns per pixel, so a
shape over half the canvas still costs about 0.7 s to composite, and its undo
about 0.4 s. That is the general compositor (#1093), not shape code. The
stroked shape's Move press also pays the stroke effect over the shape's box.
