## ADDED Requirements

### Requirement: History Brush source

The document history SHALL keep a History Brush source: the oldest state by
default, or a chosen state or named snapshot. A source index SHALL follow its
state when the depth limit drops older states, and a source that the depth
limit or a capture after undo would discard SHALL be kept as a pinned copy. A
state or snapshot that does not exist SHALL NOT become the source.

#### Scenario: The source follows its state

- **WHEN** the depth limit drops states older than the chosen source
- **THEN** the source still resolves to the same document

#### Scenario: A dropped source is pinned

- **WHEN** the depth limit or a capture after undo discards the source state
- **THEN** the brush paints from a pinned copy of it

### Requirement: History Brush tool

The History Brush SHALL paint the active layer as it was in the source state,
at the same location, through the source stroke, recording exactly one
"History Brush" history state per stroke that changed pixels, and SHALL refuse
a stroke when the source state has no pixel layer at the active layer's path.
The History panel SHALL mark the source row with the History Brush icon, and a
press in a row's left column SHALL choose that state or snapshot as the source
without jumping to it.

#### Scenario: Painting back the opening state

- **WHEN** the `history_brush_tool` self-test paints over a black Brush stroke with the default source
- **THEN** one "History Brush" state is recorded and the painted pixels are white again

#### Scenario: Choosing a source in the History panel

- **WHEN** the left column of the Brush state's row is pressed and the History Brush paints again
- **THEN** the current state is unchanged by the press, the source is the Brush state, and the stroke comes back black
