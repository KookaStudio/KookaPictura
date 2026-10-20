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
`0`, and bilinear taps SHALL be edge-clamped. The operation SHALL refuse and MUST
leave `doc` bit-identical for every case `transform_layer` refuses, and also when
any corner is non-finite, the projective map is singular or near-singular, or the
result rect has zero area or exceeds the op's pixel ceiling. It SHALL NOT
recomposite the document and MUST NOT panic on any input.

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

### Requirement: Skew, Distort, and Perspective transform modes

The Free Transform session SHALL expose a mode of `Free` (the existing
similarity), `Skew`, `Distort`, or `Perspective`. In `Free` the existing scalars
drive the transform; in a projective mode the session SHALL hold the live target
quad and the scalars SHALL be unused. A corner drag in `Distort` SHALL set that
corner to the pointer; a corner drag in `Perspective` SHALL set that corner to
the pointer and move the opposite corner by the negated delta about the quad
centre. In `Skew`, a corner drag SHALL move that corner alone along whichever of
its two adjacent edges the drag follows most, and an edge drag SHALL slide that
edge's two endpoints only along the edge's own axis. Holding Alt in `Skew` SHALL
move the opposite corner, or the opposite edge, by the same displacement in the
opposite direction; Shift SHALL have no effect in `Skew`. A `Skew` corner drag on
an editable type layer SHALL slide the whole followed edge, keeping the quad a
parallelogram so the text stays type. A `Skew` corner drag SHALL stop where the
quad would become concave or fold over itself. A projective-mode gesture that
would make the quad degenerate SHALL be refused. Hit testing in a projective mode
SHALL accept only the eight bounding handles; a press away from a handle SHALL
not move or rotate. Committing SHALL call `transform_layer_quad` for the
projective modes and `transform_layer` for `Free`, recomposite, and record
exactly one `"Free Transform"` history state; an identity commit SHALL record
none; Escape SHALL leave the document bit-identical with no history state added.
The precise gesture semantics are inferred from Photoshop's Skew mode (no
reference image oracle); the projective map itself is exact.

#### Scenario: Distort commits the dragged corner

- **WHEN** a `Distort` session drags one corner and commits
- **THEN** the layer's source corner is at the pointer's document position and exactly one `"Free Transform"` state is added

#### Scenario: Perspective mirrors the opposite corner

- **WHEN** a `Perspective` session drags a corner by a delta and commits
- **THEN** the opposite corner moves by the negated delta and the quad centre is unchanged

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

### Requirement: Skew, Distort, and Perspective menu commands

The application SHALL provide implemented commands `Edit > Transform > Skew`,
`Edit > Transform > Distort`, and `Edit > Transform > Perspective` that begin a
Free Transform session in the matching mode on the active layer, using the same
active-layer resolution and refusal as `Edit > Free Transform`. Each command
SHALL be enabled exactly when `Edit > Free Transform` is enabled, including for
a target whose selected pixels Free Transform lifts.

#### Scenario: The Distort command enters a Distort session

- **WHEN** `Edit > Transform > Distort` is invoked with exactly one transformable layer active
- **THEN** a session begins in `Distort` mode on that layer

#### Scenario: A non-transformable target leaves the command disabled

- **WHEN** the active layer is a group, an adjustment layer, or position-locked, or is the Background with no active selection
- **THEN** the Distort/Skew/Perspective commands are disabled and invoking them begins no session

### Requirement: Mesh warp op

The system SHALL provide
`pictura_render::transform_layer_warp(doc: &mut Document, path: &str, mesh:
&WarpMesh, params: WarpParams) -> bool`, where `WarpMesh` is a row-major control
net of `rows × cols` points in the source rect's local space and `WarpParams`
carries `distort_h`/`distort_v` percentages. It SHALL resample every channel
plane (and the layer mask, about its own rect) through the tensor-product cubic
Bézier surface defined by the control net, after scaling each row about its edge
midpoint by `1 + (2v−1)·distort_v/100` and each column likewise by `distort_h`.
A destination pixel whose source point lies outside the source rect SHALL be `0`,
and the result `rect` SHALL be the integer bounding box of the forward-mapped
lattice. The op SHALL refuse and leave `doc` bit-identical for every case
`transform_layer` refuses, and additionally when a mesh point is non-finite,
`rows` or `cols` is below 2, or the point count is not `rows·cols`. It SHALL NOT
recomposite and MUST NOT panic. A uniform control net SHALL map a point to
itself (the identity), so an identity mesh leaves the document byte-identical.
The named Adobe warp presets and their `Bend`/`X`/`Y` geometry are out of scope
(no Photoshop oracle); parity is behavioral.

#### Scenario: An identity mesh is a no-op

- **WHEN** `transform_layer_warp` is called with `identity_mesh` over the source rect and zero distortion
- **THEN** it returns true and the layer rect and every plane are unchanged

#### Scenario: An interior control point deforms the interior

