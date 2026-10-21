# free-transform Specification

## Purpose
The Free Transform session: layer transforms with bilinear resampling, bounding-box hit-testing, modifiers, and undo.

## Requirements

### Requirement: Layer similarity transform entry point and refusal

The system SHALL provide
`pictura_render::transform_layer(doc: &mut Document, path: &str, transform:
LayerTransform) -> bool`, where `LayerTransform` carries `scale_x: f64`,
`scale_y: f64`, `angle_radians: f64`, `dx: f64`, and `dy: f64`. On success it
SHALL resample every channel plane of the layer at `path` into the transformed
bounding rect, update that layer's `rect`, and return `true`. The operation SHALL
refuse and MUST leave `doc` bit-identical when the path does not resolve, the
target is a group, an adjustment layer, or a Background layer, the target is
position-locked, the source rect has zero area, any transform parameter is
non-finite, either scale factor is zero or below a small epsilon, or the
computed result rect has zero area. The operation SHALL NOT recomposite the
document; the caller owns the composite refresh. It MUST NOT panic on any input,
including a 1×1 source.

#### Scenario: A scale and rotation updates the rect and channels

- **WHEN** `transform_layer` is called on a raster layer with a 90° rotation and a 2× uniform scale
- **THEN** it returns true, the layer `rect` is the transformed bounding box, and every channel plane is sized to that rect

#### Scenario: Group, adjustment, Background, and locked targets are refused

- **WHEN** `transform_layer` is called on a group, an adjustment layer, a Background layer, or a position-locked layer
- **THEN** it returns false and every field of `doc` equals its pre-call value

#### Scenario: Degenerate transforms are refused

- **WHEN** `transform_layer` is called with a zero scale factor, a non-finite value, or a source rect of zero area
- **THEN** it returns false and the document is unchanged

#### Scenario: The op does not recomposite

- **WHEN** `transform_layer` succeeds
- **THEN** the layer channels and rect change but `doc.composite` is left for the caller to rebuild

### Requirement: Bilinear resampling and coverage rules

The transform SHALL map a layer-local point `p` by
`p' = c + R(θ)·(S·(p − c)) + (dx, dy)`, where `c` is the source rect centre, `S`
is `diag(scale_x, scale_y)`, and `R(θ)` is a clockwise-positive screen-space
rotation matching `pictura_ops::rotate_arbitrary`. The result `rect` SHALL be the
integer bounding box of the four transformed corners: `left = floor(min x')`,
`top = floor(min y')`, `right = ceil(max x')`, `bottom = ceil(max y')`. Each
destination pixel SHALL be sampled with bilinear interpolation from the inverse
map `src = c + S⁻¹·R(−θ)·(q − c) − (dx, dy)`; every channel plane is resampled
independently. A destination whose source point lies outside
`[0, w) × [0, h)` SHALL be written as `0` for every channel (transparent for the
alpha plane, black for colour), and bilinear taps SHALL be edge-clamped. When the
layer carries a `LayerMask`, the mask SHALL be transformed by the same
document-space matrix about its own rect, its `rect` updated to the transformed
bounding box, and out-of-source mask samples written as `0`.

#### Scenario: A scale doubles the plane dimensions and rect

- **WHEN** a `w × h` layer is transformed by `scale_x = scale_y = 2.0` with no rotation or offset
- **THEN** the layer rect and every channel plane are `2w × 2h` and the result is a bilinear resample of the source

#### Scenario: Pixels outside the source are transparent

- **WHEN** a layer is rotated by 45° so the destination bounding box has empty corners
- **THEN** every destination pixel whose source point is outside the source rect has all channels equal to 0

#### Scenario: The mask follows the layer

- **WHEN** a masked layer is transformed
- **THEN** the mask rect is the transformed bounding box, the mask plane is resampled, and out-of-source mask samples are 0

### Requirement: Channel-less embedded smart-object materialization

`transform_layer` SHALL materialize the raster proxy of a channel-less embedded
smart object from its payload before resampling, when `path` resolves to such a
layer and its payload renders through the existing embedded source path. On
success it SHALL drop the smart object, its preserved `SoLd`/`plLd` blocks, and
its linked-source record so no stale untransformed source can survive the
transform. A channel-less target that is not a decodable embedded object SHALL
be refused with no mutation.

#### Scenario: A channel-less placed object is transformed

- **WHEN** `transform_layer` runs on a channel-less embedded smart-object layer with a decodable payload
- **THEN** the layer gains resampled channels, its rect is transformed, and `layer.smart_object` is `None`

#### Scenario: An undecodable channel-less target is refused

- **WHEN** the target has no channels and its embedded payload cannot be rendered
- **THEN** the operation returns false and the document is unchanged

### Requirement: Free Transform session lifecycle and undo contract

