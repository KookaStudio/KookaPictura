## ADDED Requirements

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
