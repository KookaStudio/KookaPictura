# Design

## Context

The canvas draws two different outlines for the same selection, and only one of
them is clipped to the canvas.

- The **committed** outline is traced from the selection mask
  (`pictura_select::contour` over the mask in `pictura-select/src/contour.rs`).
  The mask is document-sized and every shape that reaches it is already
  canvas-clipped: `Selection::rect` and `Selection::ellipse` clamp their box to
  the canvas (`pictura-select/src/lib.rs:108`), `Selection::polygon` samples
  only the canvas, and `Selection::translate` drops what leaves it. The traced
  outline therefore turns the corner along the canvas border on its own.
- The **live** outline is the analytic shape the tool is dragging
  (`tool_selection.cpp:161`/`:163` for the marquee, `tool_selection.cpp:204`/
  `:221` for the freehand lasso, `tool_magneticlasso.cpp:257` for the magnetic
  lasso, `tool_region_drag.cpp:35`/`:66` for the content-aware-move and patch
  region trace). It is unclipped, and the paint crops it with a hard rectangle
  clip (`image_view.cpp:738`, `painter.setClipRect(docClip)`), so the band stops
  at the border instead of closing along it. The polygonal lasso's band
  (`tool_selection.cpp:329`/`:340`) is the one live preview that is open rather
  than closed, and it is not in scope (see D6).

The paint runs in image space, and the document rectangle is
`(0, 0, docSize_.width(), docSize_.height())`.

The preview channel is shared with overlays that are not selection shapes: the
zoom marquee (`tool_hand_zoom.cpp:63`), the red-eye ring (`tool_redeye.cpp:54`),
and the region drag's moved outline (`tool_region_drag.cpp:57`). The
move-selection drag's offset outline is drawn from the committed contour
(`tools_selection_move.cpp:103` -> `setSelectionContourOffset`), which the
capability already specifies as "the committed outline translated by the drag
offset".

The Qt behaviour below was measured on the repo's Qt 6.11.1 with a scratch
program, not assumed.

See proposal.md for the motivation and
`specs/tools/selection-outline-performance/spec.md` for the behaviour this
design delivers.

## Goals / Non-Goals

**Goals:**

- The live outline is the dragged shape intersected with the canvas rectangle,
  so the part of the shape inside the canvas is drawn as a closed band.
- One implementation of the clip, shared by every selection-shape preview.
- The clip is computed when the preview is set, not on every painted frame.
- The clip is exact for the marquee shapes, where the live outline and the
  committed outline are the same region.

**Non-Goals:**

- The move-selection drag's offset outline. It is a committed outline being
  dragged, not a shape being defined, and its behaviour is specified: the
  capability requires the committed outline translated by the drag offset.
  Closing it along the border would change that requirement, so it is not part
  of this change.
- The region drag's moved outline, for the same reason.
- The polygonal lasso's band. It is an open rubber band
  (`tool_selection.cpp:329`, `setSelectionPreview({polygonPoints_}, false,
  /*solid=*/true)`) and an open band has no closing edge to add: the border
  run the clip adds would close a gesture the user has not closed. The paint
  already crops the band at the border, so its appearance does not change
  either way; what the change pins is that the band gains no closing edge.
- Making the live ellipse preview pixel-identical to the committed outline. The
  committed one is a pixel-quantised staircase; the live one stays analytic.
- Any change to how the committed outline is traced or drawn.

## Decisions

### D1. Clip when the preview is set, not when it is painted

`ImageView::setSelectionPreview` already runs on every pointer move, while the
paint runs on every frame including the 120 ms dash-phase timer. Clipping in
the setter keeps the per-frame cost at zero and leaves the stored geometry as
the clipped shape, so a test can assert the outline without rendering.

*Alternative:* clip in `paintEvent`. Rejected: it would run the clip per frame,
and the paint cannot tell a selection shape from the zoom marquee.

### D2. The clip is a pure helper in `selection_geometry`

