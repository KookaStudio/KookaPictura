# crop-straighten Specification

## Purpose

Photoshop-style rotate-crop: the image and layers spin behind a fixed, axis-aligned crop box while the canvas stays axis-aligned and grows to the rotated content's bounding box, so committing a crop crops to the box without truncating the content.

## Requirements

### Requirement: Straighten model

While a crop is active the system SHALL support a rotation angle applied to the
document content without tilting the crop box or the canvas frame. The crop box
SHALL remain axis-aligned and keep its position and size as the angle changes;
the content SHALL spin about the crop box centre. The rotation pivot SHALL be
the crop box centre at the moment the angle is set and SHALL remain fixed as the
box is moved afterwards, so repositioning the box leaves the content in place.
The displayed canvas SHALL remain axis-aligned and SHALL grow to the
axis-aligned bounding box of the rotated content, so no layer is truncated at
the original canvas edges. The canvas bounding box SHALL contain the rotated
content for any angle.

#### Scenario: The box holds while the content spins [lcs_box_fixed]

- **WHEN** a non-zero straighten angle is applied during an active crop
- **THEN** the crop box keeps its position and size and stays axis-aligned, while the content rotates about the box centre

#### Scenario: Moving the box leaves the content in place [lcs_move_keeps_content]

- **WHEN** the crop box is moved after a straighten angle has been set
- **THEN** the content stays in place and only the box moves

#### Scenario: The canvas grows to contain the rotated content [lcs_canvas_bbox]

- **WHEN** a non-zero straighten angle is applied to a non-square document
- **THEN** the canvas stays axis-aligned, its aspect ratio becomes the rotated content's bounding box, and no layer corner is cut off

#### Scenario: Default angle is a no-op

- **WHEN** the Crop tool is chosen and no rotation is applied
- **THEN** the model is identical to the axis-aligned crop with zero angle

### Requirement: Crop rotation gesture

The system SHALL enter the rotate gesture when a box is active and a press
begins outside the box, at least a resize margin away from it (so the gesture
does not conflict with the resize handles), including past the canvas edge, and
SHALL show a rotate cursor there. Dragging SHALL change the straighten angle
about the box centre. A new box SHALL be drawn only in the init mode where no box
exists; a press outside the box while a box is active SHALL NOT draw a new box.
Holding Shift during rotation SHALL snap the angle to 15° steps. Releasing SHALL
record the new angle in the tool session.

#### Scenario: Outside the box enters rotate [lcs_rotate_zone]

- **WHEN** a press begins outside the crop box, beyond the resize margin
- **THEN** the system is in the rotate gesture, the cursor is the rotate cursor, and dragging changes the angle about the box centre

#### Scenario: Rotate past the canvas edge [lcs_rotate_past_canvas]

- **WHEN** a press begins outside the crop box past the canvas edge, not on a handle
- **THEN** the system is in the rotate gesture and no new box is drawn

#### Scenario: New box only in init mode [lcs_new_box_init_mode]

- **WHEN** a drag begins while no box is drawn (the init mode)
- **THEN** a new crop box is drawn, clamped to the canvas

#### Scenario: The resize margin does not rotate [lcs_rotate_margin]

- **WHEN** a press begins within the resize margin just outside the crop box edge
- **THEN** the resize handle is grabbed and no rotation begins

#### Scenario: Shift snaps rotation [lcs_rotate_shift]

- **WHEN** the user rotates with Shift held
- **THEN** the applied angle is an exact multiple of 15°

### Requirement: Crop pointer cursors

While a crop is active the pointer cursor SHALL reflect the zone under the
pointer: a resize cursor over a handle, the workspace default arrow over the box
body, the rotate cursor outside the box (beyond the resize margin, including
past the canvas edge), and the new-crop crosshair only in the init mode where no
box is drawn. The cursor SHALL be re-resolved on a modifier keypress or a
framework refresh (tool switch, canvas rebind) and SHALL NOT revert to the tool's
static cursor while the crop is active. The workspace default arrow SHALL be the
bare tool-arrow the tool cursors are built on (tip at the top left, a vertical
left edge, an up-right barb, a 0° notch, and a 45° edge back to the tip; black
with a white outline), used over the canvas and its surrounding workspace; panels,
menus, and dialogs SHALL keep the system cursor.

#### Scenario: Resize over a handle [lcs_cursor_handle]

- **WHEN** the pointer is over a crop handle
- **THEN** the cursor is the resize cursor for that handle

#### Scenario: Default arrow over the box [lcs_cursor_box]

- **WHEN** the pointer is over the crop box body
- **THEN** the cursor is the workspace default arrow

#### Scenario: Rotate outside the box [lcs_cursor_rotate]

