## ADDED Requirements

### Requirement: Solid gradient opacity stops and midpoints

A solid gradient SHALL carry colour stops and opacity stops. Each stop SHALL
carry a location and a midpoint, and each opacity stop SHALL also carry an
opacity. Colour and opacity SHALL be interpolated independently, and a stop's
midpoint SHALL bias the interpolation of the segment that starts at it so the
segment is half mixed at that fraction; a midpoint of 50 % SHALL be the straight
blend. The Gradient Editor SHALL expose opacity stops above the ramp and colour
stops below it, SHALL draw a draggable midpoint between adjacent stops, and
SHALL let a stop be added by clicking the empty part of its row and removed by
dragging it off the bar or pressing Delete. At least two stops SHALL remain in
each row.

#### Scenario: Opacity stops drive the alpha [ge_opacity_stops]

- **WHEN** a black-to-white gradient has opacity stops at 0 % opacity at the
  start and 100 % at the end
- **THEN** sampling the ramp starts fully transparent, ends fully opaque, and
  the colour channels still run black to white

#### Scenario: A midpoint moves the even mix [ge_midpoint]

- **WHEN** a black-to-white gradient's start stop has a midpoint of 25 %
- **THEN** sampling at 25 % of the ramp is mid grey and sampling at 50 % is
  lighter than mid grey

#### Scenario: Stops can be added and removed [ge_stop_editing]

- **WHEN** an opacity stop is added above the ramp and the resulting stop is
  dragged off the bar
- **THEN** the opacity list grows by one and then returns to its original size

#### Scenario: The selected stop's row is live [ge_stop_selection]

- **WHEN** an opacity stop is selected in the editor
- **THEN** Opacity, Location, Delete, and Midpoint act on it and the colour
  row's Color field is disabled
