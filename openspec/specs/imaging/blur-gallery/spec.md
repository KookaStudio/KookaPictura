# imaging/blur-gallery Specification

## Purpose
The Blur Gallery workspace: on-canvas pins and the Field, Iris, Tilt-Shift,
and Path blur effects.

## Requirements

### Requirement: Blur Gallery workspace and pins

`Filter ▸ Blur Gallery ▸ Field Blur…` SHALL be enabled whenever filters are
and SHALL open a Blur Gallery workspace instead of the generic filter dialog:
a full-size preview with on-canvas pins beside a Blur Tools panel and an
Effects / Motion Effects / Noise tabbed panel. The `Filter ▸ Blur Gallery`
submenu SHALL list Field Blur…, Iris Blur…, Tilt-Shift…, Path Blur…, and
Spin Blur… in that order; only Field Blur SHALL be implemented and enabled,
and the rest SHALL stay disabled stubs (Iris Blur and Tilt-Shift await
#341/#342; Path Blur and Spin Blur are post-CS6). The Blur Tools panel SHALL
list Field Blur checked and expanded with its Blur amount, Iris Blur and
Tilt-Shift as unchecked selectable rows, and Path Blur and Spin Blur as
disabled rows (see the combined-effects requirement). The workspace SHALL open on one pin at the picture's centre
with blur 0 and the Effects tab's defaults (Bokeh on, Light Bokeh 14 %,
Bokeh Color 40 %, Light Range 40..118; Noise off, Amount 0, Size 50,
Roughness 50, Color 0, Highlights 0, Gaussian). A click on the picture SHALL
add
a pin carrying the selected pin's blur; dragging a pin SHALL move it
(positions outside the picture allowed); dragging a pin's blur handle ring
SHALL change its blur; each pin's ring SHALL show its blur amount as a white
arc from the top, empty at 0 and a full ring at 100 px and above; Delete or
Backspace SHALL remove the selected pin but
never the last one; the Blur Tools panel SHALL edit the selected pin's Blur
amount in pixels (`0..=500`). Pressing `H` SHALL hide and restore the pin
overlay without changing the result. The Effects tab SHALL carry the Bokeh
checkbox, Light Bokeh and Bokeh Color fields with sliders, and a Light Range
bar with a low and a high handle and their value boxes; the Noise tab SHALL
carry the Gaussian/Uniform/Grain picker, its checkbox, and the Amount, Size,
Roughness, Color, and Highlights fields with sliders; the Motion Effects tab
SHALL state that Path/Spin motion effects are not implemented. Unchecking Bokeh or Noise
SHALL switch that effect off at render and commit time without losing the
controls' values, which stay editable. The preview
SHALL re-render as the pins, the amount, or any effect control changes, and a
Preview checkbox SHALL restore the unfiltered picture when unchecked. Cancel
SHALL close without touching the document; OK SHALL commit one `"Filter"`
history state named for the menu row.

#### Scenario: The workspace opens on CS6's default pin

- **WHEN** Field Blur is chosen from the Filter menu on an open document
- **THEN** the workspace shows one pin at the picture's centre with blur 0,
  the Field Blur row checked and editable, the Iris Blur / Tilt-Shift rows
  unchecked but selectable, the Path Blur / Spin Blur rows present but
  disabled, and the Effects, Motion
  Effects, and Noise tabs

#### Scenario: The Blur Gallery submenu lists the gallery entries

- **WHEN** the Filter menu is opened
- **THEN** a Blur Gallery submenu lists Field Blur…, Iris Blur…, Tilt-Shift…,
  Path Blur…, and Spin Blur…, with only Field Blur implemented and enabled

#### Scenario: Pins are added, moved, and removed

- **WHEN** the picture is clicked away from the existing pin
- **THEN** a second pin is added and selected; dragging it moves it; Delete
  removes it; and Delete on the only remaining pin is refused

#### Scenario: A blur amount previews live

- **WHEN** the selected pin's Blur is raised
- **THEN** the workspace preview blurs around that pin, and unchecking
  Preview shows the unfiltered picture

