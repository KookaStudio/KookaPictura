# document/transform-session-warp Specification

## Purpose
Warp mode of a Free Transform session, ported from photocraft: the box baked
into an editable Bezier mesh or replaced by a preset style, previewed live, and
committed as one history state.

## Requirements

### Requirement: Warp mode commit

Committing a session in Warp mode SHALL apply the session's warp followed by
the box's projective frame (the session quad), resampling with the session's
interpolation through `transform_layer_warp`, then recomposite and record exactly one
`"Free Transform"` history state when the layer changed. An identity warp on an
untransformed box SHALL record nothing. Escape SHALL end the session with the document bit-identical to
its pre-session value and no history state added.

#### Scenario: A dragged anchor commits one state [tsw_commit]

- **WHEN** a Warp-mode session drags the top-left anchor up and left, then Apply is clicked
- **THEN** the layer's rect starts at the dragged corner and exactly one `"Free Transform"` state is added

#### Scenario: An untouched warp records nothing [tsw_identity]

- **WHEN** a session enters Warp mode and commits without an edit
- **THEN** the history count is unchanged

#### Scenario: Escape restores a warped session exactly [tsw_escape]

- **WHEN** a Warp-mode session picks a preset and Escape is pressed
- **THEN** the document equals its pre-session value and the history count is unchanged

### Requirement: Warp mode

`Edit > Transform > Warp` SHALL open the active layer's Free Transform session in
Warp mode, or switch an open session into it, as photocraft's Warp mode does.
Entering Warp SHALL start the identity one-patch Bezier mesh over the session's
source box and keep the box's current quad as a projective frame applied after
the warp (as Photoshop keeps `Trnf` apart from `warp`), so a transform made
first, Distort and Perspective included, carries over exactly. While warping, the session SHALL report the mesh's corner anchors
as its quad, and the canvas SHALL draw the patch boundaries, the
rule-of-thirds guides of a one-patch grid, and, for a custom warp, its anchors
(always) and handles (near the pointer or while their anchor is dragged, and
back to near the pointer once the drag is released) in
place of the box and reference point. Leaving Warp SHALL drop the warp and
restore the box as it was. Post-CS6 split warps and the Grid menu are not
provided.

#### Scenario: The menu opens Warp mode [tsw_menu]

- **WHEN** `Edit > Transform > Warp` is invoked on a transformable layer
- **THEN** a session is open in Warp mode with the `custom` style, and the canvas shows four boundaries, four guides, and twelve points of which four are anchors

#### Scenario: The toggle switches between box and warp [tsw_toggle]

- **WHEN** the options-bar warp toggle is clicked twice during a session
- **THEN** the first click enters Warp mode (a box corner is now an anchor) and the second restores the box and its handles

#### Scenario: A box transform carries into the mesh [tsw_carry]

- **WHEN** a session is rotated 30° and then enters Warp mode
- **THEN** the mesh's corners equal the rotated box's corners, and leaving Warp gives back the 30° box

#### Scenario: A perspective box carries over exactly [tsw_carry_projective]

- **WHEN** a session's box is a perspective quad and Warp mode is entered
- **THEN** the grid's edges are where the box's homography puts the source box's edges, interior points included, and an untouched warp previews the projective transform

#### Scenario: Release returns the dragged handles to near-pointer [tsw_release]

- **WHEN** a custom warp's anchor is dragged on the canvas and released
- **THEN** its handles are drawn as dragged during the drag and the overlay holds no dragged handles after the release

### Requirement: Warp gestures

In Warp mode a press within 12 screen pixels of a custom warp's anchor or handle
SHALL drag that point: an anchor carries its handles, and a handle turns the
opposite handle. Only a custom warp's points SHALL be grabbable: a preset hides
them, so a press on a preset SHALL grab only its surface. A press elsewhere
inside the warped grid SHALL pull the surface under the pointer. Presses and
drags SHALL be taken through the box's projective frame, so a grabbed point
follows the pointer. A press outside the grid SHALL grab nothing. A press
that grabs something on a preset SHALL first turn the preset into the same shape
as a custom mesh.

#### Scenario: Presses grab a point, the surface, or nothing [tsw_grab]

- **WHEN** a custom warp is pressed on an anchor, on a handle, inside the grid, and far outside it
- **THEN** the press grabs the anchor, the handle, the surface, and nothing respectively

#### Scenario: A press on a preset makes it custom [tsw_preset_press]

- **WHEN** a Bulge preset is pressed outside its grid, then inside it, and dragged
- **THEN** the outside press grabs nothing and keeps Bulge, the inside press pulls the surface and the style becomes `custom`, and the grabbed shape equals the preset's

#### Scenario: A preset's hidden handles are not grabbed [tsw_preset_hidden]

- **WHEN** a Bulge preset is pressed on one of its control points that lies off its surface
- **THEN** the press grabs nothing and the style stays Bulge

### Requirement: Warp preview

Every warp edit SHALL refresh the preview by warping the layer on a document
clone with the session's interpolation and handing only the moving image to the
canvas. The document MUST equal its pre-session value until commit.

#### Scenario: A preset previews its shape [tsw_preview]

- **WHEN** Arc is chosen in Warp mode
- **THEN** the preview image is taller than the unwarped layer image and no history state is added