The system SHALL provide a modal, per-`PictureView` Free Transform session. It
SHALL begin on an explicit target path, hold the target path, the original rect,
and the current `scale_x`, `scale_y`, `angle_radians`, and `(dx, dy)`, and SHALL
expose press, move, and release callbacks that take document-space image
coordinates and modifier flags. Enter/Return SHALL commit: the session SHALL call
`transform_layer` once, recomposite, and record exactly one history state
labelled `"Free Transform"`. Escape SHALL cancel: the session SHALL end with the
document bit-identical to its state before the session began and with no history
state added. Beginning a session on the path already active SHALL be a no-op;
beginning one on a different path SHALL cancel the active session and start a
new session on the new path. Switching tools, switching documents, or closing
the document SHALL cancel the active session. Committing an unchanged (identity)
transform SHALL record no history state.

#### Scenario: Begin, gesture, commit records one state

- **WHEN** a session is begun on a raster layer, a scale gesture is applied, and Enter is pressed
- **THEN** exactly one history state labelled `"Free Transform"` is added and the layer rect reflects the gesture

#### Scenario: Escape restores the document exactly

- **WHEN** a session is begun, a rotation gesture is applied, and Escape is pressed
- **THEN** the document equals its pre-session value and the history count is unchanged

#### Scenario: An identity commit records nothing

- **WHEN** a session is begun and Enter is pressed without any gesture
- **THEN** no history state is added and the document is unchanged

#### Scenario: Beginning on the active path is a no-op

- **WHEN** a session is active on a path and begin is called again on that same path
- **THEN** the active session is unchanged

#### Scenario: Beginning on a different path restarts the session

- **WHEN** a session is active on one path and begin is called on a different transformable path
- **THEN** the first session is cancelled without mutating the document and a new session on the second path begins

#### Scenario: Switching documents cancels the session

- **WHEN** a session is active and the active document changes
- **THEN** the session ends without mutating the original document

### Requirement: Transform bounding-box geometry and hit-testing

The system SHALL compute the session's bounding quad as the four corners of the
source rect under the current transform and SHALL draw the quad with eight scale
handles (four corners and four edge midpoints) and a rotate affordance. Hit
testing SHALL be in screen space with a tolerance of 6 screen pixels for handles;
a press inside the quad but away from a handle SHALL move, and a press within a
20 screen-pixel band outside the quad's perimeter — any side or corner — SHALL
rotate about the reference point. A press beyond the band SHALL be inert: it MUST
NOT move, rotate, commit, or cancel the session. The canvas SHALL show a
diagonal, horizontal, or vertical size cursor over a handle, an open or closed
hand while moving, and a rotate cursor oriented to the grabbed side or corner
throughout the rotate band.

#### Scenario: Dragging a corner handle scales

- **WHEN** the user presses on a corner handle and drags away from the layer centre
- **THEN** the layer scales about the opposite corner and the bounding quad follows the pointer

#### Scenario: Dragging inside the quad moves

- **WHEN** the user presses inside the quad away from every handle and drags
- **THEN** the layer translates by the pointer delta and does not scale or rotate

#### Scenario: Dragging the rotate band rotates

- **WHEN** the user presses in the rotate band outside a corner and drags
- **THEN** the layer rotates about the reference point and the quad rotates with the pointer

#### Scenario: Rotating from an edge midpoint [ft_edge_rotate]

- **WHEN** the user presses in the rotate band just outside an edge midpoint and drags
- **THEN** the layer rotates about the reference point and the quad rotates with the pointer

#### Scenario: A press beyond the band is inert [ft_far_press_inert]

- **WHEN** the user presses and releases beyond the rotate band while a session is active
- **THEN** the session stays active, the document is unchanged, and no history state is added

#### Scenario: Projective modes keep handle-only hit testing [ft_projective_band]

- **WHEN** a Distort, Perspective, or Skew session is pressed outside the quad near an edge
- **THEN** nothing is hit and the quad does not change

### Requirement: Scale and rotate modifiers and minimum size

Holding Shift while dragging a corner handle SHALL lock the aspect ratio by
applying one uniform scale factor to both axes; edge handles SHALL always scale
one axis. Holding Shift while rotating SHALL snap the angle to 15° steps. The
session SHALL clamp the transform so the transformed rect keeps at least 1 pixel
in each axis and SHALL refuse a gesture that would produce a zero-area result.
Negative scale factors SHALL be permitted and flip the content. The reference
point SHALL be the session's movable pivot, the source rect centre by default
(see `document/transform-reference-point`).

#### Scenario: Shift on a corner keeps the aspect ratio

- **WHEN** the user drags a corner handle with Shift held
- **THEN** both scale factors are equal and the layer aspect ratio is preserved

#### Scenario: Shift rotation snaps to 15° steps

- **WHEN** the user rotates with Shift held
- **THEN** the committed rotation angle is an exact multiple of 15°

#### Scenario: A gesture cannot collapse the layer

- **WHEN** the user drags a handle so the transformed rect would have zero width or height
- **THEN** the session clamps to at least 1 pixel per axis and does not commit a zero-area rect

