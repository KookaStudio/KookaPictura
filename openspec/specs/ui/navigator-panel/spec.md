# navigator-panel Specification

## Purpose
The Navigator thumbnail and proxy, its zoom controls, and the Navigator dock and toggle.

## Requirements

### Requirement: Navigator thumbnail and proxy
The system SHALL show a thumbnail of the active document and a rectangle marking
the portion currently visible in the canvas. The proxy SHALL reflect the canvas
zoom and pan and SHALL update when either changes.

#### Scenario: Proxy reflects the view
- **WHEN** the canvas is zoomed or panned
- **THEN** the Navigator's proxy rectangle changes to match the visible region

#### Scenario: No document
- **WHEN** no document is open
- **THEN** the Navigator shows an empty thumbnail and does not crash

### Requirement: Navigator zoom controls
The system SHALL provide a zoom slider spanning 0.01x to 32x, a Fit control, and
a 100% control that drive the canvas magnification. The zoom slider SHALL use a
tracking slider so pressing on the groove and dragging changes the zoom
continuously, rather than page-stepping or jumping to the clicked position and
stopping there.

#### Scenario: Slider sets zoom
- **WHEN** the user moves the Navigator zoom slider
- **THEN** the canvas magnification follows

#### Scenario: Dragging the slider tracks continuously [lnav_slider_track]
- **WHEN** the user presses the Navigator zoom slider's groove and drags without
  releasing
- **THEN** the canvas magnification follows the pointer continuously for the
  whole drag

#### Scenario: Fit and 100%
- **WHEN** the user activates Fit or 100%
- **THEN** the canvas fits the document or returns to actual pixels respectively

### Requirement: Navigator dock and toggle
The system SHALL host the Navigator in a registered dock with a stable
`objectName` and SHALL expose a `Window > Panels > Navigator` toggle.

#### Scenario: Toggle the Navigator
- **WHEN** the user toggles Navigator from the Window menu
- **THEN** the panel is shown or hidden

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
