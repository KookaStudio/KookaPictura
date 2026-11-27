# tools/ruler-tool Specification

## Purpose
The Ruler tool: a measuring line (view state, never saved or undone) and its X, Y, W, H, A, D1 readout.

## Requirements

### Requirement: Ruler measurement

`pictura_core::Ruler::measure` SHALL report the start X and Y, the width and
height from start to end, the length D1, and the angle A in degrees
anticlockwise from east as seen on screen, so a line running down-right reads
negative. `Ruler::constrained` SHALL snap the end to the nearest 45° ray from
the start.

#### Scenario: A 3-4-5 line

- **WHEN** a ruler runs from (0, 0) to (3, 4)
- **THEN** W is 3, H is 4, D1 is 5, and A is -53.13°

#### Scenario: Shift snaps to 45°

- **WHEN** the end is at (10, 1) from a start at the origin, constrained
- **THEN** the end snaps to (10, 0)

### Requirement: Ruler tool

The Ruler tool SHALL draw a measuring line by dragging (Shift snaps it to 45°),
move the nearer end when a drag starts on an end, and move the whole line when
a drag starts on it; a click without a drag SHALL leave no line. The line SHALL
be view state: never recorded in history nor saved. The options bar SHALL show
X, Y, W, H, A, and D1 for the line and a Clear that removes it. The canvas SHALL
show the line only while the Ruler is active.

#### Scenario: Measuring without history

- **WHEN** the `annotation_tools` self-test drags a ruler from (0, 0) to (3, 4)
- **THEN** the options bar reads A -53.1° and D1 5.0 and no history state is added

#### Scenario: The line follows the tool

- **WHEN** another tool is selected and the Ruler is selected again
- **THEN** the line is hidden and then shown again

### Requirement: Ruler protractor

A Ruler line SHALL support a second arm from either endpoint by Alt-dragging that endpoint or double-clicking the line and dragging. The pressed endpoint SHALL be the shared vertex for Alt-drag; double-clicking SHALL choose the endpoint nearest the click, choosing the end on an equal-distance tie. Creating either arm SHALL preserve the ordered primary start and end, D1, and the primary line used by Arbitrary rotation and Straighten. Shift SHALL constrain the new arm to 45-degree increments. The canvas SHALL show both arms and their endpoints while the Ruler is active. The options bar and Info panel SHALL show D1, D2 and the nonnegative included angle between the arms; clearing the measurement SHALL remove both arms. A protractor SHALL remain transient view state and SHALL NOT add history.

#### Scenario: Create and constrain a protractor [protractor_create]

- **WHEN** the user Alt-drags from each endpoint of an existing ordered line, drawing a second arm with Shift
- **THEN** the pressed endpoint is the vertex, the primary start and end and D1 remain unchanged, both arms and the correct D2 and nonnegative included angle are displayed, and the second arm snaps to a 45-degree ray without adding history

#### Scenario: Double-click creates a protractor [protractor_double]

- **WHEN** the user double-clicks an existing measuring line away from its endpoints and drags, including an equal-distance click
- **THEN** the nearer endpoint is the vertex (the end on a tie), the ordered primary line remains unchanged, and a second arm appears with its length and included angle reported

#### Scenario: Clear and restore visibility [protractor_clear]

- **WHEN** the user switches away from and back to the Ruler, then clears its protractor
- **THEN** the arms hide and return with the tool, then both disappear and D2 clears

### Requirement: Straighten from a ruler line

The Ruler options bar SHALL offer Straighten for a nonzero measuring line. It SHALL rotate the entire document to align the line with its nearest horizontal or vertical axis, expand the canvas as necessary, and use the current image interpolation preference. A successful rotation SHALL create exactly one undoable history state; a zero-angle or absent line and a refused rotation SHALL leave the document and history unchanged. The existing Arbitrary rotation prefill SHALL continue to use the primary line.

#### Scenario: Straighten and undo [ruler_straighten]

- **WHEN** the user draws a sloped line and activates Straighten
- **THEN** the image rotates to align the line within 0.1 degrees, the canvas expands as needed, and one Undo restores the original document

#### Scenario: Straighten no-op and refusal [ruler_straighten_noop]

- **WHEN** no usable line exists, the line is already aligned, or the rotation is refused
- **THEN** no document or history change occurs