### Requirement: Live preview reuses the move-preview overlay

The session SHALL preview by drawing a cached base composite with the target
layer hidden plus a cached image of the target layer under the current
`QTransform`, reusing the Move tool's cached base/layer overlay. The session MUST
NOT rebuild the document composite on each mouse move. For a channel-less
embedded target the preview layer image SHALL be rendered from its embedded
source without mutating the document. When the target's raw channels do not
carry its whole appearance — a layer mask, vector mask, layer style, fill
adjustment, clipping group, or editable type — the preview layer image SHALL instead be rendered
through the compositor with the layer's opacity and blend neutralized, cropped
to the layer's rect, so the live preview keeps that appearance. The preview
SHALL apply the layer opacity once.

#### Scenario: A drag does not recomposite

- **WHEN** a session gesture updates the transform
- **THEN** the cached base and layer images are redrawn with the new transform and no document composite is rebuilt

#### Scenario: Previewing a channel-less target mutates nothing

- **WHEN** a session is begun on a channel-less embedded smart object
- **THEN** the document equals its pre-session value until commit

#### Scenario: A styled layer keeps its color [ft_preview_appearance]

- **WHEN** a transform session begins on a layer whose visible color comes from a layer style, layer mask, vector mask, fill adjustment, clipping group, or type
- **THEN** the live preview shows that appearance rather than the bare channels

### Requirement: Place starts a Free Transform session

After a successful Place the application SHALL select the newly placed layer and
start a Free Transform session on it, whether the place came through the
`File > Place…` command or a canvas file drop. For a multi-file canvas drop the
session SHALL target the last successfully placed layer and the other placed
layers SHALL remain where they landed. Cancelling the session SHALL leave every
placed layer where it landed, untransformed, and SHALL NOT remove or roll back
the `"Place"` history state recorded by the placement.

#### Scenario: Menu Place enters the session

- **WHEN** `File > Place…` completes successfully with a supported image
- **THEN** a Free Transform session begins on the new layer and the `"Place"` state remains

#### Scenario: Canvas drop enters the session

- **WHEN** one or more supported files are dropped on the document canvas
- **THEN** a session begins on the last placed layer and every placed layer keeps its `"Place"` state

#### Scenario: Cancelling keeps the placement

- **WHEN** a session started by a Place is cancelled
- **THEN** the placed layer remains at its landed rect and the `"Place"` history state is still present

### Requirement: Free Transform resolves the active layer

The application SHALL begin a Free Transform session on the active layer, where
the active layer is the exactly-one layer resolved by the shared active-layer
resolver, rather than defaulting to the topmost raster layer. When no layer is
active or more than one layer is selected, beginning a session SHALL be refused
and the document SHALL be unchanged. The refusal SHALL be surfaced to the user.
An explicit target path supplied by a Place still takes precedence over the
resolver.

#### Scenario: Free Transform targets the active layer [lft_active_layer]

- **WHEN** Free Transform is begun with exactly one layer active
- **THEN** the session targets that layer and its transform updates that layer

#### Scenario: No single active layer refuses [lft_no_active_layer]

- **WHEN** Free Transform is begun with no layer active or with more than one
  layer selected
- **THEN** no session begins, the document is unchanged, and the refusal is
  reported

#### Scenario: A Place target overrides the resolver [lft_place_target]

- **WHEN** a Place starts a Free Transform session on its newly placed layer
- **THEN** the session targets the placed layer even if the resolver would name a
  different layer

### Requirement: Projective (quad) layer transform op

The system SHALL provide
`pictura_render::transform_layer_quad(doc: &mut Document, path: &str, quad:
[(f64, f64); 4]) -> bool`, where `quad` gives the document-space target of the
source layer rect's four corners in top-left, top-right, bottom-right,
bottom-left order. On success the operation SHALL resample every channel plane
(and the layer mask, about its own rect) with the bilinear inverse of the
projective map sending the source corners to `quad`, SHALL set the layer `rect`
to the integer bounding box of `quad` (`floor(min x)`, `floor(min y)`,
`ceil(max x)`, `ceil(max y)`), and SHALL return `true`. A destination pixel whose
inverse-mapped source point lies outside `[0, w) × [0, h)` SHALL be written as
`0`, and bilinear taps SHALL be edge-clamped. A destination pixel whose
inverse-mapped source point lies on the far side of the projective horizon SHALL
also be written as `0`. The front side SHALL be the side holding the majority of
the source rect's four corners' forward homogeneous divisors, or the centre's
side on a tie. The live projective preview SHALL clip to the same front side.
When `quad` crosses itself (its side edges, or its top and bottom edges,
intersect), the operation SHALL instead fill the quad's own outline: the two
straight-edged triangles that meet where its edges cross, leaving every pixel
outside them at `0`. The source SHALL split at the row (or, for crossing top
and bottom edges, the column) whose fraction is the mean of the crossing
point's fractions along the two crossing edges; the part before it fills the
first triangle and the rest the second, each source row parallel to its
triangle's base and shrinking to the crossing point. This matches Photoshop's
crossed Perspective box (inferred from a CS6 recording of a flat-colour layer;
the pixel layout inside the triangles is not verified).
The operation SHALL refuse and MUST leave `doc` bit-identical for every case
`transform_layer` refuses, and also when any corner is non-finite, the
projective map is singular or near-singular, or the result rect has zero area or
exceeds the op's pixel ceiling. It SHALL NOT recomposite the document and MUST
NOT panic on any input.

