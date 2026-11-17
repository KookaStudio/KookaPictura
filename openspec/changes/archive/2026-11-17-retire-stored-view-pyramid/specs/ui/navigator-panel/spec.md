## MODIFIED Requirements

### Requirement: Navigator thumbnail from the view pyramid

The Navigator thumbnail SHALL be drawn from the document's tile pyramid (`compositing/tile-pyramid`) — a
level, or a crop of a level, sized to the widget — rather than from a fresh
reduction of the full-resolution composite. It MUST NOT allocate a second
full-resolution copy of the document. The thumbnail SHALL refresh when the
document content changes.

#### Scenario: The thumbnail comes from a pyramid level

- **WHEN** the Navigator paints a thumbnail for a large document
- **THEN** it samples a tile-pyramid level sized to the widget and does not
  rescale the full-resolution composite

#### Scenario: No second full-resolution copy is held

- **WHEN** the Navigator is shown for a document already using a tile pyramid
- **THEN** no additional full-resolution document buffer is allocated for the
  thumbnail

#### Scenario: A content change refreshes the thumbnail

- **WHEN** the document composite changes
- **THEN** the Navigator thumbnail redraws from the updated composite tiles at that level
