# filter-gallery Specification

## Purpose
The Filter Gallery dialog's preview surface: how its zoom level is chosen and how
its thumbnail pane is shown and hidden.

## Requirements

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

### Requirement: Filter Gallery thumbnails toggle

The Filter Gallery SHALL provide a checkable toggle that shows and hides the
thumbnail pane, drawn with the Lucide `panel-right-close` icon while the pane is
shown and `panel-right-open` once it is hidden, on the same button surface as the
other icon buttons.

#### Scenario: The toggle reflects the pane [fg_thumbnails_toggle]

- **WHEN** the toggle is clicked
- **THEN** the thumbnail pane hides and the toggle shows the open icon, and a
  second click restores the pane and the close icon

### Requirement: Filter Gallery dialog

`Filter ▸ Filter Gallery…` SHALL open a dialog with a zoomable preview, the gallery filters as thumbnails grouped under collapsible category headers (Artistic, Brush Strokes, Distort, Sketch, Stylize, Texture, in CS6 order, each listing only filters with an engine kernel), a toggle that hides the thumbnails, OK and Cancel, a menu of every gallery filter, the selected effect's options, and an effect-layer list. The entry SHALL be enabled exactly when the active layer can take a filter.

#### Scenario: Categories follow CS6

- **WHEN** the gallery lists its categories
- **THEN** Artistic has 15 filters, Brush Strokes 8, Sketch 14, Stylize only Glowing Edges, and Texture 6

#### Scenario: Thumbnails show the picture filtered

- **WHEN** the gallery opens
- **THEN** each thumbnail is rendered from the picture through its filter

### Requirement: Filter Gallery effect stack

The gallery SHALL hold a stack of effects applied in order, shown top-down as last-applied first. Clicking a thumbnail or choosing from the menu SHALL make the selected effect that filter on its defaults. New effect layer SHALL insert a copy of the selected effect above it; Delete SHALL remove the selected effect but never the last one; the eye SHALL hide an effect from the preview and the commit, and SHALL be the row's check state, so Space toggles it from the keyboard and assistive technology reads it; dragging SHALL reorder. The stack SHALL preview in the dialog as it changes (see `imaging/filter-gallery-preview`), OK SHALL commit the visible effects as one "Filter Gallery" history state, and Cancel SHALL leave the layer bit-identical. The gallery SHALL reopen on the session's last stack.

#### Scenario: OK commits the stack as one state

- **WHEN** a two-effect stack is committed
- **THEN** the layer changes and the history gains exactly one state

#### Scenario: Cancel restores the layer

- **WHEN** the gallery previews a stack and is cancelled
- **THEN** the canvas is pixel-identical to before it opened

#### Scenario: Hidden effects are not applied

- **WHEN** every effect is hidden and OK is pressed
- **THEN** nothing is committed, the layer is unchanged, and no refusal is reported

#### Scenario: Space toggles the eye

- **WHEN** an effect row has focus and Space is pressed
- **THEN** the effect is hidden and its row unchecked, and a second Space shows it again