#### Scenario: An identity quad is the source rect

- **WHEN** `transform_layer_quad` is called with `quad` equal to the source rect corners
- **THEN** it returns true and the layer rect and every plane are unchanged

#### Scenario: A translated quad matches the similarity translation

- **WHEN** `transform_layer_quad` is called with every corner translated by an integer `(dx, dy)`
- **THEN** the resulting layer rect and channel planes equal those produced by `transform_layer` with the same translation

#### Scenario: A perspective quad maps the corners onto the targets

- **WHEN** a projective quad with a non-affine corner is applied
- **THEN** the source corners map onto the requested `quad` within a small tolerance

#### Scenario: Degenerate quads are refused

- **WHEN** `quad` has a non-finite corner, three collinear corners, or zero area
- **THEN** it returns false and every field of `doc` equals its pre-call value

#### Scenario: Pixels outside the source are transparent

- **WHEN** a projective quad pulls part of the destination outside the source rect
- **THEN** every destination pixel whose source point is outside the source rect has all channels equal to `0`

#### Scenario: Past the horizon only the front side is drawn [quad_front_side]

- **WHEN** a projective quad leaves part of the source rect beyond the horizon
- **THEN** every destination pixel whose inverse-mapped source point is on the far side is transparent, while the front side is drawn

#### Scenario: The projective preview clips at the horizon [preview_front_side]

- **WHEN** a live projective preview's map has a horizon crossing the source
- **THEN** the preview draws only the front side and never the mirrored far side

#### Scenario: A crossed quad fills an hourglass [quad_bowtie]

- **WHEN** `transform_layer_quad` is called with a quad whose top corners have swapped sides
- **THEN** pixels inside the two triangles between the crossing edges are drawn and pixels beside them, where a homography would draw, are transparent

#### Scenario: A lopsided bowtie keeps straight edges [quad_bowtie_straight]

- **WHEN** a crossed quad's edges cross off-centre (its top edge tilted)
- **THEN** the drawn pixels are exactly the two triangles formed by the quad's edges and the crossing point, with no curved waist

### Requirement: Skew, Distort, and Perspective menu commands

The application SHALL provide implemented commands `Edit > Transform > Skew`,
`Edit > Transform > Distort`, and `Edit > Transform > Perspective` that begin a
Free Transform session in the matching mode on the active layer, using the same
active-layer resolution and refusal as `Edit > Free Transform`. The Skew command
SHALL be enabled exactly when `Edit > Free Transform` is enabled, including for
a target whose selected pixels Free Transform lifts. The Distort and Perspective
commands SHALL additionally be disabled when the target is an editable type
layer whose whole layer would move; a selection that lifts its pixels keeps the
commands enabled.

#### Scenario: The Distort command enters a Distort session

- **WHEN** `Edit > Transform > Distort` is invoked with exactly one transformable layer active
- **THEN** a session begins in `Distort` mode on that layer

#### Scenario: The Distort command is disabled for editable type [distort_menu_type]

- **WHEN** the active layer is an editable type layer and no selection lifts its pixels
- **THEN** the Distort command is disabled and invoking it begins no session

#### Scenario: The Perspective command is disabled for editable type [perspective_menu_type]

- **WHEN** the active layer is an editable type layer and no selection lifts its pixels
- **THEN** the Perspective command is disabled and invoking it begins no session

#### Scenario: A non-transformable target leaves the command disabled

- **WHEN** the active layer is a group, an adjustment layer, or position-locked, or is the Background with no active selection
- **THEN** the Distort/Skew/Perspective commands are disabled and invoking them begins no session

### Requirement: Free Transform of selected pixels

When a non-empty, document-sized selection is active and the target is a pixel layer or the Background that is not pixel-locked, not transparency-locked while it has alpha, and not position-locked unless it is the Background, beginning a Free Transform session (in any mode) SHALL lift the selected pixels: they are copied into a floating layer directly above the target, trimmed to the selection bounds, and cleared from the target as `Edit > Clear` clears them. The session SHALL then transform the floating layer. Committing a non-identity transform SHALL composite the floating layer's pixels back over the target's own channels and remove it, leaving every other target attribute (name, locks, Background status, opacity, blend, mask, effects) unchanged; a target with alpha SHALL grow to hold pixels moved past its rect, and the Background SHALL clip them to its rect. The floating layer SHALL be sized to the selection bounds, not the document. A selection whose pixels are already the clear colour (white on a Background) SHALL still lift. A commit SHALL record exactly one `"Free Transform"` history state and SHALL drop the selection. Cancelling, or committing an identity transform, SHALL restore the document exactly as it was before the lift and record nothing. `layer_can_free_transform` SHALL report such a target as transformable, so `Edit > Free Transform` is enabled for it. Without a selection, the existing refusals (including the Background) SHALL apply unchanged.

