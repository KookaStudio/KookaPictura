# crop-straighten Specification

## Purpose

Photoshop-style rotate-crop: the image and layers spin behind a fixed, axis-aligned crop box while the canvas stays axis-aligned and grows to the rotated content's bounding box, so committing a crop crops to the box without truncating the content.

## Requirements

### Requirement: Straighten model

While a crop is active the system SHALL support a rotation angle applied to the
document content without tilting the crop box or the canvas frame. The crop box
SHALL remain axis-aligned and keep its position and size as the angle changes,
except that a finished straighten reference line SHALL shrink it about its
centre (see Straighten box fit); the content SHALL spin about the crop box
centre. The rotation pivot SHALL be
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
begins outside the box, within a rotate band that runs from a resize margin just
outside the box edge (so the gesture does not conflict with the resize handles)
out to an outer margin of 100 screen pixels from the nearest box edge, measured
at any zoom. The band SHALL extend past the canvas edge. Within the band the
system SHALL show a rotate cursor and dragging SHALL change the straighten angle
about the box centre. A press beyond the outer margin SHALL NOT rotate:
while the box is active it SHALL apply the crop instead, and while the preview
state is shown it SHALL be a no-op. A new box SHALL be drawn only in the init
mode where no box exists; a press outside the box while a box is active SHALL
NOT draw a new box. Holding Shift during rotation SHALL snap the angle to 15°
steps. Releasing SHALL record the new angle in the tool session.

#### Scenario: Outside the box enters rotate [lcs_rotate_zone]

- **WHEN** a press begins outside the crop box, within the rotate band
- **THEN** the system is in the rotate gesture, the cursor is the rotate cursor, and dragging changes the angle about the box centre

#### Scenario: Rotate past the canvas edge [lcs_rotate_past_canvas]

- **WHEN** a press begins outside the crop box past the canvas edge but within the rotate band, not on a handle
- **THEN** the system is in the rotate gesture and no new box is drawn

#### Scenario: Beyond the rotate band applies the crop [lcs_rotate_far_apply]

- **WHEN** a press begins farther than the outer margin from an active crop box, on the canvas or on the workspace past the canvas edge
- **THEN** the crop is applied and no rotation begins

#### Scenario: A far press in preview does nothing [lcs_rotate_far_preview]

- **WHEN** a press begins farther than the outer margin from the box while the preview state is shown
- **THEN** no crop is applied, no box is adopted, and no rotation begins

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
body, the rotate cursor within the rotate band outside the box (from the resize
margin out to the 100-pixel outer margin, including past the canvas edge), the
workspace default arrow beyond the outer margin, the new-crop crosshair only
in the init mode where no box is drawn, and the straighten cursor (a crosshair
with a spirit level) anywhere while the straighten line is armed. The cursor SHALL be re-resolved on a
modifier keypress (a Ctrl keypress arms the straighten line) or a framework refresh (tool switch, canvas rebind) and SHALL
NOT revert to the tool's static cursor while the crop is active. The workspace
default arrow SHALL be the bare tool-arrow the tool cursors are built on (tip at
the top left, a vertical left edge, an up-right barb, a 0° notch, and a 45° edge
back to the tip; black with a white outline), used over the canvas and its
surrounding workspace; panels, menus, and dialogs SHALL keep the system cursor.

#### Scenario: Resize over a handle [lcs_cursor_handle]

- **WHEN** the pointer is over a crop handle
- **THEN** the cursor is the resize cursor for that handle

#### Scenario: Default arrow over the box [lcs_cursor_box]

- **WHEN** the pointer is over the crop box body
- **THEN** the cursor is the workspace default arrow

#### Scenario: Rotate outside the box [lcs_cursor_rotate]

- **WHEN** the pointer is outside the box beyond the resize margin but within the rotate band
- **THEN** the cursor is the rotate cursor, even past the canvas edge

#### Scenario: Default arrow beyond the band [lcs_cursor_far]

- **WHEN** the pointer is farther than the outer margin from the box
- **THEN** the cursor is the workspace default arrow

#### Scenario: New-crop in init mode [lcs_cursor_newcrop]

- **WHEN** no crop box is drawn (the init mode)
- **THEN** the cursor is the new-crop cursor

#### Scenario: A modifier keypress keeps the cursor [lcs_cursor_modifier]

- **WHEN** a modifier key is pressed while the crop is active
- **THEN** the cursor is re-resolved to the same zone, not reverted to the tool's static cursor

#### Scenario: Straighten armed [lcs_cursor_straighten]

- **WHEN** the straighten line is armed from the toggle or by holding Ctrl
- **THEN** the cursor is the straighten cursor over the canvas and its workspace

### Requirement: Crop rotate cursor zones

Within the rotate band outside the box the rotate cursor SHALL take one of eight
orientations chosen by the pointer's zone around the crop-box centre: a diagonal
for each of the four corners and an axis for each of the four edges. The corner
zones SHALL be wider than a bare 45° wedge so the corner variant is reachable on
a small box. The eight zones SHALL partition the rotate band, and beyond the
band the cursor SHALL be the workspace default arrow rather than a rotate
cursor. The cursor SHALL be the curved double-arrow used by Free Transform.

#### Scenario: Corner zone picks the diagonal [lcs_rotate_cursor_corner]

- **WHEN** the pointer is outside a corner, within the widened corner wedge and inside the rotate band
- **THEN** the rotate cursor is oriented along that corner's diagonal

#### Scenario: Edge zone picks the axis [lcs_rotate_cursor_edge]

- **WHEN** the pointer is outside the middle of an edge and inside the rotate band
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

