## MODIFIED Requirements

### Requirement: Tilt-Shift workspace

`Filter ▸ Blur Gallery ▸ Tilt-Shift…` SHALL be enabled whenever filters are
and SHALL open the Blur Gallery workspace in Tilt-Shift mode, superseding
its earlier disabled stub. The Blur Tools panel SHALL show Tilt-Shift
checked and expanded with its Blur amount (`0..=500 px`), Distortion
(`-100..=100 %`), and Symmetric Distortion, Field Blur and Iris Blur as
unchecked selectable rows, and Path Blur and Spin Blur as disabled rows. The workspace SHALL open on one pin at
the picture's centre with a horizontal band, focus 0.1 and fade 0.15 of the
picture's height on each side, distortion 0, symmetric off, blur 15, and the
shared effects defaults. The selected pin SHALL draw its two solid focus
lines and two dashed fade lines across the picture and a rotation dot on
each solid line: dragging a solid line SHALL set that side's focus extent
while its dashed line stays put until pushed; dragging a dashed line SHALL
set that side's fade; dragging a dot SHALL turn the band toward the pointer.
Hovering a line SHALL show a double-arrow resize cursor along the band's
normal, and hovering a dot SHALL show the rotate cursor; a pin under the
pointer wins over a line. While a pin, line, dot, or the Blur or Distortion
slider is being dragged, the preview SHALL render from a reduced copy of at
most 360 px on its long edge, and releasing it SHALL render the full proxy.
A click elsewhere on the picture SHALL add a pin copying the selected pin.
Pins move, delete, hide (`H`), and preview as in Field Blur. In every
gallery mode the preview SHALL filter only the active layer and composite it
with the rest of the document as the commit will, so other layers are never
blurred with it and the layer's blend mode, Blend If and masks show. OK SHALL commit
one history state labelled `Filter: Tilt-Shift`.

#### Scenario: The workspace opens on the default band

- **WHEN** Tilt-Shift is chosen from the Filter menu on an open document
- **THEN** the workspace shows one centred band with blur 15 and distortion
  0, the Tilt-Shift row checked, Field Blur and Iris Blur unchecked, and
  Path Blur and Spin Blur disabled

#### Scenario: The lines and dots reshape the band

- **WHEN** the lower solid line is dragged outward, the upper dashed line
  outward, and the upper dot sideways
- **THEN** the lower focus grows with its dashed line fixed, the upper fade
  grows, and the band turns

#### Scenario: Hovering shows the drag cursors

- **WHEN** the pointer hovers a solid or dashed line of a horizontal band, a
  rotation dot, empty picture, and a band turned 45°
- **THEN** the cursor is the vertical resize arrow, the rotate arrow, the
  plain arrow, and the matching diagonal resize arrow

#### Scenario: A drag previews a draft

- **WHEN** the pin of a 1000 px picture is dragged and released
- **THEN** the preview is at most 360 px wide during the drag and the full
  proxy after the release

#### Scenario: The preview keeps the band sharp

- **WHEN** the amount is raised with Preview on
- **THEN** the preview's centre row matches the unfiltered picture and its
  top and bottom rows blur

#### Scenario: Only the active layer blurs

- **WHEN** a black box on its own layer over a patterned Background is
  blurred in the preview and then committed
- **THEN** Background pixels away from the box are unchanged in both, and
  the box's edge feathers

#### Scenario: OK commits once

- **WHEN** the Tilt-Shift workspace is accepted from the menu
- **THEN** exactly one history state labelled `Filter: Tilt-Shift` is added
  and Last Filter stores the `tilt-shift` slots