#### Scenario: A Background with no selection stays refused

- **WHEN** Ctrl+T is pressed on an opened photo whose only layer is the Background and no selection is active
- **THEN** `Edit > Free Transform` is disabled and no session begins

#### Scenario: A selection on the Background is lifted and transformed

- **WHEN** a rectangle is selected on the Background, Ctrl+T is pressed, the box is dragged 10 px right and 5 px down, and Return commits
- **THEN** one `"Free Transform"` state is recorded, the document still has a single Background layer, the vacated pixels are white, the moved pixels land at the offset, and the selection is gone

#### Scenario: Lifting and merging back leaves the layer exactly as it was

- **WHEN** a selection on a layer with 50 % opacity, Multiply blend, and a mask is lifted and put back untransformed
- **THEN** the document equals the one before the lift

#### Scenario: Cancel restores the lifted pixels

- **WHEN** a session that lifted a selection is cancelled with Escape
- **THEN** the floating layer is gone, every pixel is as before, and no history state is recorded

### Requirement: Free Transform's box hugs the layer content

Beginning a Free Transform session on a whole pixel layer (no lifted selection) SHALL first trim the layer's rect to the bounds of its non-transparent pixels when it has alpha and no mask, so the transform box and its handles surround the visible content rather than the canvas-sized rect. Only fully transparent pixels SHALL be dropped, so the composite is unchanged and no history state is recorded for the trim.

#### Scenario: Ctrl+T on a canvas-sized layer boxes its content

- **WHEN** Ctrl+T is pressed on a canvas-sized layer whose only pixels are a 12 × 9 square at (10, 8)
- **THEN** the transform quad is that square's bounds, and cancelling leaves the composite unchanged

### Requirement: Transform overlay is not clipped to the document edge

The Free Transform overlay — the bounding quad, its eight handles, the rotate
knob, and the reference point — SHALL be painted without clipping to the
document rectangle, so a layer transformed past the canvas edge keeps its full
box and handles. Only the rasterised preview content SHALL be clipped to the
canvas. The rule SHALL apply to the modal session and to the Move tool's
controls box, which share the overlay path.

#### Scenario: An off-canvas corner keeps its handle [ft_overlay_unclipped]

- **WHEN** a layer is scaled so one corner lies outside the document and the canvas is painted
- **THEN** the off-canvas corner's handle is drawn exactly as the on-canvas corner's handle

#### Scenario: The preview content stays clipped [ft_preview_clipped]

- **WHEN** the transformed layer extends past the document edge
- **THEN** its rasterised preview is not painted outside the canvas rectangle

### Requirement: Live projective preview uses a CPU warp

The live preview for a projective transform SHALL render the moving layer by
inverse-mapping the visible destination region through the projective matrix
and sampling the preview layer image on the CPU, rather than drawing the image
under a projective `QTransform`. The warp SHALL keep the whole shape at every
convex quad, SHALL omit pixels on the far side of the horizon, and SHALL be
bounded by the visible canvas region sampled at the view's scale (at most one
sample per document pixel). A source that crosses the horizon SHALL NOT be
bounded by its projected corners. The warp SHALL be rebuilt when the view pans,
zooms, rotates, or resizes. While a preset warp is active the preview SHALL
draw the whole warped preview layer. When the target quad crosses itself the
warp SHALL fill the same two straight-edged triangles as the committed op, and
an empty result SHALL draw nothing. The warp SHALL divide by the projective
divisor whatever its sign, so a front side lying behind a negative divisor is
drawn. Otherwise, when the warp produces no image the
preview MAY fall back to the projective `QTransform` draw.

#### Scenario: A steep Distort keeps the whole shape [distort_preview_full]

- **WHEN** a live Distort preview maps a filled rectangle to a steep convex quad
- **THEN** the preview shows the layer across the whole quad, not one triangle

#### Scenario: Masked and clipped content survives the warp [distort_preview_layers]

- **WHEN** the moving layer's appearance comes from a layer mask, vector mask, or clipping group
- **THEN** the warped preview shows that appearance across the whole quad

#### Scenario: The far side of the horizon is omitted [distort_preview_front]

- **WHEN** the warp's projective map has a horizon through the source
- **THEN** the preview draws only the front side and leaves the far side transparent

#### Scenario: A zoomed-out preview samples at the view scale [distort_preview_scale]