`crates/pictura-app/cpp/selection_geometry.{h,cpp}` is the app's module for
pure selection geometry (`dragRect`, `marqueeDragRect`): it has no Rust
dependency, no window, and no document, which is what the clip needs. The
helper takes polygons and a canvas size and returns polygons.

*Alternatives:*

- `api/selection_api.{h,cpp}`. Rejected: that module is the typed adapter over
  the Rust selection (`SelectionBounds`, `SelectionContour` read the engine);
  a view-side geometry helper does not belong to the bridge adapter.
- A new `selection_outline.{h,cpp}` pair. Rejected: one function does not
  justify a new CMake entry and a new file.

### D3. The clip is a path intersection against the canvas rectangle

`QPainterPath::intersected` with the shape path and the canvas-rect path
produces the border-closing segments directly. Measured on Qt 6.11.1, a
rectangle from (50,50) to (100,100) against a 64x64 canvas comes back as
`(64,50) (64,64) (50,64) (50,50)`: the run along `x = 64`, the corner at
(64,64), and the run along `y = 64` are all present, which is exactly the
closure the committed contour has.

*Alternatives:*

- A hand-rolled Sutherland-Hodgman clip. Rejected: it degenerates on a concave
  subject (doubled border runs) and needs a second implementation for open
  bands.
- Rasterising the shape into a document-sized mask and tracing it. Rejected:
  that is the engine's own path, and it costs a document-sized buffer per
  frame, which is exactly what `tools/selection-outline-performance` exists to
  avoid.

### D4. The clip is on the region the shape encloses

`QPainterPath::intersected` intersects the *filled* regions, not the outlines.
Measured on Qt 6.11.1, a shape that overlaps itself and crosses the border
comes back as the boundary of the region it encloses: a bowtie
`(10,10) (90,50) (90,10) (10,90)` against a 64x64 canvas comes back as six
vertices with runs along `x = 64` and `y = 64`, and a self-crossing lasso
`(20,20) (90,20) (20,50) (90,50)` comes back as seven vertices with fractional
coordinates.

That is the same region the engine rasterises (`Selection::polygon` is an
even-odd scanline fill) and the same region the committed contour traces, so
for a shape that overlaps itself the live band and the committed outline agree
more closely than the raw trace would. The requirement says so, and the case is
covered by a scenario.

The cost is that the band is no longer the stroke the user laid down, which is
what CS6 shows while a lasso is being dragged: CS6's rubber band is the trace
and its ants are the region, and they differ for a self-overlapping lasso.
This change keeps one geometry for both, in the direction the issue asks for
(the live outline shows the shape the release will have).

### D5. Degenerate loops pass through unchanged

Measured: a one-point polygon comes back from `intersected` as an empty path,
and a two-point polygon survives as a bare segment. The one-point case is
reachable: the freehand lasso's and the magnetic lasso's first preview is a
single point (`tool_selection.cpp:201-205`, `tool_magneticlasso.cpp:141-146`).
A loop with fewer than three points is therefore returned unchanged.

The fast path (a shape wholly inside the canvas is returned unchanged) is
written out as an explicit comparison, not `QRectF::contains`: measured,
`contains` is false for a null rectangle, so a naive containment test would
send a degenerate shape through the boolean op and lose it. The outside test
is written out for the same reason: `QRectF::intersects` is false for a flat
bounding box, so a horizontal stroke would be dropped as if it lay outside the
canvas.

Two shapes have no clipped geometry to show, and both keep their trace:

- A shape whose part inside the canvas encloses nothing, which a stroke that
  doubles back on itself does: the intersection is empty, and the trace is kept
  so the band does not vanish.
- A shape whose region inside the canvas is empty for the same reason.

### D6. Open bands are not clipped

The polygonal lasso's band is the one open preview
(`tool_selection.cpp:329`, `setSelectionPreview({polygonPoints_}, false,
/*solid=*/true)`). Clipping it would either close it along the border, which
misrepresents a gesture the user has not closed, or need a second, per-segment
clip. The band keeps the painter's crop, which already stops it at the border,
so nothing about it changes visually; the requirement pins that it gains no
closing edge.

