## ADDED Requirements

### Requirement: Rotate View tool

Dragging with the Rotate View tool SHALL turn the displayed canvas about its
centre by the angle the pointer sweeps, without changing the document or
recording history, showing a compass whose needle points to the document's top
while dragging; Shift held SHALL snap the angle to 15° steps. The options bar's
Rotation Angle field and dial SHALL show and set the angle, and Reset View or
Esc SHALL return it to 0°. Pointer input SHALL map to the document point under
the pointer at any angle.

#### Scenario: Turn, snap, set, and reset

- **WHEN** the `tst_rotate_view` test selects the tool with R, rotates the view 90° and 180°, drags a quarter turn, Shift-drags 50°, edits the angle field, and presses Reset View and Esc
- **THEN** widget and document points round-trip through the mapping, the painted canvas shows each half of the image where the mapping puts it, the drag turns the canvas 90° with the compass shown only while dragging, Shift lands on 45°, the field and dial follow, Reset View and Esc return to 0°, and no history state is recorded
