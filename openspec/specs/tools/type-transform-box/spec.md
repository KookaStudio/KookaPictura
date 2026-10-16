# tools/type-transform-box Specification

## Purpose
The Type session's transform box: resize, scale, rotate, and the reference point, with readouts.

## Requirements

### Requirement: The type session's transform box

While a Type tool session is open, paragraph type's box, and point type's text
bounds while Ctrl is held, SHALL show eight handles, a rotate band just outside
the box, and a reference point at the box centre, each with its own cursor.
Dragging a paragraph box's handle SHALL resize the box and reflow the text, and
dragging a handle with Ctrl, or on point type's box, SHALL scale the text about
the opposite edge or corner. Both SHALL show W and H in the rulers' unit by the
pointer. Dragging in the rotate band SHALL rotate the text and its box about the
reference point and show the angle. Dragging the reference point SHALL move it
and show its X and Y. The transform SHALL be the type layer's own, so the text
stays editable, and nothing SHALL be recorded until the session commits.

#### Scenario: The frame's gestures [ttb_frame_math]

- **WHEN** `tst_paragraph_type::frameGestureMath` rotates a box a quarter turn, scales a box from a corner, resizes one edge, and drags the reference point
- **THEN** the rotation reads 90° and keeps the reference point fixed, the scale keeps the opposite corner fixed and stretches each axis by the drag, the resize moves only that edge, and the reference point moves by the drag

#### Scenario: Rotate in the band, with the angle [ttb_rotate]

- **WHEN** `tst_paragraph_type::boxRotatesWithReadout` types into a dragged box, hovers just outside a corner, drags around the reference point, releases, and commits
- **THEN** the zone is the rotate band, the readout shows the angle while dragging and clears on release, the box turned, nothing was recorded before the commit, and the committed layer's transform is rotated

#### Scenario: Resize shows W and H [ttb_resize_readout]

- **WHEN** `tst_paragraph_type::resizeShowsSizeReadout` drags a paragraph box's corner handle
- **THEN** the zone over the handle is a handle, and the readout shows W over H

#### Scenario: The reference point drags [ttb_pivot]

- **WHEN** `tst_paragraph_type::pivotDrags` presses the reference point and drags it by (10, 6)
- **THEN** the zone there is the reference point, it moves by (10, 6), and the readout shows X over Y

#### Scenario: Ctrl scales point type [ttb_ctrl_scale]

- **WHEN** `tst_paragraph_type::ctrlScalesPointType` types point type, holds Ctrl, drags the box's bottom-right handle outward, and commits
- **THEN** a box appears only while Ctrl is held, and the committed layer is scaled up