#### Scenario: The Effects and Noise tabs edit the preview

- **WHEN** Light Bokeh is raised on the Effects tab, or Noise is enabled with
  an amount on the Noise tab
- **THEN** the preview lifts or roughens, and unchecking Bokeh or Noise turns
  the effect off while the tab's controls stay editable

#### Scenario: Cancel restores and OK commits once

- **WHEN** the workspace is cancelled after edits, or accepted
- **THEN** Cancel leaves the document bit-identical, and OK records exactly
  one history state labelled `Filter: Field Blur`

### Requirement: Field Blur slots and Last Filter

The `field-blur` kind SHALL take a variable slot list `[count, x₀, y₀, blur₀,
…, bokeh enabled, light bokeh, bokeh color, range low, range high, noise
enabled, noise amount, noise size, noise roughness, noise color, noise
highlights, noise kind, seed]`, with an arity-table canonical count of one
pin and the 13-slot effects tail (17 slots). An empty list and the canonical
all-defaults list (count 0) SHALL both map to one centred pin with blur 0;
the empty list SHALL carry the documented effect defaults and the all-defaults
list the zeroed effects; a count of 1..=16 SHALL require exactly
`1 + 3·count + 13` slots; any other list and a count above 16 SHALL be
refused without changing the document. The workspace SHALL produce the list
with pin coordinates as fractions of the picture and blur amounts in
document pixels. `Filter ▸ Last Filter` SHALL re-apply Field Blur with the
stored slots without reopening the workspace, and `Last Filter Settings`
SHALL reopen the workspace prefilled with them.

#### Scenario: The canonical one-pin list maps

- **WHEN** `field-blur` is mapped from the canonical 17 slots
- **THEN** the filter is one centred pin with blur 0 and the zeroed effects,
  and the empty list maps to the same pin with the documented defaults

#### Scenario: A malformed pin list is refused

- **WHEN** the kind is mapped from a list whose length does not match its
  count slot plus the effects tail, from the old no-tail layout, or from more
  than 16 pins
- **THEN** the mapping returns no filter and the document is unchanged

#### Scenario: Last Filter reuses the pin list

- **WHEN** `Filter ▸ Last Filter` is chosen after a Field Blur commit, or
  `Last Filter Settings` is opened
- **THEN** Last Filter applies the same pins, amounts, and effects with one
  more history state, and Last Filter Settings prefills the workspace with
  them

### Requirement: Iris Blur workspace

`Filter ▸ Blur Gallery ▸ Iris Blur…` SHALL be enabled whenever filters are
and SHALL open the Blur Gallery workspace in Iris mode, superseding its
earlier disabled stub. The Blur Tools panel SHALL show Iris Blur checked and
expanded with its Blur amount (`0..=500 px`), Field Blur and Tilt-Shift as
unchecked selectable rows, and Path Blur and Spin Blur as disabled rows. The workspace SHALL open on one
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
  row checked, Field Blur and Tilt-Shift unchecked, and Path Blur and Spin
  Blur disabled

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

### Requirement: Path Blur workspace

