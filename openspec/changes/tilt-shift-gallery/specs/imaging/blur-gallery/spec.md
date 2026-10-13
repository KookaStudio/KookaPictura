## ADDED Requirements

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
gallery mode the preview SHALL filter only the active layer, drawn over the
document with that layer hidden, so other layers are never blurred with it. OK SHALL commit
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

### Requirement: Tilt-Shift slots

The `tilt-shift` kind SHALL take `[count, (x, y, angle, focus above, focus
below, fade above, fade below, distortion, symmetric, blur)×count, effects
tail (13)]`, with an arity-table canonical list of one pin plus the tail (24
slots). An empty list and the canonical all-defaults list (count 0) SHALL
map to one default band; a count of 1..=16 SHALL require exactly
`1 + 10·count + 13` slots; any other list and a count above 16 SHALL be
refused. Last Filter SHALL re-apply the stored slots and Last Filter
Settings SHALL reopen the workspace prefilled with them.

#### Scenario: A variable band list maps

- **WHEN** `tilt-shift` is mapped from two 10-slot pins plus the tail
- **THEN** both bands, the bokeh, and the noise settings map in order

#### Scenario: A malformed band list is refused

- **WHEN** the kind is mapped from an Iris stride, a short list, or 17 pins
- **THEN** the mapping returns no filter

### Requirement: Combined Blur Gallery effects

The Blur Tools panel SHALL let Field Blur, Iris Blur, and Tilt-Shift be
used together. Each effect SHALL keep its own pin list for the life of the
workspace. Clicking an effect's name SHALL select it — switching it on,
giving it its default pin when it has none, expanding its controls, and
making the canvas show and edit its pins — and its name SHALL read bold
while selected. Its checkbox SHALL switch it on (selecting it) or off
(keeping its pins, leaving the selection). Its arrow SHALL expand or
collapse its controls without selecting it. An amount control SHALL edit its
own effect's selected pin. The preview and OK SHALL use every effect that is
on: exactly one commits as its own kind and history label (`field-blur`,
`iris-blur`, `tilt-shift`); two or more commit the `blur-gallery` kind,
labelled `Filter: Blur Gallery`; none commits nothing. Last Filter SHALL
re-apply `blur-gallery`, and Last Filter Settings SHALL reopen the workspace
with each stored effect on and the first of them selected. A `blur-gallery`
command spec SHALL exist for labels and Last Filter Settings without a menu
row.

#### Scenario: A row's name selects and expands its effect

- **WHEN** Iris Blur's name is clicked in a Field Blur workspace
- **THEN** Iris Blur is checked, selected, expanded with its default pin,
  the canvas edits the iris, Field Blur stays on and expanded, and the kind
  is `blur-gallery`

#### Scenario: The arrow only expands

- **WHEN** Tilt-Shift's arrow is clicked in an Iris Blur workspace, then
  Iris Blur's
- **THEN** Tilt-Shift's controls show while it stays off and Iris stays
  selected, then Iris Blur's controls collapse

#### Scenario: The checkbox switches an effect

- **WHEN** Tilt-Shift is checked in a Field Blur workspace, Field Blur
  unchecked, then checked again
- **THEN** the kind goes `blur-gallery`, then `tilt-shift` with the
  Tilt-Shift slots alone, then `blur-gallery` with Field Blur's pin kept

#### Scenario: Pins stay with their effect

- **WHEN** a second Field pin is added, Iris Blur selected, then Field Blur
  selected again
- **THEN** the canvas shows the iris alone, then both Field pins with the
  second still selected

#### Scenario: Combined effects commit and reopen

- **WHEN** Tilt-Shift is selected in an Iris Blur workspace opened from the
  menu and OK is pressed, then Last Filter Settings is chosen
- **THEN** one `Filter: Blur Gallery` state is added with the `blur-gallery`
  slots, and the workspace reopens with Iris Blur and Tilt-Shift on, Field
  Blur off, and Iris Blur selected

#### Scenario: Nothing on commits nothing

- **WHEN** the only effect is unchecked and OK is pressed
- **THEN** no history state is added

### Requirement: Blur Gallery combined slots

The `blur-gallery` kind SHALL take each effect's count and records in turn —
Field `(x, y, blur)`, Iris (8 slots), Tilt-Shift (10 slots), a count of 0
for an effect that is off — then the 13-slot effects tail, with an
arity-table canonical list of one record of each plus the tail (37 slots).
An empty list and the canonical list with every count 0 SHALL map to one
default Field pin; a count that is negative, fractional, or above 16, a
short list, or trailing slots SHALL be refused.

#### Scenario: A combined list maps

- **WHEN** `blur-gallery` is mapped from no Field pins, one iris, and two
  bands plus the tail
- **THEN** the Field list is empty, the iris and both bands map in order,
  and the tail maps

#### Scenario: A malformed combined list is refused

- **WHEN** the list is one slot short, or a count is fractional or 17
- **THEN** the mapping returns no filter

