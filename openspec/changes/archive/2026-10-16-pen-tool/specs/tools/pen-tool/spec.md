## ADDED Requirements

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
