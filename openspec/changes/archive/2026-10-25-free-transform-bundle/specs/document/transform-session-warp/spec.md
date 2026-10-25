# Spec Delta

## MODIFIED Requirements

### Requirement: Warp mode commit

Committing a session that holds a warp SHALL apply the warp followed by the
box's projective frame (the session quad), resampling with the session's
interpolation through `transform_layer_warp`, then recomposite and record
exactly one `"Free Transform"` history state when the layer changed, whether the
session is still in Warp mode or has returned to the box. Apply and Cancel keep
their session-wide meaning: Apply commits through this path, and Cancel or
Escape ends the session without a document change. An identity warp on an
untransformed box SHALL record nothing. Escape SHALL end the session with the
document bit-identical to its pre-session value and no history state added.
Cancel, Escape, and an Undo or Redo that cancels a live transform SHALL clear the
canvas transform preview, box, and warp grid.

#### Scenario: A dragged anchor commits one state [tsw_commit]

- **WHEN** a Warp-mode session drags the top-left anchor up and left, then Apply is clicked
- **THEN** the layer's rect starts at the dragged corner and exactly one `"Free Transform"` state is added

#### Scenario: An untouched warp records nothing [tsw_identity]

- **WHEN** a session enters Warp mode and commits without an edit
- **THEN** the history count is unchanged

#### Scenario: Escape restores a warped session exactly [tsw_escape]

- **WHEN** a Warp-mode session picks a preset and Escape is pressed
- **THEN** the document equals its pre-session value, the canvas holds no transform preview or grid, and the history count is unchanged

#### Scenario: Cancel clears a warped session [tsw_cancel_overlay]

- **WHEN** a Warp-mode session picks a preset and Cancel is clicked
- **THEN** the session ends, the canvas holds no transform preview or grid, the document equals its pre-session value, and no history state is added

#### Scenario: Undo cancels a live transform and clears its overlay [tsw_undo_live]

- **WHEN** a transform session is live and Edit > Undo is invoked
- **THEN** the session ends, the canvas shows no transform preview, box, or grid, and the document moves back one history state

#### Scenario: A retained warp commits with a box edit [tsw_commit_retained]

- **WHEN** a warp is edited, the session returns to the box, the box is scaled, and the session commits
- **THEN** the layer shows the warp and the box scale together and exactly one `"Free Transform"` state is added

### Requirement: Warp mode

`Edit > Transform > Warp` SHALL open the active layer's Free Transform session
in Warp mode, or switch an open session into it, as photocraft's Warp mode does.
Entering Warp on a session without a warp SHALL start the identity one-patch
Bezier mesh over the session's source box and keep the box's current quad as a
projective frame applied after the warp (as Photoshop keeps `Trnf` apart from
`warp`), so a transform made first, Distort and Perspective included, carries
over exactly. Entering Warp on a session that already holds a warp SHALL reuse
that mesh, style, bend, and distortion. The session's box SHALL remain its
quad in Warp mode, while the canvas SHALL draw the patch boundaries, the
rule-of-thirds guides of a one-patch grid, and, for a custom warp, its anchors
(always) and handles (near the pointer or while their anchor is dragged, and
back to near the pointer once the drag is released) in place of the reference
point. Leaving Warp SHALL keep the warp in the session, return the box and its
reference point, and move the box to the warped preview's document bounds. A
source-space post-frame SHALL keep the preview pixels unchanged while the box
moves; the warp SHALL stay applied, the post-frame SHALL apply after the mesh,
and the box's transform SHALL apply after that frame. While a warp is stored and
Warp mode is not active, the canvas SHALL show the box (now the warped bounds)
and reference point, and every box change (a handle drag, a numeric
W/H/angle/skew set, or an exact rotate or flip) SHALL refresh the preview
through the warp. Re-entering Warp SHALL not lose the mesh, and the grid SHALL
follow the same post-frame. A new transform session (a mode command, a new
target, or a tool switch) SHALL not carry the mesh over. Post-CS6 split warps
and the Grid menu are not provided.

#### Scenario: The menu opens Warp mode [tsw_menu]

- **WHEN** `Edit > Transform > Warp` is invoked on a transformable layer
- **THEN** a session is open in Warp mode with the `custom` style, and the canvas shows four boundaries, four guides, and twelve points of which four are anchors

#### Scenario: The toggle switches between box and warp [tsw_toggle]

- **WHEN** the options-bar warp toggle is clicked twice during a session
- **THEN** the first click enters Warp mode (the box stays the box while the grid appears), the second restores the box and its handles at the warped preview's bounds, and the warp stays in the session

#### Scenario: A box transform carries into the mesh [tsw_carry]

- **WHEN** a session is rotated 30° and then enters Warp mode
- **THEN** the mesh's corners equal the rotated box's corners, and leaving Warp gives back the 30° box, now at the warped preview's bounds, with the warp still applied

#### Scenario: A perspective box carries over exactly [tsw_carry_projective]

- **WHEN** a session's box is a perspective quad and Warp mode is entered
- **THEN** the grid's edges are where the box's homography puts the source box's edges, interior points included, and an untouched warp previews the projective transform

#### Scenario: Release returns the dragged handles to near-pointer [tsw_release]

- **WHEN** a custom warp's anchor is dragged on the canvas and released
- **THEN** its handles are drawn as dragged during the drag and the overlay holds no dragged handles after the release

#### Scenario: Leaving and re-entering Warp keeps the mesh [tsw_retain]

- **WHEN** a custom warp's anchor is dragged, Warp mode is left, and Warp mode is entered again
- **THEN** the anchor is still displaced, the style is still `custom`, and the grid follows the same preview

#### Scenario: A box drag keeps the warped preview [tsw_box_preview]

- **WHEN** a session holds a warp, Warp mode is left, and a box handle is dragged
- **THEN** the preview shows the scaled warped layer and the canvas shows the box, not the warp grid

#### Scenario: A numeric box edit refreshes the warped preview [tsw_box_numeric]

- **WHEN** a session holds a warp, Warp mode is left, and W is set to 200
- **THEN** the preview width grows with the box and the canvas still shows the box

#### Scenario: An exact transform refreshes the warped preview [tsw_box_exact]

- **WHEN** a session holds a warp, Warp mode is left, and Rotate 90° Clockwise is invoked
- **THEN** the preview rotates the warped layer with the box and the canvas still shows the box

#### Scenario: Leaving Warp moves the box to the warped bounds [tsw_box_bounds]

- **WHEN** a custom warp's anchor is dragged and Warp mode is left
- **THEN** the box equals the warped preview layer's document bounds, the preview pixels are unchanged, and re-entering Warp keeps the grid on the same preview
