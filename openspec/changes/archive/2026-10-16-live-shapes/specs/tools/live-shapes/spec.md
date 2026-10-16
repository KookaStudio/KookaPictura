## ADDED Requirements

### Requirement: Live shape layers

A Rectangle, Rounded Rectangle, or Ellipse drawn in Shape mode SHALL be a live
shape: its layer SHALL carry a `vogk` block recording the shape's origin type,
box, and corner radii beside its `vmsk` outline. A Polygon SHALL NOT be live.
While a shape tool is active outside Path mode, the active shape layer's
outline SHALL be drawn with its anchors.

#### Scenario: Live origin data reads elsewhere

- **WHEN** the `vector_mask_oracle` test saves a live rounded rectangle and reads it with psd-tools
- **THEN** psd-tools reports origin type 2, the rectangle's box, and its four radii

#### Scenario: The new shape shows its corners

- **WHEN** the `tst_shape_tools` test draws a Rectangle in Shape mode
- **THEN** the canvas draws the new layer's outline with four anchors and the layer is live

### Requirement: Turning a live shape into a path

Path Selection and Direct Selection SHALL edit the active shape layer's
outline when there is one. Moving a live shape's component whole SHALL keep it
live. Starting to drag an anchor or handle of a live shape with Direct
Selection SHALL first ask "This operation will turn a live shape into a
regular path. Continue?" with Yes, No, and "Don't show again": No SHALL leave
the shape unchanged; Yes SHALL make it a regular path; "Don't show again" SHALL
be remembered so later conversions happen without asking. Undoing the edit
that follows a conversion SHALL restore the live shape.

#### Scenario: Prompt, convert, reshape, undo

- **WHEN** the `tst_shape_tools` test moves a live rectangle with Path Selection, drags its corner with Direct Selection answering No, then Yes, drags it again, undoes, then answers Yes with "Don't show again" and reshapes a new live ellipse
- **THEN** the move keeps it live, No changes nothing, Yes makes it a regular path without recording a state, the next drag reshapes it as one "Drag Anchor Point" state without asking, undo makes it live again, and the ellipse is reshaped without a prompt
