# Spec Delta

## MODIFIED Requirements

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
workspace default arrow beyond the outer margin, and the new-crop crosshair only
in the init mode where no box is drawn. The cursor SHALL be re-resolved on a
modifier keypress or a framework refresh (tool switch, canvas rebind) and SHALL
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

## ADDED Requirements

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