- **WHEN** the view is zoomed out below 100% during a live projective preview
- **THEN** the warp samples about one output pixel per screen pixel, covering the same document region

#### Scenario: Changing the viewport redraws the preview [distort_preview_viewport]

- **WHEN** the view zooms out or pans during a live projective preview without another transform update
- **THEN** the newly visible parts of the moving layer are drawn

#### Scenario: A preset warp is not clipped to the original box [warp_preview_whole]

- **WHEN** a preset warp is applied after the transform moved content outside the original box
- **THEN** the preview draws that content

#### Scenario: A crossed preview fills an hourglass [preview_bowtie]

- **WHEN** a live projective preview's target quad has its top corners swapped
- **THEN** the preview draws the two triangles between the crossing edges and nothing beside or above them

#### Scenario: A lopsided crossed preview keeps straight edges [preview_bowtie_straight]

- **WHEN** a live preview's crossed target quad has edges crossing off-centre
- **THEN** every pixel centre off the triangle edges is drawn exactly when it lies inside one of the two triangles

### Requirement: Skew, Distort, and Perspective transform gestures

The Free Transform session SHALL expose a mode of `Free` (the existing
similarity), `Skew`, `Distort`, or `Perspective`. In `Free` the existing scalars
drive the transform; in a projective mode the session SHALL hold the live target
quad and the scalars SHALL be unused. A corner drag in `Distort` SHALL move that
corner by the press-to-pointer delta, preserving the grab offset, and SHALL stop
where the quad would become concave or fold over itself. A `Distort` corner drag
SHALL carry the reference point with the quad, keeping its position within the
box. When document guides are visible, a `Distort` corner SHALL snap to a
vertical guide and/or a horizontal guide within 8 screen pixels of its proposed
position, and the snapped position SHALL be the one committed. `Distort` SHALL
be refused for an editable type layer unless the session targets pixels lifted
by a selection. A corner drag in `Perspective` SHALL project the press-to-pointer
delta onto the press-time directions of the corner's two edges, pick the edge
with the larger projection, and move the corner along that edge by the
projection while the edge's other corner moves by the same amount in the
opposite direction, so the edge's midpoint stays fixed. The edge SHALL be
re-picked on every move of the same drag. The two corners MAY meet and pass
through each other, turning the box into a triangle and then a bowtie. An edge
drag in `Perspective` SHALL slide that edge's two endpoints along the edge's own
axis, leaving the opposite edge fixed. A `Perspective` drag SHALL carry the
reference point, keeping its bilinear position within the box. `Perspective`
SHALL be refused for an editable type layer unless the session targets pixels
lifted by a selection.
In `Skew`, a corner drag SHALL move that corner alone along whichever of its two
adjacent edges the drag follows most, and an edge drag SHALL slide that edge's
two endpoints only along the edge's own axis. Holding Alt in `Skew` SHALL move
the opposite corner, or the opposite edge, by the same displacement in the
opposite direction; Shift SHALL have no effect in `Skew`. A `Skew` corner drag on
an editable type layer SHALL slide the whole followed edge, keeping the quad a
parallelogram so the text stays type. A `Skew` corner drag SHALL stop where the
quad would become concave or fold over itself. A `Distort` or `Skew` gesture
that would make the quad degenerate SHALL be clamped to the last convex quad, or
refused when no finite convex result exists. A `Perspective` gesture that would
make the quad degenerate (a non-finite corner, under 1 px² of area, or three
consecutive corners on one line) SHALL be refused, keeping the last quad. Hit testing in a projective mode
SHALL accept only the eight bounding handles; a press away from a handle SHALL
not move or rotate. Committing SHALL call `transform_layer_quad` for the
projective modes and `transform_layer` for `Free`, recomposite, and record
exactly one `"Free Transform"` history state; an identity commit SHALL record
none; Escape SHALL leave the document bit-identical with no history state added.
The precise gesture semantics are inferred from Photoshop's Distort, Skew, and
Perspective modes (a CS6 screen recording, no reference image oracle); the
projective map itself is exact.

#### Scenario: Distort commits the dragged corner

- **WHEN** a `Distort` session drags one corner and commits
- **THEN** the layer's source corner is at its press-time position plus the pointer delta and exactly one `"Free Transform"` state is added

#### Scenario: A Distort corner keeps the grab offset [distort_grab_offset]

- **WHEN** a `Distort` corner is grabbed a few pixels away from its exact handle position and dragged
- **THEN** the corner moves by the pointer delta and does not jump to the pointer

#### Scenario: A Distort corner stops before folding [distort_convex]

- **WHEN** a `Distort` session drags a corner past the point where the quad would become concave
- **THEN** the stored quad stays convex and the dragged corner has not crossed to the other side

#### Scenario: A Distort corner carries the reference point [distort_pivot]

- **WHEN** a `Distort` session drags a corner
- **THEN** the reference point keeps the same position within the moving box

