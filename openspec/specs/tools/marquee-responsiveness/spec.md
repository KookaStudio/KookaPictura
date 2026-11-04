# tools/marquee-responsiveness Specification

## Purpose
Keep the Marquee responsive on large documents: a pointer move, a release and
the marching ants never rescan or copy the whole selection mask when the mask
has not changed, and tool cursors are built once.

## Requirements

### Requirement: Selection bounds and count are read once per selection

The application SHALL compute the selection's bounding box and its selected-pixel
count at most once per selection revision, and SHALL reuse them until the
selection is written. A pointer move that writes no selection MUST NOT rescan the
mask. During a selection move preview the bounds MUST still follow the drag.

#### Scenario: A hover reuses the bounds [mq_bounds_once]

- **WHEN** a rectangle is selected, the Info panel refreshes, and the pointer
  then moves over the canvas several times
- **THEN** the bounds are computed once, the W/H readout keeps the rectangle's
  size, and a new selection is computed again and read out

#### Scenario: The caches follow the selection [mq_caches_follow_revision]

- **WHEN** the bounds and count are read twice, the selection is replaced, a
  selection move preview offsets it, and the selection is cleared
- **THEN** the second read does not recompute, the replacement reports its own
  bounds and count, the preview's bounds are offset by the drag, and the
  cleared selection reports none

#### Scenario: The Marquee stays responsive on a large document [mq_large_manual]

- **WHEN** on a real display with a 16507×16196 document, the Marquee is chosen,
  a rectangle is dragged and released, and the pointer hovers inside and outside
  it, with PICTURA_PAINT_TIMING=1
- **THEN** the switch, release and hover show no full-mask stall, and the ants
  and Info W/H match the rectangle

### Requirement: The outline is traced within the selection's bounds

The committed outline SHALL be traced only over the selection's bounding box and
MUST equal the outline traced over the whole mask.

#### Scenario: Windowed and whole outlines agree [mq_contour_window]

- **WHEN** random masks, a mask touching the canvas edges, and Select All are
  traced over their bounds and over the whole canvas
- **THEN** both traces draw the same edges

### Requirement: Shape masks are unchanged

Rectangle and ellipse selections SHALL equal their per-pixel definitions,
clipped to the canvas, however they are built.

#### Scenario: Rectangles and ellipses match the per-pixel shapes [mq_shape_masks]

- **WHEN** rectangles and ellipses inside, across and outside the canvas edges,
  and with no area, are built
- **THEN** every pixel is 255 when the shape covers it and 0 otherwise

### Requirement: A cursor is built once

The application SHALL build a tool cursor once per asset, hotspot, angle and
device pixel ratio, and SHALL return the same cursor for later requests.

#### Scenario: A repeated request reuses the cursor [mq_cursor_cached]

- **WHEN** the same cursor is requested twice, and once with another hotspot
- **THEN** the two requests share one pixmap, and the other hotspot is its own
  cursor
