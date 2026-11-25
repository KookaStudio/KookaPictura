# ui/canvas-rendering-indicator Specification

## Purpose
Tells the user the canvas is still rendering while the display tiles it asked
for are being built, so a slow redraw does not look like a frozen window.

## Requirements

### Requirement: The canvas shows that it is rendering

The document canvas SHALL show an animated "Rendering…" indicator in its
top-right corner once the display tiles its view asked for have been pending
for 250 ms without a break, whatever made them pending: a history restore, a
zoom, a pan, or an edit. The indicator SHALL hide as soon as no display tile is
pending. Pending tiles that land within 250 ms SHALL show no indicator. The
indicator SHALL NOT take input focus or mouse events from the canvas.

#### Scenario: A slow redraw shows the indicator [cri_shows]

- **WHEN** the canvas paints while display tiles are pending and they stay
  pending for more than 250 ms
- **THEN** the indicator is visible and animating

#### Scenario: The indicator hides when the tiles land [cri_hides]

- **WHEN** the indicator is visible and the pending tiles land
- **THEN** the indicator hides

#### Scenario: A fast redraw shows nothing [cri_fast]

- **WHEN** display tiles are pending and land within 250 ms
- **THEN** the indicator never becomes visible

#### Scenario: The indicator passes the mouse through [cri_transparent]

- **WHEN** the indicator is visible and the user presses the mouse over it
- **THEN** the canvas receives the press

### Requirement: The indicator costs nothing while idle

While no display tile is pending and the indicator is hidden, the indicator
SHALL run no timer.

#### Scenario: No timer while idle [cri_idle]

- **WHEN** the indicator has hidden and the canvas is idle
- **THEN** none of the indicator's timers is active

### Requirement: The view reports pending display tiles

The view SHALL report display tiles as pending while a worker is building tiles
or tiles are queued for one, and SHALL report none pending once every requested
tile has landed or the document is gone.

#### Scenario: Requested tiles are pending until they land [cri_pending]

- **WHEN** tiles of a document large enough to build them on a worker are
  requested, then the worker lands them
- **THEN** tiles read as pending after the request and as not pending after
  the landing