#### Scenario: A Distort corner snaps to a visible guide [distort_guide_snap]

- **WHEN** a `Distort` corner is dragged within 8 screen pixels of a visible document guide
- **THEN** the corner lands on the guide and the committed quad uses that position

#### Scenario: Distort is refused on editable type [distort_type]

- **WHEN** `Distort` is begun on an editable type layer without a selection that lifts its pixels
- **THEN** no session begins and the document is unchanged

#### Scenario: A Perspective corner spreads its edge symmetrically [perspective_corner]

- **WHEN** a `Perspective` session drags the top-right corner of an unrotated box mostly to the right and commits
- **THEN** the top-right corner moves right by the horizontal delta, the top-left corner moves left by the same amount, the bottom corners stay put, and exactly one `"Free Transform"` state is added

#### Scenario: A Perspective drag re-picks its edge [perspective_switch]

- **WHEN** a `Perspective` corner drag first moves mostly sideways and then, in the same drag, mostly vertically
- **THEN** the quad first spreads the corner's horizontal edge and then instead spreads its vertical edge, with the horizontal edge back at its press-time position

#### Scenario: A tilted edge spreads along itself [perspective_tilted]

- **WHEN** a `Perspective` corner drag starts from a box whose edge is no longer axis-aligned
- **THEN** both corners of the picked edge move along that edge's press-time line, about its midpoint

#### Scenario: Perspective corners may cross into a bowtie [perspective_bowtie]

- **WHEN** a `Perspective` corner is dragged past its edge partner
- **THEN** the two corners have swapped sides of the edge's midpoint and the session keeps the crossed quad

#### Scenario: A Perspective edge handle slides only its edge [perspective_edge]

- **WHEN** a `Perspective` session drags the top edge handle diagonally
- **THEN** the top corners move only along the top edge's axis and the bottom corners stay put

#### Scenario: A Perspective drag carries the reference point [perspective_pivot]

- **WHEN** a `Perspective` session with the reference point at the box centre drags a corner
- **THEN** the reference point equals the average of the new quad's corners

#### Scenario: Perspective is refused on editable type [perspective_type]

- **WHEN** `Perspective` is begun on an editable type layer without a selection that lifts its pixels
- **THEN** no session begins and the document is unchanged

#### Scenario: A Skew corner follows its dominant edge [skew_corner]

- **WHEN** a `Skew` session drags a corner mostly along one of its two adjacent edges
- **THEN** only that corner moves, along that edge, and the other corners stay put

#### Scenario: Skew slides one edge

- **WHEN** a `Skew` session drags an edge handle with an off-axis component
- **THEN** the edge's two endpoints move only along the edge's axis and the opposite edge is unchanged

#### Scenario: Alt mirrors the opposite corner or edge [skew_alt]

- **WHEN** a `Skew` session drags a corner or edge with Alt held
- **THEN** the opposite corner or edge moves by the same displacement in the opposite direction

#### Scenario: Skew keeps editable type a parallelogram [skew_type]

- **WHEN** a `Skew` session on an editable type layer drags a corner
- **THEN** the corner's whole followed edge slides and the quad remains a parallelogram

#### Scenario: A Skew corner stops before folding [skew_convex]

- **WHEN** a `Skew` session drags a corner past the point where the quad would become concave
- **THEN** the stored quad stays convex and the dragged corner has not crossed to the other side

#### Scenario: Escape cancels a projective mode exactly

- **WHEN** a projective-mode session is begun, a gesture is applied, and Escape is pressed
- **THEN** the document equals its pre-session value and the history count is unchanged

#### Scenario: A degenerate drag is refused

- **WHEN** a projective gesture would place three corners on one line
- **THEN** the session's quad is not collapsed to a degenerate shape

### Requirement: Free Transform perspective shortcut

In a `Free` session, a corner press with Ctrl, Alt, and Shift all held SHALL
start a perspective drag: the drag SHALL move the live quad exactly as a
`Perspective` corner drag does, including re-picking the edge on every move and
allowing a bowtie. The session SHALL then keep the resulting box as a
perspective base, a quad in source space after which the existing similarity
(scale, rotate, move, skew scalars, and the numeric setters) is applied, so the
live quad is the similarity applied to the base's corners. Later plain drags
SHALL keep scaling, rotating, and moving that box; a scale drag SHALL scale the
box in its own projective frame, keeping the opposite handle's edge or corner
exactly in place. Further Ctrl+Alt+Shift corner drags SHALL edit the
same base. A session with a base SHALL preview and commit through the projective
quad op and record exactly one `"Free Transform"` state; Escape SHALL leave the
document bit-identical. The shortcut SHALL be ignored, leaving an ordinary scale
drag, on an editable type layer whose whole layer would move. Edge handles and
the move and rotate areas SHALL keep their ordinary Free behavior with the
modifiers held. The composition order is inferred (no Adobe oracle).

