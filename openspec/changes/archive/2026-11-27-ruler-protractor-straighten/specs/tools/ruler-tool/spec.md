## ADDED Requirements

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
