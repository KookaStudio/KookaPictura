# Design: port-photorust-shear

## Model

Ported from perfecto25/photorust `core/src/filters/distort.rs::shear`, with
the curve generalized from its fixed 17-point lattice to Kooka's control
points so the filter stays a `Filter::Shear { curve, fill }` value.

- **Geometry.** Destination pixel `(x, y)` samples `(x − offset·w/2, y)`.
  `offset` is a cubic Hermite through `curve` with Catmull-Rom tangents,
  read at the row's normalized position: the top row's pixel centre is `-1`,
  the bottom row's `+1`. A 1-pixel-tall image reads the curve at `0`. The
  tangents are the finite differences of the neighbouring points, so the
  slope is continuous at every point and a two-point curve is exactly the
  straight line between them.
- **Sign.** A positive `offset` samples to the left, so the row's content
  moves right — the same direction as dragging the curve box's line right,
  and as ImageMagick's `-shear {angle}x0` (a positive horizontal shear).
- **Edges.** `fill` maps to the shared `EdgeMode`: `WrapAround` → `Wrap`,
  `RepeatEdgePixels` → `Clamp`.
- **No-op.** An all-zero `offset` curve returns before touching the buffer,
  so it is bit-exact even on translucent pixels (the shared `remap`
  premultiplies, which rounds a translucent no-op).
- **Alpha.** The shared `remap`/`write_colour` path never writes the alpha
  plane.

## Kooka adaptations

- **Validation.** `dispatch.rs` checks the curve before any mutation:
  at least two points, all finite, `position` and `offset` in `-1..=1`, and
  `position` strictly increasing. Failures are `FilterError::InvalidParams`
  with the buffer untouched.
- **Slots.** `SHEAR_MAX_POINTS = 8` lives in `pictura-filters`. The app's 18
  shear slots are a point count (`2..=8`), then eight `(position, offset)`
  pairs with the unused ones zero, then the fill index. The editor's points
  and the filter's points are the same points, so a drag cannot drift from
  what the engine applies and a reopen restores exactly what was committed.
  The cap keeps the filter a value that can be copied, compared, and
  replayed — photorust's rationale — while an 8-point curve is far finer
  than CS6's box can usefully distinguish.
- **Preview.** `shear` joins `filter_preview_needs_whole_layer`: the
  displacement is a fraction of the layer's half-width and the row mapping
  spans the layer height, so a cropped section preview would be wrong (the
  pre-port column model had the same flaw).

## Dialog

The curve editor is a `FilterParamControls` control (`ShearCurve`, a count
plus eight pairs = 17 slots), not a dedicated dialog, so the Filter Gallery's
options pane gets the same editor. `ShearCurveWidget` is photorust's
`ShearCurveWidget`: a 140 px white box, a dotted 4×4 guide, a smooth line
through square handles, click to insert, drag out to remove, the two ends
pinned to the top and bottom edges and sliding horizontally only, and the
same Hermite the engine runs. `Radio` is a new one-slot control for CS6's
`Undefined Areas` pair (the generic `Choice` is a combo box).

`FilterCommandSpec` gains `previewBelow`: the generic `FilterPreviewDialog`
then puts the thumbnail under the body instead of beside it, drops the zoom
row, and drops the Preview checkbox (CS6's Shear has neither; the preview is
live). The bottom pane composites the active layer — filtered at proxy scale
over the layer's own bounds, mask and opacity baked in — over the document
with that layer hidden, so a drag follows live and the other layers are never
displaced with it; the whole-layer canvas render waits for the mouse release.