- **WHEN** the pointer is outside the box beyond the resize margin
- **THEN** the cursor is the rotate cursor, even past the canvas edge

#### Scenario: New-crop in init mode [lcs_cursor_newcrop]

- **WHEN** no crop box is drawn (the init mode)
- **THEN** the cursor is the new-crop cursor

#### Scenario: A modifier keypress keeps the cursor [lcs_cursor_modifier]

- **WHEN** a modifier key is pressed while the crop is active
- **THEN** the cursor is re-resolved to the same zone, not reverted to the tool's static cursor

### Requirement: Crop rotate cursor zones

Outside the box the rotate cursor SHALL take one of eight orientations chosen by
the pointer's zone around the crop-box centre: a diagonal for each of the four
corners and an axis for each of the four edges. The corner zones SHALL be wider
than a bare 45° wedge so the corner variant is reachable on a small box. The
eight zones SHALL partition the area outside the box, and the cursor SHALL be
the curved double-arrow used by Free Transform.

#### Scenario: Corner zone picks the diagonal [lcs_rotate_cursor_corner]

- **WHEN** the pointer is outside a corner, within the widened corner wedge
- **THEN** the rotate cursor is oriented along that corner's diagonal

#### Scenario: Edge zone picks the axis [lcs_rotate_cursor_edge]

- **WHEN** the pointer is outside the middle of an edge
- **THEN** the rotate cursor is oriented along that edge's axis

### Requirement: Crop box growth

The first crop box drawn in the init mode SHALL be clamped to the canvas. Once a
box exists, a handle resize MAY extend it beyond the canvas, and the displayed
canvas SHALL grow to contain the box. Committing such a box SHALL produce a
document of the box's size, padding the area outside the original canvas with the
background colour when the document has a Background layer and transparency
otherwise.

#### Scenario: The first box is clamped [lcs_grow_first_clamped]

- **WHEN** a first box is drawn with the pointer dragged past the canvas edge
- **THEN** the box is clamped to the canvas

#### Scenario: A resized box grows the canvas [lcs_grow_resize_out]

- **WHEN** a handle of an existing box is dragged past the canvas edge
- **THEN** the box keeps the out-of-canvas extent and the displayed canvas grows to contain it

#### Scenario: Committing a grown box pads [lcs_grow_commit_pad]

- **WHEN** a box larger than the canvas is committed on a document with a Background layer, and again without one
- **THEN** the document is the box's size and the added area is the background colour, or transparency without a Background layer

### Requirement: Crop canvas backdrop

When the crop box extends beyond the canvas or the content is rotated, the area
outside the content SHALL be filled with the background colour (the FG/BG pair)
when the document has a Background layer, and SHALL show the transparency
checkerboard otherwise.

#### Scenario: Background fills the expanded canvas [lcs_backdrop]

- **WHEN** the crop box grows beyond the canvas on a document with a Background layer
- **THEN** the expanded area is filled with the background colour; without a Background layer it is transparent

### Requirement: Crop snapping

While resizing or moving the crop box, each edge or corner SHALL snap to the
nearest canvas edge or layer bounding edge when within a screen-pixel threshold,
so the box aligns exactly with the canvas or a layer. A snap SHALL NOT move an
axis that is not within the threshold, and snapping SHALL respect a set aspect
ratio.

#### Scenario: Corner snaps to the canvas [lcs_snap_canvas]

- **WHEN** a crop handle is dragged to within the threshold of a canvas corner
- **THEN** the box corner is placed exactly on the canvas corner

#### Scenario: Edge snaps to a layer [lcs_snap_layer]

- **WHEN** a crop edge is dragged to within the threshold of a layer's bounding edge
- **THEN** the crop edge is placed exactly on that layer edge

#### Scenario: No snap outside the threshold

- **WHEN** a crop handle is dragged beyond the threshold from every candidate
- **THEN** the box follows the pointer with no snap

### Requirement: Crop shield scope

The crop shield SHALL dim only the canvas area that lies outside the crop box.
It SHALL NOT dim the workspace surface around the canvas. The shield SHALL
follow the crop box and the canvas bounding box as the crop rotates.

#### Scenario: The workspace is not dimmed [lcs_shield_canvas_only]

- **WHEN** a crop box smaller than the canvas is shown
- **THEN** the canvas region outside the box is dimmed and the workspace surface around the canvas is not dimmed

### Requirement: Straighten commit

Committing a straighten crop SHALL produce the box-sized result sampled from the
rotated content and SHALL record exactly one history state. With Delete Cropped
Pixels on, pixels outside the committed box SHALL be discarded as for an
axis-aligned crop; with it off they SHALL be kept beyond the new canvas edge.
Cancelling SHALL leave the document unchanged and discard the tool session.

