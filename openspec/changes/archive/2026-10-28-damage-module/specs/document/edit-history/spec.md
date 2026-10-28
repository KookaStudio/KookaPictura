# Spec Delta

## ADDED Requirements

### Requirement: Hinted capture compares only declared tiles

When an edit supplies changed document rectangles, the history capture SHALL
compare only the tiles of each tracked plane that intersect those rectangles,
treating every other tile as equal, and SHALL store the same delta an unhinted
capture of the same change stores **when the rectangles cover every change**. An
edit without a hint, or with an empty hint, SHALL compare every tracked plane in
full. The hint MUST NOT change the stored state, the undo or redo result, the
history order, labels, cursor, depth bound, or redo truncation when it covers
every change.

A hint that does not cover every change is a caller contract violation. The
capture treats non-intersecting tiles as equal; that can omit a changed tile and
make the omission permanent once `stamp_agreed` marks the planes equal. This is
a mechanism test target, not a correctness guarantee.

A tracked plane's tile grid maps to document coordinates through the plane's
own origin: the composite plane maps each channel block's rows to document rows,
a layer channel maps its top-left to the layer rectangle, a raster mask maps its
top-left to the mask rectangle, and a document channel maps to the canvas. A
tile that partially intersects a hint rectangle SHALL be compared in full. A
capture whose tracked-plane geometry changed SHALL keep the existing full-state
fallback regardless of the hint.

#### Scenario: Hinted and unhinted captures store the same delta [eh_hint_equal]

- **WHEN** the same change is captured once with rectangles covering every change and once without a hint
- **THEN** both captures store the same plane deltas and the same tiles

#### Scenario: A hinted capture ignores tiles outside the hint [eh_hint_skips]

- **WHEN** a plane changes outside the declared rectangles while the rest of the change lies inside them
- **THEN** the hinted capture's delta contains only tiles that intersect the rectangles
- **AND** this is a caller-contract mechanism check, not a claim that the omitted tiles are equal

#### Scenario: Undo after a hinted capture is bit-identical [eh_hint_undo]

- **WHEN** a hinted capture records a change and the state is undone
- **THEN** the restored document and composite are byte-identical to the state before the change

#### Scenario: A hinted capture still falls back on a geometry change [eh_hint_geometry]

- **WHEN** the tracked-plane count, stride, height, or order changes between the two states
- **THEN** the capture starts a new full segment regardless of the hint

#### Scenario: An empty hint captures every plane [eh_hint_empty]

- **WHEN** a capture receives no hint or an empty hint
- **THEN** it compares every tracked plane in full and stores the same state an unhinted capture does

#### Scenario: Random hinted captures match unhinted captures [eh_hint_random]

- **WHEN** randomized bounded edits are captured once with covering hints and once without hints
- **THEN** every pair stores the same plane deltas and the same tiles, and undo restores the same state

#### Scenario: Document channels use their canvas tiles [eh_hint_document_channel]

- **WHEN** a bounded edit changes a document channel inside the hint
- **THEN** the capture compares and stores the corresponding canvas tiles for that channel
