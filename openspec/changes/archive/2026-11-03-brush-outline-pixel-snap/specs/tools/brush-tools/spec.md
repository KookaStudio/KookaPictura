# Spec Delta

## MODIFIED Requirements

### Requirement: Brush-size outline overlay

While the Brush or Pencil tool is active over the canvas, the system SHALL draw a
circle outline at the pointer position. The outline represents the current brush
diameter in image space; the ring SHALL be drawn in the canvas's screen frame at
`round(diameter × zoom)` screen pixels, clamped to at least one screen pixel, with
its centre and diameter snapped to the nearest whole screen pixel so the ring is
crisp at a device pixel ratio of 1. At a higher integer device pixel ratio the
snap quantises to `devicePixelRatioF()` device pixels; at a fractional ratio the
centre can fall on a half-device pixel.
At integer zoom and offset and a device pixel ratio of 1, this coincides with the
scaled image-pixel boundary.
The outline SHALL use a one-device-pixel halo independent of the zoom. It SHALL
update as the brush size or the pointer position changes. The outline SHALL be a
drawn overlay, not an OS cursor pixmap, and SHALL be hidden when a paint tool is
not active or the pointer leaves the canvas. The outline SHALL NOT be clipped to
the document's image rectangle: it SHALL render outside the document's image
rectangle while remaining inside the canvas widget, so it stays visible when the
pointer is near or beyond the document edge.

#### Scenario: The outline tracks the brush size [lbt_outline_size]

- **WHEN** the brush size changes while a paint tool hovers the canvas
- **THEN** the drawn circle's diameter changes to match

#### Scenario: The outline scales with zoom [lbt_outline_zoom]

- **WHEN** the canvas is zoomed while a paint tool hovers the canvas
- **THEN** the circle is drawn at `round(diameter × zoom)` screen pixels, so its
  on-screen size scales with the zoom

#### Scenario: The outline edge is snapped to the screen pixel grid [lbt_outline_snap]

- **WHEN** the canvas is at a zoom level other than 100 % with the view unrotated
  and a device pixel ratio of 1, and a paint tool hovers the canvas
- **THEN** the ring's centre is on a whole screen pixel and each cardinal stroke
  is one device pixel thick; the ring's extent is `round(diameter × zoom)` screen
  pixels, within one pixel for an odd rounded diameter

#### Scenario: The halo is one device pixel independent of zoom [lbt_outline_halo]

- **WHEN** the canvas is at 50 %, 100 %, 200 %, or 400 % zoom and a paint tool
  hovers the canvas
- **THEN** the white ring's radial offset outside the black ring is one device
  pixel at every zoom

#### Scenario: A sub-pixel brush keeps a one-pixel ring [lbt_outline_subpixel]

- **WHEN** the brush diameter times the zoom is below 0.5 and a paint tool hovers
  the canvas
- **THEN** the ring is drawn at one screen pixel instead of disappearing

#### Scenario: The outline renders outside the document [lbt_outline_outside]

- **WHEN** the brush pointer is near or beyond the document edge
- **THEN** the portion of the circle outside the document bounds is still drawn,
  bounded only by the canvas widget

#### Scenario: No outline for other tools [lbt_outline_hidden]

- **WHEN** a tool other than Brush or Pencil is active, or the pointer leaves the
  canvas
- **THEN** no brush-size circle is drawn