### D7. The canvas clips; the tools say what a preview is

The canvas owns the document rectangle, so it is the only place that can clip
without every caller repeating the rect lookup. A dedicated setter,
`ImageView::setSelectionShapePreview(loops)`, marks the previews that stand for
a shape the user is defining. It stores a closed, dashed band, and it calls
`updateAntsTimer()` so the live band keeps animating its dash phase; without
that call the live ants would be static. The existing `setSelectionPreview`
keeps its meaning for the open polygonal band, the zoom marquee, the red-eye
ring, and the moved outline.

*Alternatives:*

- A `clipToCanvas` flag on `setSelectionPreview`. Rejected: it pushes a fourth
  positional argument, and three of them are booleans, onto every call site.
- Clipping in each tool. Rejected: six call sites would each repeat the rect
  lookup and the call.
- Clipping every preview. Rejected: the zoom marquee and the red-eye ring are
  not selection shapes and must stay unclipped.

### D8. The clip rectangle is the document rectangle

The paint clips to `docClip = canvasDoc` (`image_view.cpp:599`), which is the
document rectangle except while a crop box is live, when it is
`cropCanvasImageRect()` (`image_view.cpp:569-571`). The two coincide in every
reachable selection drag, because the crop handler clears the box when the
crop tool deactivates (`tool_crop.cpp`). The clip uses the document rectangle;
if a future change keeps a crop box alive across a tool switch, the two would
disagree and the band would be cut at the wrong rectangle. That is the seam to
revisit then, not now.

### D9. The committed-ants marker is re-worded, not deleted

The `ponytail:` marker at `image_view.cpp:750` sits in the committed-ants
block, whose only outline that can cross the canvas edge is the move-selection
drag's offset outline. It stays a marker (the cut is a deliberate ceiling) and
is re-worded to name that case and the requirement that specifies it, so it no
longer reads as a note about the marquee.

## Risks / Trade-offs

- [A lasso that overlaps itself is drawn as the boundary of the region it
  encloses, not as the stroke CS6 shows while dragging] -> the region is what
  the release selects and what the committed ants trace, so the live band and
  the committed outline agree; the requirement says so and the case has a
  scenario. A stroke that encloses nothing keeps its trace, so no band
  disappears.
- [The live ellipse preview stays analytic while the committed outline is a
  pixel-quantised staircase] -> the two differ by at most one pixel along the
  curve; the border segments they share are exact. Accepted: the ellipse
  preview was analytic before this change too.
- [A shape leaving and re-entering the canvas draws its border run twice] ->
  it reads as one dashed line.
- [The clipped loop starts at an arbitrary vertex, so the dash pattern
  re-anchors when a shape first crosses the border] -> measured: the clipped
  rectangle starts at (64,50) where the shape started at (50,50), and the
  result's vertex list may or may not repeat its first point (the rectangle
  does not, a concave shape does). The shift is under one dash length, once,
  when the drag crosses the border. Accepted; a test compares edge sets, not
  vertex lists.
- [The clip runs a path intersection on every pointer move, and the freehand
  lasso's trace grows with the drag] -> measured on a 64x64 canvas: 3.6 us at
  four vertices, 17 us at 64, 163 us at 1024 and 2.9 ms at 16384. The fast
  path skips the intersection for a shape wholly inside the canvas, which is
  the common case, and a long lasso drag is the ceiling. ponytail: a lasso
  trace over a few thousand points drops a frame; decimate the trace before
  clipping if that shows up.
- [`tsc_ellipse_preview` asserts more than six preview points] -> a shape
  straddling the border legitimately clips to six points (measured). The
  self-test's ellipse is 11x11 in a 16x16 document, so it stays inside and the
  fast path leaves it alone; the case is re-verified by the change.
- [`image_view.h` is at 799 lines, two under the 800-line target] -> the new
  setter and accessor fit; the next addition has to move the accessor beside
  the other `ForTest` accessors to stay under the target, and the 1200-line
  hard cap is far away.
