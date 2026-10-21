# Spec Delta

## ADDED Requirements

### Requirement: A selection content move refreshes only its bounds

A committed selection content move SHALL refresh only the union of the
selection's source bounds and its destination bounds, clipped to the canvas,
through the regional composition path. The refreshed canvas MUST be
byte-identical to a full recomposite of the document, every pixel outside the
union MUST keep its previous value, and the commit MUST NOT run the
full-document composite. The commit SHALL still emit the document-changed
notification, because it records history and marks the document dirty.

#### Scenario: A content move refreshes its source and destination union [dd_content_move_region]

- **WHEN** a selection covering part of a layer is moved by `(dx, dy)` and committed
- **THEN** only the union of the selection's source and destination rectangles is composited, and every pixel outside it keeps its previous value

#### Scenario: The content move result equals a full recomposite [dd_content_move_equal]

- **WHEN** a content move has been committed through the regional path
- **THEN** `doc.composite` equals a full recomposite of the committed document
