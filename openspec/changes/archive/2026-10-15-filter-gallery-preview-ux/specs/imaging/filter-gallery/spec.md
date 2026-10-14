# Spec Delta

## Purpose

The Filter Gallery dialog's preview surface: how its zoom level is chosen and how
its thumbnail pane is shown and hidden.

## ADDED Requirements

### Requirement: Filter Gallery zoom control

The Filter Gallery SHALL show its zoom level in an editable drop-down that
displays the current percentage and lists CS6's preset levels (6 %, 12 %, 25 %,
33 %, 50 %, 66 %, 100 %, 200 %, 300 %, 400 %, 600 %, 800 %, 1000 %, 1200 %,
1600 %) grouped by separators, followed by Actual Pixels, Fit in View, and Fit on
Screen. Choosing an entry SHALL set the zoom; typing a percentage SHALL set the
zoom; the −/+ buttons, the zoom shortcuts, and a mouse wheel over the preview
SHALL step one level and keep the drop-down in sync. The committed result SHALL
be unaffected by the zoom.

#### Scenario: Preset sets the zoom [fg_zoom_preset]

- **WHEN** the user picks 200 % from the zoom drop-down
- **THEN** the preview zooms to 200 % and the drop-down shows "200%"

#### Scenario: The wheel zooms [fg_zoom_wheel]

- **WHEN** the mouse wheel turns up over the preview
- **THEN** the zoom steps to the next level up

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