#### Scenario: Commit yields the box content [lcs_commit_box]

- **WHEN** a crop with a non-zero angle is applied
- **THEN** the document becomes the crop box's size and its content matches the rotated content sampled inside the box

#### Scenario: Commit records one state [lcs_commit_one_state]

- **WHEN** a straighten crop is applied after several angle changes
- **THEN** exactly one history state is added

#### Scenario: Cancel returns to the init crop mode [lcs_cancel]

- **WHEN** Escape or Cancel is invoked during a straighten crop
- **THEN** the drawn box and angle are cleared, the document is unchanged, the tool session is discarded, and the Crop tool remains active with no box shown

#### Scenario: A fresh box after cancel [lcs_cancel_redraw]

- **WHEN** the user drags after cancelling
- **THEN** a new crop box is drawn freely, honoring the active aspect ratio

### Requirement: Crop preview and active states

The Crop tool SHALL distinguish a preview state from an active state. A box in
the preview state SHALL be drawn as a dashed outline without rule-of-thirds
guides and without resize handles; an active box SHALL be drawn with its frame,
guides, and eight handles. Selecting the tool in Modern SHALL show a preview box
fitted to the active aspect ratio and centered on the canvas; selecting it in
Classic SHALL show no box. Dragging inside the preview SHALL draw a new active
box clamped to the canvas; a press and release without movement inside the
preview SHALL adopt the preview as the active box; a rotate press outside SHALL
activate the box and begin rotation. Escape and Cancel SHALL clear the box and
return to no box in both modes.

#### Scenario: Modern starts in preview [lcs_preview_modern]

- **WHEN** the Crop tool is selected in Modern mode
- **THEN** a dashed preview box, centered and fitted to the active ratio, is shown with no handles or guides

#### Scenario: Dragging the preview draws a new box [lcs_preview_draw]

- **WHEN** a drag begins inside the preview box
- **THEN** a new active crop box is drawn, clamped to the canvas

#### Scenario: Clicking the preview activates it [lcs_preview_click]

- **WHEN** the preview box is pressed and released without movement
- **THEN** the preview box becomes the active crop box

#### Scenario: Escape clears to no box [lcs_preview_escape]

- **WHEN** Escape or Cancel is invoked in either mode
- **THEN** the box and angle are cleared and no box is shown

### Requirement: Crop repositioning

Dragging inside an active box SHALL differ by mode: in Classic it SHALL move the
box; in Modern it SHALL keep the box fixed in the workspace and pan the composite
behind it. A Modern pan SHALL change the region the box selects, and committing
SHALL crop to the region currently under the box.

#### Scenario: Modern drag pans the composite [lcs_modern_pan]

- **WHEN** an active Modern box is dragged from inside
- **THEN** the box stays fixed in the workspace, the composite moves behind it, and the cropped region changes accordingly

#### Scenario: Classic drag moves the box [lcs_classic_move]

- **WHEN** an active Classic box is dragged from inside
- **THEN** the box moves and the composite stays in place

### Requirement: Crop options fields

In ratio mode the `W` and `H` fields SHALL show unit-less aspect-ratio values and
SHALL render as integers unless a decimal value is entered. Only the
`W x H x Resolution` entry SHALL show the pixel dimensions and a resolution
field. The resolution SHALL default to the document's resolution, falling back to
72 ppi when the document has none, with a unit of pixels per inch or pixels per
centimeter. The `Cancel` and `Apply` controls SHALL be visible only while a box
is active.

#### Scenario: Ratio values are unit-less [lcs_fields_ratio]

- **WHEN** a ratio preset or the free `Ratio` entry is chosen
- **THEN** the `W`/`H` fields show the ratio values without a unit and without a slider control

#### Scenario: Resolution mode shows pixels and a resolution [lcs_fields_resolution]

- **WHEN** the `W x H x Resolution` entry is chosen
- **THEN** the `W`/`H` fields show pixel dimensions and the resolution field shows the document resolution with its unit

#### Scenario: Cancel and Apply only when active [lcs_fields_commit_visibility]

- **WHEN** no active box is shown
- **THEN** the Cancel and Apply controls are hidden

### Requirement: Straighten from the init state

A straighten line drawn while no active box exists SHALL activate the crop first
so that the rotation can be previewed: Classic SHALL use the full canvas as the
box and Modern SHALL use the preview box. The rotation SHALL preview from the
first drag frame and update immediately on release.

#### Scenario: Drawing a straighten line activates the crop [lcs_straighten_activates]

- **WHEN** the straighten toggle is armed and a horizon line is drawn with no active box
- **THEN** a crop box is activated and the content rotation previews during the drag and on release
