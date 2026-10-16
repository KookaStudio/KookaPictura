## ADDED Requirements

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

## MODIFIED Requirements

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