#### Scenario: Ctrl+Alt+Shift on a corner drags perspective [free_perspective]

- **WHEN** a `Free` session presses a corner with Ctrl, Alt, and Shift held and drags mostly sideways
- **THEN** the quad spreads that corner's horizontal edge symmetrically, exactly as a `Perspective` corner drag would

#### Scenario: A plain drag scales the perspective box [free_perspective_scale]

- **WHEN** a `Free` session that has a perspective base drags its right edge handle without modifiers
- **THEN** the box's left edge stays fixed, the right corners move, and the box keeps its perspective (its side edges stay non-parallel)

#### Scenario: The perspective box commits through the quad op [free_perspective_commit]

- **WHEN** a `Free` session with a perspective base is committed
- **THEN** the layer's corners land on the live quad and exactly one `"Free Transform"` state is recorded

#### Scenario: The shortcut is ignored on editable type [free_perspective_type]

- **WHEN** a `Free` session on an editable type layer presses a corner with Ctrl, Alt, and Shift held and drags
- **THEN** the drag scales the box and no perspective base is created

### Requirement: Perspective drag angle readout

While a `Perspective` drag is in progress (a `Perspective`-mode drag or the Free
Transform shortcut), the canvas SHALL show a drag readout next to the pointer
with the angle in degrees, one decimal place. The angle SHALL be the rotation,
from the press-time quad, of the box side that joins the moved corner to its
unmoved neighbour (for an edge handle, either side the edge drags). Its sign
SHALL be negated when the moved edge is the top or bottom edge, so spreading an
edge reads negative and pinching it positive. The readout SHALL clear on
release. The angle convention is inferred from the CS6 recording (no Adobe
oracle).

#### Scenario: Spreading the top edge reads a negative angle [perspective_readout]

- **WHEN** a `Perspective` drag moves the top-right corner of a 100 × 100 box 100 px to the right
- **THEN** the drag readout shows `-45.0°`

#### Scenario: The readout clears on release [perspective_readout_clear]

- **WHEN** the `Perspective` drag is released
- **THEN** the drag readout is empty

### Requirement: A pending transform locks layer edits

While a transform is pending (a modal Free Transform, `Edit > Transform`, or
Warp session that has not been committed or cancelled), every command under the
`Layer` menu SHALL be disabled and SHALL refuse dispatch, and the Layers panel
SHALL be disabled and shown greyed, so no layer can be created, deleted,
hidden, reordered, or otherwise edited. Both SHALL return to their own state as
soon as the session commits or cancels. The Move tool's `Show Transform
Controls` box, which commits each gesture on release, SHALL NOT lock.

#### Scenario: The Layer menu is disabled during a transform [lock_layer_menu]

- **WHEN** a Free Transform session is pending
- **THEN** `Layer > New > Layer`, `Layer > Delete > Layer`, `Layer > Hide Layers`, and `Layer > Arrange > Send Backward` are disabled, dispatching New Layer adds no layer, and after the commit they are enabled again

#### Scenario: The Layers panel is greyed during a transform [lock_layers_panel]

- **WHEN** a Perspective session is pending
- **THEN** the Layers panel is disabled, and it is enabled again once the session is cancelled

### Requirement: A tool switch asks to apply a pending transform

Choosing another tool while a transform is pending, by the tool strip, a
flyout, or a tool keyboard shortcut, SHALL first ask "Apply the
transformation?" with `Apply`, `Cancel`, and `Don't Apply`. `Apply` SHALL commit
the transform (one `"Free Transform"` state) and switch tools; `Don't Apply`
SHALL cancel it (no state) and switch tools; `Cancel` SHALL keep the transform
pending and the current tool active. Without a pending transform no prompt
SHALL appear. On the offscreen platform (headless runs) and under the control
server the prompt SHALL NOT show and SHALL answer `Don't Apply`.

#### Scenario: Apply commits and switches [switch_apply]

- **WHEN** a moved Free Transform is pending and the Brush tool is chosen and `Apply` is answered
- **THEN** one `"Free Transform"` state is recorded, no session remains, and the Brush tool is active

#### Scenario: Don't Apply cancels and switches [switch_dont_apply]

- **WHEN** a moved Free Transform is pending and the Gradient tool is chosen and `Don't Apply` is answered
- **THEN** no state is recorded, no session remains, and the Gradient tool is active

#### Scenario: Cancel keeps the transform [switch_cancel]

- **WHEN** a transform is pending and another tool is chosen and `Cancel` is answered
- **THEN** the session is still pending, the previous tool is still active, and the layers stay locked

#### Scenario: A tool shortcut asks too [switch_shortcut]

- **WHEN** a transform is pending and the Brush tool's shortcut key is pressed
- **THEN** the prompt is asked, and the answer decides as it does for a click

#### Scenario: No prompt without a pending transform [switch_idle]

- **WHEN** no transform is pending and another tool is chosen
- **THEN** no prompt is asked and the tool switches
