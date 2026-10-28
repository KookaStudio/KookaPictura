# Spec Delta

## ADDED Requirements

### Requirement: Revision-keyed committed outline

The committed selection outline SHALL be cached against a selection revision
that changes whenever the selection is written. An edit that does not write the
selection MUST NOT retrace the outline, and viewing that unchanged outline MUST
NOT repaint the canvas. A selection write with a different mask MUST invalidate
the cached outline and retrace it from the new mask.

#### Scenario: An edit that does not write the selection keeps the outline [contour_cache_hit]

- **WHEN** a notifying edit commits and does not write the selection
- **THEN** the committed outline is not retraced and the canvas is not repainted
  because of the outline

#### Scenario: A paint stroke commit does not write the selection [contour_stroke_commit]

- **WHEN** a paint stroke commits without writing the selection
- **THEN** no committed outline is retraced

#### Scenario: A selection write with a different mask retraces the outline [contour_invalidate]

- **WHEN** the selection is written with a different mask
- **THEN** the committed outline is retraced from the new mask and shown on every
  canvas of the document

### Requirement: Bounded ants repaint

The marching-ants animation SHALL repaint only the canvas region covered by the
animated outline or live preview, expanded to cover the cosmetic stroke and
clipped to the widget. It MUST NOT repaint outside that clipped region. When the
animated outline spans the canvas, the whole widget may be repainted.

#### Scenario: The ants tick repaints only the outline region [ants_partial_region]

- **WHEN** the marching-ants animation advances on a canvas that shows a
  selection, live preview, or Move-tool outline
- **THEN** the repainted widget region covers the animated outline's bounds plus
  the stroke and, for a selection that does not span the canvas, is smaller than
  the full canvas
