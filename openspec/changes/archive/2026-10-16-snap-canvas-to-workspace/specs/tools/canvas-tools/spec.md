## ADDED Requirements

### Requirement: Workspace snap clamp policy

When `view.snapCanvasToWorkspace` is checked and the scaled document fits
entirely inside the viewport on both axes, the shared range helper SHALL collapse
both axes to the single centred offset `(viewport − scaledDocument) / 2`, so the
canvas is pinned to the workspace centre and panning cannot move it. While the
scaled document overflows either axis, the helper SHALL behave exactly as if the
flag were unchecked, so the free-pan policy still governs and panning works. A
zero-size or empty document SHALL be unaffected. Every mutation point that routes
through the shared range helper — pan, zoom anchoring, centre, fit, actual
pixels, initial view, resize, and a document-size replacement — SHALL honour the
policy, and the canvas SHALL remain the single source of the offset.

#### Scenario: A fitting document is pinned to centre [lct_ws_snap_pinned]

- **WHEN** Snap Canvas To Workspace is on and the scaled document fits the
  viewport on both axes
- **THEN** the offset is the workspace centre on both axes and a pan attempt
  leaves it unchanged

#### Scenario: An overflowing document pans normally [lct_ws_snap_overflow]

- **WHEN** Snap Canvas To Workspace is on and the scaled document overflows the
  viewport on an axis
- **THEN** the canvas pans through the reveal-margin clamp exactly as it does
  with the flag off

#### Scenario: The toggle re-clamps immediately [lct_ws_snap_toggle]

- **WHEN** Snap Canvas To Workspace is turned on while a fitting document is
  panned away from centre
- **THEN** the canvas re-clamps to the centred position at once and repaints

#### Scenario: An empty document is unaffected [lct_ws_snap_empty]

- **WHEN** Snap Canvas To Workspace is on and the canvas has no document
- **THEN** the clamp keeps today's behaviour and no centring offset is produced