The Crop tool SHALL distinguish a preview state (the box the tool starts with,
not yet a pending crop) from an active state. A box in the preview state SHALL
be drawn with its frame and eight handles but without rule-of-thirds guides; an
active box SHALL be drawn with its frame, guides, and eight handles. Selecting
the tool in either mode SHALL show a preview box fitted to the active aspect
ratio and centered on the canvas, as CS6 frames the whole image. Dragging a
preview handle SHALL resize the box, which becomes the active crop box; dragging
inside the preview SHALL draw a new active box clamped to the canvas; a press and release without movement inside the
preview SHALL adopt the preview as the active box; a rotate press outside SHALL
activate the box and begin rotation. Escape and Cancel SHALL clear the box and
return to no box in both modes.

#### Scenario: Modern starts in preview [lcs_preview_modern]

- **WHEN** the Crop tool is selected in Modern mode
- **THEN** a preview box, centered and fitted to the active ratio, is shown with its eight handles and no guides

#### Scenario: Classic starts in preview too [lcs_preview_classic]

- **WHEN** the Crop tool is selected in Classic mode
- **THEN** the same preview box with its handles is shown, not an empty canvas

#### Scenario: A preview handle resizes [lcs_preview_handle]

- **WHEN** a corner or edge handle of the preview box is dragged
- **THEN** the box is resized from that handle and becomes the active crop box

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
so that the rotation can be previewed: the preview box when one is shown, or the
full canvas when there is no box (after Escape). The rotation SHALL preview from the
first drag frame and update immediately on release.

#### Scenario: Drawing a straighten line activates the crop [lcs_straighten_activates]

- **WHEN** the straighten toggle is armed and a horizon line is drawn with no active box
- **THEN** a crop box is activated and the content rotation previews during the drag and on release

### Requirement: Crop handle rendering

The crop box SHALL draw its eight handles inside the box outline rather than
centred on it: four L-shaped corner brackets, each formed from two short strokes
inset from the corner along the box edges, and four thin flat bars centred on
the edge midpoints and hugging the inside of each edge. The handles SHALL keep a
fixed screen size at any zoom and SHALL be drawn white over a dark outline so
they read on any background. Only the drawn glyphs SHALL be inset; the handle
hit zones and resize cursors SHALL remain centred on the box border.

#### Scenario: Corners draw inside brackets [lcs_handle_corner]

- **WHEN** an active crop box is painted
- **THEN** each corner shows an L-shaped bracket inset inside the box outline, not a filled square centred on the border

#### Scenario: Edges draw inside bars [lcs_handle_edge]

- **WHEN** an active crop box is painted
- **THEN** each edge midpoint shows a thin flat bar hugging the inside of that edge

#### Scenario: Grab zones stay centred [lcs_handle_hitzone]

- **WHEN** a press begins within the resize margin of the box border
- **THEN** the resize gesture begins even though the drawn bracket is inset inside the box

### Requirement: Straighten control

The Crop options bar SHALL carry a checkable Straighten control: a spirit-level
icon (`crop.straighten`, original artwork) with the label `Straighten` to its right, where a click on the icon or the
label toggles the straighten line. Holding Ctrl (Cmd on macOS) while no other
crop gesture is in progress SHALL arm the straighten line and check the control
until the key is released; a press with Ctrl held SHALL draw a straighten line
even when no key event reached the canvas. Releasing Ctrl SHALL NOT disarm a
line armed from the control, and a line being drawn SHALL finish first. A
finished line SHALL disarm the control.

#### Scenario: Icon and label toggle [lcs_straighten_control]

- **WHEN** the Crop tool is active
- **THEN** the options bar shows the Straighten icon with its label, and a click toggles the straighten line on and off

#### Scenario: Holding Ctrl arms the line [lcs_straighten_ctrl]

- **WHEN** Ctrl is pressed and then released with the Crop tool active
- **THEN** the straighten line is armed while the key is held and disarmed on release, unless it was armed from the control

### Requirement: Straighten reference line angle

A straighten reference line from A to B SHALL rotate the content by the
negative of the line's inclination from the nearer of the horizontal and the
vertical, so the result lies in (-45°, 45°] whichever direction the line is
drawn. The rotation SHALL add to any angle already set in the session, so a
second line levels what is on screen. While the line is drawn the image SHALL
hold still and the rotation the release will apply SHALL show beside the pointer
in the rotate gesture's `∠: <angle>°` readout; releasing the line SHALL rotate
the image preview by that angle, keep the crop active, and clear the readout.

#### Scenario: A line at θ commits a rotation of −θ [lcs_straighten_angle]

- **WHEN** a reference line at θ = 5° from the horizontal is drawn and the crop is committed
- **THEN** the content is rotated by −θ within 0.1°

#### Scenario: The image rotates on release [lcs_straighten_release]
- **WHEN** a reference line is dragged and then released
- **THEN** the image stays unrotated during the drag, with the readout showing the coming angle, and the preview rotates by that angle on release
#### Scenario: Plumb lines and either direction [lcs_straighten_axis]

- **WHEN** a near-vertical line, or a line drawn right to left, is drawn
- **THEN** the angle levels it against the vertical or horizontal respectively, never rotating by more than 45°

### Requirement: Straighten box fit

When a straighten reference line finishes, the crop box SHALL shrink about its
centre, keeping its aspect ratio, to the largest box whose corners lie inside the
rotated image, so committing leaves no uncovered canvas corners. The box SHALL
NOT grow, and a box whose centre lies outside the rotated image SHALL be left
unchanged. The rotate-band gesture SHALL keep the box size.

#### Scenario: No uncovered corners [lcs_straighten_fit]

- **WHEN** a straighten line is drawn over a full-canvas box
- **THEN** every corner of the shrunken box maps inside the unrotated canvas and one corner touches an edge
