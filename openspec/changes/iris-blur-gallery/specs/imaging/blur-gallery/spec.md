## ADDED Requirements

### Requirement: Iris Blur workspace

`Filter ▸ Blur Gallery ▸ Iris Blur…` SHALL be enabled whenever filters are
and SHALL open the Blur Gallery workspace in Iris mode, superseding its
earlier disabled stub. The Blur Tools panel SHALL show Iris Blur checked and
expanded with its Blur amount (`0..=500 px`), and Field Blur, Tilt-Shift,
Path Blur, and Spin Blur as disabled rows. The workspace SHALL open on one
pin at the picture's centre with blur 15, a horizontal semi-axis of 0.35 of
the picture's width, a vertical semi-axis scaled by the picture's aspect,
squareness 50, feather 0.65, angle 0, and the shared effects defaults. The
selected pin SHALL draw its outline with four radius handles, four feather
dots, and a roundness diamond: dragging a radius handle SHALL set that
semi-axis from the pointer's distance and rotate the outline toward it;
dragging a feather dot SHALL set the sharp core; dragging the diamond SHALL
set the squareness. A click elsewhere on the picture SHALL add a pin copying
the selected pin's outline and amount. Pins move, delete, hide (`H`), and
preview as in Field Blur. OK SHALL commit one history state labelled
`Filter: Iris Blur`.

#### Scenario: The workspace opens on the default iris

- **WHEN** Iris Blur is chosen from the Filter menu on an open document
- **THEN** the workspace shows one centred iris with blur 15, the Iris Blur
  row checked, and the other four rows disabled

#### Scenario: The handles reshape the outline

- **WHEN** a radius handle is dragged outward, a feather dot inward, the
  diamond outward, and a perpendicular handle sideways
- **THEN** that semi-axis grows, the sharp core shrinks, the squareness
  rises, and the outline turns

#### Scenario: The preview keeps the core sharp

- **WHEN** the iris amount is raised with Preview on
- **THEN** the preview's centre matches the unfiltered picture and its
  corners blur

#### Scenario: OK commits once

- **WHEN** the Iris Blur workspace is accepted from the menu
- **THEN** exactly one history state labelled `Filter: Iris Blur` is added
  and Last Filter stores the `iris-blur` slots

### Requirement: Iris Blur slots

The `iris-blur` kind SHALL take `[count, (x, y, radius x, radius y, angle,
squareness, feather, blur)×count, effects tail (13)]`, with an arity-table
canonical list of one pin plus the tail (22 slots). An empty list and the
canonical all-defaults list (count 0) SHALL map to one default iris; a count
of 1..=16 SHALL require exactly `1 + 8·count + 13` slots; any other list and
a count above 16 SHALL be refused. Last Filter SHALL re-apply the stored
slots and Last Filter Settings SHALL reopen the workspace prefilled with
them.

#### Scenario: A variable iris list maps

- **WHEN** `iris-blur` is mapped from two 8-slot pins plus the tail
- **THEN** both pins, the bokeh, and the noise settings map in order

#### Scenario: A malformed iris list is refused

- **WHEN** the kind is mapped from a Field Blur stride, a short list, or 17
  pins
- **THEN** the mapping returns no filter
