## ADDED Requirements

### Requirement: Free pan clamp policy

The canvas pan clamp SHALL be per axis. On each axis, when `view.freePan` is
checked (the default), the shared range helper SHALL behave as today: the reveal
margin, so at least 96 logical pixels of the document stay visible. When
`view.freePan` is unchecked, on each axis independently, if the scaled document
is not larger than the viewport the allowed offset range SHALL collapse to the
single centred offset `(viewport − scaledDocument) / 2`; if the scaled document
overflows the viewport on that axis the reveal-margin clamp SHALL be unchanged.
A zero-size or empty document SHALL keep today's behaviour (the collapse SHALL
NOT fire). Every mutation point that routes through the shared range helper —
pan, zoom anchoring, centre, fit, actual pixels, initial view, resize, and a
document-size replacement — SHALL honour the same policy.

#### Scenario: A fitting axis is constrained [lct_free_pan_constrained]

- **WHEN** Free Pan is off and the document fits the viewport along an axis
- **THEN** panning toward the backdrop leaves the offset at the centred position
  on that axis

#### Scenario: Mixed axes constrain only the fitting axis [lct_free_pan_mixed]

- **WHEN** Free Pan is off and the document fits one axis but overflows the
  other
- **THEN** the fitting axis is pinned to centre and the overflowing axis pans
  within the reveal margin

#### Scenario: Zoomed-in pan is unaffected [lct_free_pan_zoomed]

- **WHEN** Free Pan is off and the document overflows the viewport on both axes
- **THEN** panning uses the reveal-margin clamp exactly as it does with Free Pan
  on

#### Scenario: Toggling re-clamps immediately [lct_free_pan_toggle]

- **WHEN** Free Pan is unchecked while a fitting document is panned away from
  centre
- **THEN** the canvas re-clamps to the centred position at once and repaints

#### Scenario: A document resized to fit is re-clamped [lct_free_pan_resize_doc]

- **WHEN** Free Pan is off and a document that overflowed the viewport is reduced
  to fit by an image or canvas resize
- **THEN** the canvas re-clamps to the centred position rather than keeping its
  old off-centre offset

#### Scenario: An empty document is unaffected [lct_free_pan_empty]

- **WHEN** Free Pan is off and the canvas has no document
- **THEN** the clamp keeps today's behaviour and no centring offset is produced