`Filter ▸ Blur Gallery ▸ Path Blur…` SHALL be enabled whenever filters are
and SHALL open the Blur Gallery workspace in Path mode, superseding its
earlier disabled stub and the disabled Path Blur row; Spin Blur SHALL stay
a disabled row and menu stub. The Blur Tools panel SHALL show Path Blur
checked and expanded with a Basic Blur / Rear Sync Flash dropdown, Speed
(`0..=500 %`, default 50), Taper (`0..=100 %`, default 0), Centered Blur
(default on), End Point Speed (`0..=500 px`, enabled only while an end
point is selected), and a disabled Edit Blur Shapes; the Motion Effects tab
SHALL show Strobe Strength (`0..=100 %`, default 0) and Strobe Flashes
(`1..=100`, default 4), and the dropdown SHALL read Custom while Strobe
Strength is above 0. The workspace SHALL open on one path from the
picture's centre running right 0.08 of its width, both end speeds 0.04 of
the document width. Each path SHALL draw as the curve through its points,
a dot at each point and an arrow at its end. A click on empty picture SHALL
pin a new path's first stop point; the curve SHALL then follow the pointer
and each further click SHALL pin another stop, so several stops make a
curve; a double-click, Enter, Esc, or a click on the last stop SHALL end
the path (a path ended on its first stop runs right the opening length),
and Esc SHALL not close the workspace while a path is being placed.
Hovering empty picture or a curve SHALL show a pushpin-with-plus cursor.
Dragging a point SHALL move it; pressing the curve between points SHALL
insert a point there; a Ctrl-press SHALL move the whole path; Delete SHALL
remove the selected point while its path keeps two, else the selected path
while another remains. While Path Blur is the selected effect the Effects
(Bokeh) tab SHALL be disabled and the Motion Effects tab enabled, and the
reverse for the other effects. The Noise checkbox SHALL default on (Amount
0 adds nothing), so raising Amount alone grains the preview. The workspace
SHALL be marked as a post-CS6 addition. OK SHALL commit one history state
labelled `Filter: Path Blur`.

#### Scenario: The workspace opens on the default path

- **WHEN** Path Blur is chosen from the Filter menu on an open document
- **THEN** the workspace shows one two-point path from the centre, the Path
  Blur row checked with Speed 50 and Centered Blur on, End Point Speed
  disabled, and Spin Blur disabled

#### Scenario: Points and paths are edited on the canvas

- **WHEN** a point is dragged, the curve is pressed, three stops are
  clicked on empty picture and the last double-clicked, and Delete is
  pressed on a new path and then on a middle point
- **THEN** the point moves, a point is inserted, a three-stop curve is
  drawn, and the deletions remove the new path and then the point

#### Scenario: The tabs follow the selected effect

- **WHEN** the workspace opens in Path Blur, then Field Blur is selected
- **THEN** the Effects tab is disabled and Motion Effects current, then
  Effects is enabled and current and Motion Effects disabled

#### Scenario: Noise Amount alone grains the preview

- **WHEN** the Noise Amount is raised without touching its checkbox
- **THEN** the preview shows grain over the moving picture

#### Scenario: End Point Speed edits the selected end

- **WHEN** the path's end point is selected and End Point Speed is changed
- **THEN** that end's speed changes and the start's does not; selecting a
  middle point disables the field

#### Scenario: Strobe makes the preset Custom

- **WHEN** Strobe Strength is raised above 0, then Basic Blur is chosen
- **THEN** the dropdown reads Custom, then Basic Blur with the strength 0

#### Scenario: Path Blur commits once

- **WHEN** the workspace is accepted
- **THEN** one `Filter: Path Blur` history state is added and Last Filter
  is `path-blur`

### Requirement: Path Blur slots

The `path-blur` kind SHALL take `[count, (point count, (x, y)×point count,
start speed, end speed)×count, speed, taper, centered, rear sync, strobe
strength, strobe flashes, tail(13)]`; its canonical list SHALL be one
two-point path (27 slots). An empty list or a canonical list with count 0
SHALL map to one default path; a list whose length does not match its
counts SHALL be rejected. The dialog SHALL round-trip its paths and motion
values through this list.

#### Scenario: A path list round-trips

- **WHEN** the dialog is given two paths (two and three points) and
  non-default motion values
- **THEN** `values()` returns the same list

### Requirement: Path section of the combined slots

The `blur-gallery` kind SHALL append, after the Tilt-Shift section, the path
count, its records, and the six motion slots (51 canonical); its
all-defaults form SHALL require every count, the path count included, to be
0. Path Blur SHALL be selectable and combinable with the other effects as
they are with each other.

#### Scenario: Field and Path commit together

- **WHEN** Field Blur and Path Blur are both on and OK is pressed
- **THEN** the dialog commits `blur-gallery` with the field records, empty
  Iris and Tilt-Shift sections, the path records, and the motion slots

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
