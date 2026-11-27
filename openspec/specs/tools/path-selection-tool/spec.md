# tools/path-selection-tool Specification

## Purpose
The Path Selection tool: select and move whole paths and path components.

## Requirements

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

### Requirement: Select path components and anchors

Path Selection and Direct Selection SHALL support Shift-click toggling and marquee selection of multiple components or anchors respectively, including across components of the active Work, saved, or editable shape/vector-mask path. An ordinary click SHALL replace the relevant selection. Clicking empty canvas, switching tools, changing the path target, or invalidating selection through document changes SHALL clear it. Selected components and anchors SHALL be visually distinguishable. Selection alone SHALL NOT edit geometry or add history.

#### Scenario: Toggle and replace selection [pt_select]

- **WHEN** components or anchors in different components are selected by Shift-click and then an ordinary click selects another
- **THEN** Shift-click toggles the hit item, the ordinary click replaces the selection, and geometry and history remain unchanged

#### Scenario: Marquee selection [pt_marquee]

- **WHEN** a marquee encloses items in multiple components of the active path
- **THEN** the enclosed components or anchors are selected and visibly distinguished without changing geometry or history

#### Scenario: Clear and invalidate selection [pt_clear]

- **WHEN** multiple items are selected and the user clicks empty canvas, switches tools, changes the target, or a document change invalidates the selection
- **THEN** selection clears without adding history, including when the replacement target has matching indices
