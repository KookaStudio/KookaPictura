## ADDED Requirements

### Requirement: Path Selection tool

Clicking a Work Path component (its outline, a lone anchor, or the inside of a
closed one) SHALL select it whole, drawing its anchors solid, without recording
history; clicking empty canvas SHALL clear the selection. Dragging a component
SHALL move it, handles included, and record exactly one "Drag Path" state;
Alt-dragging SHALL leave it in place and move a copy, recording one "Duplicate
Path Component" state. Delete or Backspace SHALL remove a selected component as
one "Delete Path" state. Show Bounding Box SHALL frame the selected component's
curve.

#### Scenario: Select, move, copy, and delete a component

- **WHEN** the `tst_path_selection_tools` test clicks inside a closed square beside an open line, drags it, toggles Show Bounding Box, Alt-drags it, and presses Delete
- **THEN** the click records nothing, the drag records one "Drag Path" state moving only the square, the bounding box frames its curve, the Alt-drag adds a moved copy as one "Duplicate Path Component" state, and Delete removes the copy as one "Delete Path" state