- **WHEN** an interior control point of a 4×4 identity mesh is moved and the op runs
- **THEN** the layer rect grows to the deformed bounding box and the four corner pixels are unchanged

#### Scenario: Degenerate meshes are refused

- **WHEN** a mesh point is non-finite, `rows` or `cols` is below 2, or the point count does not equal `rows·cols`
- **THEN** the op returns false and every field of `doc` equals its pre-call value

#### Scenario: Out-of-source pixels are transparent

- **WHEN** the mesh pulls part of the destination outside the source rect
- **THEN** every such destination pixel has all channels equal to `0`

### Requirement: Preset warp style mesh generation

`pictura_render` SHALL expose `pub enum WarpStyle` with the variants `None`,
`Custom`, `Arc`, `ArcLower`, `ArcUpper`, `Arch`, `Bulge`, `ShellLower`,
`ShellUpper`, `Flag`, `Wave`, `Fish`, `Rise`, `Fisheye`, `Inflate`, `Squeeze`,
and `Twist`, each carrying a CS6 display name, a stable id, and a `from_id`
constructor over a full ordered list. It SHALL expose
`style_mesh(style: WarpStyle, bend: f64, rotate_vertical: bool, w: i32, h: i32)
-> Option<WarpMesh>`. `style_mesh` SHALL return `None` for `None`, `Custom`, a
non-finite `bend`, or a non-positive `w`/`h`, and otherwise SHALL return the
preset control net in the warp box's local space with `bend` clamped to
`[-100, 100]`.

The 15 preset constructions SHALL match SethRobinson/Patchy's
`generate_style_warp_mesh` (commit
`7d14d1f6ede2dc8fb52c11eefcc7cc8783473711`, MIT) within `1e-9` at
`bend ∈ {-100, -50, 0, 50, 100}` and both orientations, and `bend == 0` SHALL
equal `identity_mesh` for that style's natural grid within `1e-9`.

#### Scenario: A preset at bend zero is the style identity grid

- **WHEN** `style_mesh` is called at `bend = 0` for any of the 15 presets and either orientation
- **THEN** the returned net has that construction's `cols`/`rows` and every point equals `identity_mesh(cols, rows, w, h)` within `1e-9`

#### Scenario: None and Custom produce no preset mesh

- **WHEN** `style_mesh` is called with `WarpStyle::None` or `WarpStyle::Custom`
- **THEN** it returns `None`

#### Scenario: Vertical orientation transposes the construction

- **WHEN** `style_mesh` is called with `rotate_vertical = true` for a non-Twist preset on a non-square box
- **THEN** the returned net has the horizontal net's `cols`/`rows` swapped and its coordinates transposed

#### Scenario: Twist ignores the orientation

- **WHEN** `style_mesh(WarpStyle::Twist, bend, ...)` is called with `rotate_vertical` false and true
- **THEN** both calls return the same control net

#### Scenario: Negative and upper styles mirror their counterparts

- **WHEN** `style_mesh(WarpStyle::Arc, -bend, false, ...)` or `style_mesh(WarpStyle::ArcUpper, bend, false, ...)` or `style_mesh(WarpStyle::ShellUpper, bend, false, ...)` is called
- **THEN** the result is the vertical mirror (`y -> h - y`, rows reversed) of the positive `Arc`, `ArcLower`, or `ShellLower` net respectively

### Requirement: Warp preset command and dialog

The application SHALL provide an `Edit > Transform > Warp` command (id
`edit.transform.warp`) that opens a Warp dialog collecting a Warp Style, a Bend
percent, X and Y distortion percents, and a vertical-orientation flag. The
dialog SHALL disable the Bend, X/Y, and orientation controls while the style is
`None` or `Custom`.

On accept, the `PictureView` bridge `apply_warp_preset(path, style, bend,
distort_x, distort_y, rotate_vertical)` SHALL build the style's mesh from the
target layer's rect, call `transform_layer_warp` with
`WarpParams { distort_h: distort_x, distort_v: distort_y }`, recomposite, and
record exactly one `"Warp"` history state when the engine returns true. It SHALL
return false without mutating the document and without recording history when
there is no document, the style id is unknown, `style_mesh` returns `None`
(`None`/`Custom`), or the engine refuses. The command SHALL be enabled only when
the active view has a document and the selected path passes
`layer_can_free_transform`.

#### Scenario: A named preset applies in one history state

- **WHEN** `apply_warp_preset` runs on a raster layer with the `warpArc` id at 50% bend and no distortion
- **THEN** it returns true, the layer rect changes, and exactly one `"Warp"` history state is appended

#### Scenario: None and unknown styles add no history

- **WHEN** `apply_warp_preset` is called with `warpNone`, `warpCustom`, or an unknown id
- **THEN** it returns false and the history count and document are unchanged

#### Scenario: The command requires a transformable target

- **WHEN** no layer is selected or the selected path cannot be free-transformed
- **THEN** `Edit > Transform > Warp` is disabled

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
