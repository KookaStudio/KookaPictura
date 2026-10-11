# Spec Delta

## ADDED Requirements

### Requirement: Canvas pan suppresses tool overlays

While the canvas is being panned with Space or the middle mouse button, tool
overlays — including the brush-size circle — SHALL be hidden, and switching into
or out of pan SHALL repaint the canvas immediately. The pan gesture SHALL
override the active tool's cursor and pointer mode over the canvas and SHALL
apply over tool overlays.

#### Scenario: Brush ring hidden while panning [uidc_pan_hides_ring]

- **WHEN** the user holds Space (or middle-drags) over the brush-size circle
- **THEN** the ring is hidden, the hand cursor is shown, and the drag pans the canvas

### Requirement: Tool switch preserves canvas view

Switching tools SHALL preserve the canvas zoom and scroll position. The canvas
SHALL re-fit only on the first sizing after a document is loaded (or when Fit is
explicitly invoked), not on a resize caused by an options-bar height change.

#### Scenario: Switching tools keeps zoom and position [uidc_tool_switch_zoom]

- **WHEN** the user zooms and pans, then switches to another tool
- **THEN** the zoom level and canvas position are unchanged
