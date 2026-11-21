# tools/pen-tool Specification

## Purpose
The Pen tool: place and edit Bezier anchors on the Work Path, close or resume
subpaths, and auto add/delete anchors.

## Requirements

### Requirement: Pen tool

A Pen click SHALL add a corner anchor to the Work Path and a press-and-drag
SHALL add a smooth anchor whose handles mirror the drag through the anchor;
Shift SHALL snap the new anchor (or handle) to 45°. Clicking the first anchor
of the subpath being drawn SHALL close it; Enter, Esc, Ctrl-click, or switching
tools SHALL end it open, and clicking an open endpoint SHALL resume drawing from
it. With Auto Add/Delete on and nothing being drawn, a click on a segment SHALL
add an anchor and a click on an anchor SHALL delete it. Each placed anchor SHALL
record exactly one history state, and undo SHALL restore the previous path.

#### Scenario: Drawing and closing a path

- **WHEN** the `tst_pen_tools` Pen test clicks twice, drags once, and clicks the first anchor
- **THEN** one "New Work Path", two "Add Anchor Point", and one "Close Path" state are recorded, the dragged anchor is smooth with mirrored handles, and the subpath is closed

#### Scenario: Auto Add/Delete

- **WHEN** the `tst_pen_tools` test clicks a closed path's segment and then the new anchor with the Pen
- **THEN** one "Add Anchor Point" and one "Delete Anchor Point" state are recorded; with Auto Add/Delete off the same click starts a new subpath

### Requirement: Paths panel lists the Work Path

Once the Work Path has an anchor, the Paths panel SHALL list it as one row
named "Work Path" in italics, selected, with a thumbnail of the path on the
canvas; with no Work Path it SHALL show "No Paths". The panel SHALL follow
edits, undo, and redo of the path.

#### Scenario: The first anchor creates the row

- **WHEN** the `tst_pen_tools` panel test places a Pen anchor, then undoes every anchor
- **THEN** the panel lists one selected, italic "Work Path" row with a thumbnail, and after the undo it lists none

### Requirement: Pen Enter waits for an unfocused value input

The Pen tool's Enter path finish SHALL run only when no keyboard-editable value
input has focus. While a value input has focus, Enter SHALL belong to the input
(see `ui/value-input-enter`); Esc, Ctrl-click, and switching tools SHALL still
end the subpath regardless of focus.

#### Scenario: A focused value field keeps the path open [pen_path_enter_field_focus]

- **WHEN** a subpath is being drawn, a value input has focus, and Enter is
  pressed
- **THEN** the field commits and the subpath stays open; a later Enter with no
  value input focused ends it open

### Requirement: Pen output modes

The Pen and Freeform Pen options bar SHALL offer Mode: Shape, Path, or Pixels,
defaulting to Shape. Shape and Pixels SHALL land when a drawing session ends —
by closing the path, Enter, Esc, Ctrl-click, a Freeform release, or switching
tools. Shape SHALL create a new shape layer from the drawn components and clear
the Work Path; Pixels SHALL stroke the path onto the active pixel layer with the
current brush and foreground colour. Path SHALL save the drawn path as the next
`Path <n>` saved path when it is closed or explicitly finished (Enter, Esc,
Ctrl-click, or a Freeform release) and leave the Work Path in place; switching
tools only ends the session. The Add, Delete, and Convert anchor tools SHALL NOT
change the output mode or land a path.

#### Scenario: The mode control is present and shared [pen_output_mode_bar]

- **WHEN** the `tst_pen_tools` mode test selects the Pen and then the Freeform Pen
- **THEN** both options bars show Shape, Path, and Pixels with Shape selected, and changing one page's mode changes the other's

#### Scenario: Shape mode creates a shape layer [pen_output_shape]

- **WHEN** the `tst_pen_tools` shape test closes a Pen square with Mode Shape
- **THEN** one shape layer is added, the Work Path is empty, the layer shows the foreground fill inside the square, and one "Pen Tool" state is recorded

#### Scenario: Path mode saves the path [pen_output_path]

- **WHEN** the `tst_pen_tools` path test closes a Pen square with Mode Path
- **THEN** a saved path named "Path 1" is added, and the Work Path still holds the four closed anchors

#### Scenario: Pixels mode strokes the layer [pen_output_pixels]

- **WHEN** the `tst_pen_tools` pixels test closes a Pen square with Mode Pixels
- **THEN** no layer is added, the square's edge is painted with the foreground colour, and one "Stroke Path" state is recorded
