# Design

## Context

See `proposal.md` — Why — for the motivation.

The Eraser's Block mode is a stroke of a square tip: `pictura-paint` sets
`StrokeConfig::square` and a diameter, and the rasterizer stamps that tip
(`crates/pictura-paint/src/stroke/raster.rs`). The tip's coverage test lives in
`TipParams::coverage` (`crates/pictura-paint/src/tip.rs`), which samples a
pixel at its centre (`lx + 0.5`).

The paint crate is Qt-free, so it cannot ask for the canvas zoom; it takes it
as a number. The zoom itself lives in the C++ `ImageView` (`zoom_`, screen
pixels per document pixel, clamped to [0.01, 32]) and reaches the tool layer as
`ctx.canvas()->zoom()`.

The brush-size ring is drawn by `ImageView` as an ellipse in image space under
the painter's `scale(zoom_, zoom_)`, so it scales with the canvas for free. It
is re-applied from `ToolController::updateBrushOutline` on every pointer move,
and cleared when the tool changes, the canvas unbinds, or the pointer leaves.

## Goals / Non-Goals

**Goals:**

- The Block covers the same number of screen pixels at every zoom, as in CS6.
- A Block dab covers exactly its diameter in document pixels, whatever the
  dab's position in its pixel.
- The Block's cursor shows the size the Block actually erases, and follows the
  zoom.

**Non-Goals:**

- The Brush and Pencil modes, whose size is in document pixels and whose ring
  scales with the zoom.
- The Background Eraser, the Magic Eraser, and the Pencil's Auto Erase.
- Block on a layer mask: the mask path paints the mask's coverage with the
  round brush tip and never reaches the Block arm. That shortcut stays, and
  the Block's cursor follows it — the controller asks the view whether a mask
  edit target is active, and on a mask it keeps the brush-size circle, because
  that is what the erase will do. Making the mask path honour the Block is one
  small step further and would match CS6, but it changes a path this change
  does not touch.
- Any change to the number of history states a stroke records.

## Decisions

### The rule and where it lives

CS6's Block is a fixed 16 *screen*-pixel square, so its document side is
`16 / zoom`:

    Block side (document px) = clamp(round(16 / zoom), 1, …)

The engine owns the constant and the formula, because the engine owns the
Block mode. `pictura-paint` gains `BLOCK_SCREEN_PX` and
`block_side_doc_px(zoom)`, and `begin_erase` takes the zoom; its
`EraserMode::Block` arm sets `diameter: block_side_doc_px(zoom)`. The zoom
travels `ctx.canvas()->zoom()` → `begin_eraser` (the bridge) → `begin_erase`.

*Alternative:* the tool layer computes `16 / zoom` and passes the diameter.
Rejected: the rule would live in two layers, and the engine would need a flag
to know the diameter it was handed is the block's rather than the brush's.

*Alternative:* a zoom field on `StrokeConfig`. Rejected: the Brush's size is
document space, so a zoom field would be meaningless for every other tool.

The engine guards the conversion: a non-finite or non-positive zoom is treated
as 1.0, and the result is clamped to the `StrokeConfig` diameter range, so a
bad zoom cannot produce a 5000-pixel block.

The stroke's tip is fixed when the stroke begins, so a zoom during a drag does
not resize the block mid-stroke, exactly as the Brush's diameter stays fixed.

### The square's boundary

The square tip's coverage test is inclusive on both axes:

    dx.abs() <= half && dy.abs() <= half

With a dab at a whole-pixel position that covers exactly `diameter` pixels, and
at a half-pixel position it covers `diameter + 1`. The block's width therefore
depended on where the mouse fell: a drag across the canvas left a band that
alternated between 16 and 17 pixels wide, and a one-document-pixel block — the
3200 % case — covered two pixels instead of one.

The boundary becomes half-open:

    dx >= -half && dx < half && dy >= -half && dy < half

The covered pixels are then exactly `diameter` a side at any dab position. The
covered area is within half a document pixel of the dab rather than exactly on
it, which is the same quantisation every pixel rasterisation has.

