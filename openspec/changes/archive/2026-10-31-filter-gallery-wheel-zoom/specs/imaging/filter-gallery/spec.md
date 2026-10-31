## MODIFIED Requirements

### Requirement: Filter Gallery zoom control

The Filter Gallery SHALL show its zoom level in an editable drop-down that
displays the current percentage and lists CS6's preset levels (6 %, 12 %, 25 %,
33 %, 50 %, 66 %, 100 %, 200 %, 300 %, 400 %, 600 %, 800 %, 1000 %, 1200 %,
1600 %) grouped by separators, followed by Actual Pixels, Fit in View, and Fit
on Screen. Choosing an entry SHALL set the zoom; typing a percentage SHALL set
the zoom; the −/+ buttons, the zoom shortcuts, and a mouse wheel over the
preview SHALL step one level and keep the drop-down in sync. A wheel step SHALL
keep the document point under the pointer under the pointer when zooming in, and
SHALL keep the document point at the viewport centre at the centre when zooming
out. The −/+ buttons, the zoom shortcuts, the drop-down, and a typed percentage
SHALL keep the document point at the viewport centre at the centre. An anchored
point SHALL be preserved to within one screen pixel, except where the scroll
range ends before it can be, where the scroll SHALL clamp to its range. The
committed result SHALL be unaffected by the zoom.

#### Scenario: Preset sets the zoom [fg_zoom_preset]

- **WHEN** the user picks 200 % from the zoom drop-down
- **THEN** the preview zooms to 200 % and the drop-down shows "200%"

#### Scenario: The wheel zooms [fg_zoom_wheel]

- **WHEN** the mouse wheel turns up over the preview
- **THEN** the zoom steps to the next level up

#### Scenario: The wheel zooms at the pointer [fg_zoom_wheel_anchor]

- **WHEN** the preview is panned and the mouse wheel turns up over a detail away
  from the viewport centre
- **THEN** the zoom steps up and the document point under the pointer stays
  under the pointer, within one screen pixel

#### Scenario: Zooming out keeps the centre [fg_zoom_zoomout_centre]

- **WHEN** the mouse wheel turns down over the preview
- **THEN** the zoom steps down and the document point at the viewport centre
  stays at the centre, within one screen pixel

#### Scenario: The buttons and the drop-down anchor at the centre [fg_zoom_centre_anchor]

- **WHEN** the preview is panned and the −/+ buttons, the zoom shortcuts, the
  drop-down, or a typed percentage change the zoom
- **THEN** the document point at the viewport centre stays at the centre,
  within one screen pixel

#### Scenario: Fit and Actual Pixels [fg_zoom_named]

- **WHEN** Actual Pixels is chosen
- **THEN** the zoom is 100 %; when Fit in View is chosen the whole picture fits
  the pane
