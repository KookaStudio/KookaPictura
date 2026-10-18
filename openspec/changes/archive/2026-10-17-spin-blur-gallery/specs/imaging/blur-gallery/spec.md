## ADDED Requirements

### Requirement: Spin Blur workspace

`Filter ▸ Blur Gallery ▸ Spin Blur…` SHALL be enabled whenever filters are
and SHALL open the Blur Gallery workspace in Spin mode, superseding its
earlier disabled stub and every earlier requirement's disabled Spin Blur
row. The Blur Tools panel SHALL show Spin Blur checked and expanded with
Blur Angle (`0..=360°`, default 15); the Motion Effects tab SHALL show
Strobe Strength and Strobe Flashes (shared with Path Blur) and, while Spin
Blur is the selected effect, Strobe Flash Duration (`0..=360°`, default 1).
The workspace SHALL open on one circular spin centred on the picture,
radius 0.13 of its width, feather 0.5. The selected spin SHALL draw its
ellipse with four radius handles (dragging one resizes that semi-axis and
turns the ellipse) and four feather dots (dragging one sets the inner
ring); every spin SHALL draw a marker at its rotation centre with a ring
whose arc shows its Blur Angle. Dragging the marker SHALL move the spin,
Alt-dragging it SHALL move only the rotation centre, and dragging the ring
SHALL change the Blur Angle. A click on empty picture SHALL add a spin
copying the selected one's ellipse and angle; Delete SHALL remove the
selected spin while another remains. While Spin Blur is the selected effect
the Effects (Bokeh) tab SHALL be disabled and the Motion Effects tab
enabled. The workspace SHALL be marked as a post-CS6 addition. OK SHALL
commit one history state labelled `Filter: Spin Blur`.

#### Scenario: The workspace opens on the default spin

- **WHEN** Spin Blur is chosen from the Filter menu on an open document
- **THEN** the workspace shows one centred circular spin, the Spin Blur row
  checked with Blur Angle 15, the Motion Effects tab current with Strobe
  Flash Duration 1 shown, and the Effects tab disabled

#### Scenario: Spins are edited on the canvas

- **WHEN** a radius handle and a feather dot are dragged, the marker is
  Alt-dragged, then dragged, the ring is dragged, the picture is clicked,
  and Delete is pressed
- **THEN** the semi-axis and feather change, the rotation centre moves
  alone, then the spin moves, the Blur Angle changes, a second spin is
  added, and the deletion removes it

#### Scenario: The duration shows only for Spin Blur

- **WHEN** the workspace opens in Spin Blur, then Path Blur is selected
- **THEN** Strobe Flash Duration is shown, then hidden, with Strobe
  Strength and Strobe Flashes shown in both

#### Scenario: The preview spins the picture

- **WHEN** the Blur Angle is raised with Preview on
- **THEN** the preview changes inside the spin and not at the picture's
  corners

#### Scenario: Spin Blur commits once

- **WHEN** the workspace is accepted
- **THEN** one `Filter: Spin Blur` history state is added and Last Filter
  is `spin-blur`

### Requirement: Spin Blur slots

The `spin-blur` kind SHALL take `[count, (x, y, radius x, radius y, angle,
feather, blur angle, pivot x, pivot y)×count, strobe strength, strobe
flashes, flash duration, tail(13)]`; its canonical list SHALL be one spin
(26 slots). An empty list or a canonical list with count 0 SHALL map to one
default spin; a list whose length does not match its count SHALL be
rejected. The dialog SHALL round-trip its spins and strobe values through
this list, the Blur Angle unscaled.

#### Scenario: A spin list round-trips

- **WHEN** the dialog is given two spins and non-default strobe values
- **THEN** `values()` returns the same list

### Requirement: Spin section of the combined slots

The `blur-gallery` kind SHALL append the spin count and records after the
path records, then the six motion slots, then the flash duration (62
canonical); its all-defaults form SHALL require every count, the spin count
included, to be 0, and the spin's strobe SHALL take the motion section's
strength and flashes. Spin Blur SHALL be selectable and combinable with the
other effects as they are with each other.

#### Scenario: Field and Spin commit together

- **WHEN** Field Blur and Spin Blur are both on and OK is pressed
- **THEN** the dialog commits `blur-gallery` with the field records, empty
  Iris, Tilt-Shift, and Path sections, the spin records, the motion slots,
  and the flash duration, and reopening it round-trips the list