*Alternative:* keep the inclusive test. Rejected: it keeps the ink exactly
centred but makes the block's width depend on the mouse's sub-pixel position,
and CS6's block is a fixed 16 screen pixels.

*Alternative:* the "fully inside" form (`|dx| <= radius - 0.5`). Rejected: it
covers `diameter - 1` or `diameter` pixels, so the count is not exact either.

`square: true` has one production user, the Block eraser, and three test sites:
`tip.rs`'s `square_tip_fills_its_corners` and the two lazy-versus-whole
parity configs in `stroke/tests.rs`. All three still hold under the half-open
form: the tip test samples 4.5 and 5.5 at a half of 5, and the parity tests
compare two code paths against each other.

### The shape dynamics do not apply to the Block

The Block's arm already overrides the brush preset's grain and Wet Edges,
which would leave the square partly opaque. It also overrides Scattering, the
dab count, and Size Jitter, which would scatter the square, stamp it several
times, or shrink it: the Block is one square of one size, so a dab cannot
cover a different number of pixels than the spec states. Angle and Roundness
Jitter are left alone — a square tip ignores both.

### The cursor carries the erase's size

The Block's cursor must show the size the Block actually erases, or it lies: a
fixed 16 screen pixels drifts from an erase of `round(16 / zoom)` document
pixels — 15.9 screen pixels at a 30 % zoom, 32 at a 3200 % zoom. So the cursor
carries the block's *document* size, and the controller recomputes it whenever
the zoom changes.

`ImageView` keeps one outline — a diameter, a position and a shape — so the
circle and the square can never both be drawn, and `clearBrushOutline` (tool
switch, canvas unbind, pointer leave) still clears both:

    setBrushOutline(diameter, imagePos)   // image px, a circle — the Brush
    setBlockOutline(side, imagePos)       // image px, a square — the Block

The block's halo is one *screen* pixel (`1.0 / zoom_` image pixels), unlike the
brush ring's one image pixel, so the white edge stays tight around the black
one at any zoom.

`ToolController::updateBrushOutline` picks the shape and the size: the Eraser
in Block mode gets the square of `block_side_doc_px(canvas_->zoom())` — the
engine's own function, exposed through the bridge, so the cursor and the erase
cannot drift — and every other brush tool keeps the circle.

Three things re-apply the outline, through one `refreshBrushOutline()` helper
that reads the live pointer position and does nothing when the pointer is
outside the canvas:

- the Eraser's Mode changes (the Eraser option group has no `changed` handler
  today; the change assigns one, and it only re-applies an outline, and only
  while a brush tool is active, so the self-test's option restore cannot
  disturb the canvas);
- the canvas zooms (`ImageView::zoomChanged` fires on all four zoom paths);
- the brush size changes (the existing handler, which today would replace the
  square with a circle).

*Alternative:* store the block's size in screen pixels in the view and convert
at paint time. Rejected: the cursor would then drift from the erase, and the
view would duplicate the engine's rounding rule.

## Risks / Trade-offs

- [The block's ink is within half a document pixel of the pointer] → the
  half-open boundary trades a sub-pixel offset for an exact width. Above 1600 %
  zoom the block is one document pixel, so the cursor can straddle the erased
  pixel's edge. Recorded as a `ponytail:` ceiling at the tip.
- [The cursor is a document-space square, so it grows with the zoom like the
  brush ring] → that is the point: it tracks the erase. Its *nominal* size
  stays 16 screen pixels, because the erase's document side is `16 / zoom`.
- [The half-open boundary changes a shared tip] → `square: true` has one
  production user, and its three test sites are re-checked.
- [A zoom change now reaches into the tool layer] → the controller only
  re-applies an outline that is already showing, and does nothing when the
  pointer is outside the canvas.
- [The bridge signature changes] → the only callers are `tool_eraser.cpp` and
  two call sites in `tst_mask_edit.cpp`; the self-test layer drives the UI and
  never calls the bridge.
