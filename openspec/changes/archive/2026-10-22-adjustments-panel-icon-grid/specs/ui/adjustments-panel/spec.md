# Spec Delta

## Purpose

The Adjustments panel surface: an always-visible icon grid that creates adjustment and fill layers in one click and hands editing to the Properties panel, matching the CS6 panel.

## ADDED Requirements

### Requirement: Adjustments panel icon grid

The Adjustments panel SHALL display an always-visible icon grid instead of an empty placeholder. The grid SHALL contain nineteen entries in a fixed order: the three fill-layer types first (Solid Color, Gradient, Pattern), then the sixteen adjustment types in CS6 order (Brightness/Contrast, Levels, Curves, Exposure, Vibrance, Hue/Saturation, Color Balance, Black & White, Photo Filter, Channel Mixer, Color Lookup, Invert, Posterize, Threshold, Gradient Map, Selective Color). Each entry SHALL show an icon only (no text label) and SHALL carry a tooltip naming the adjustment or fill type.

#### Scenario: Grid shows all nineteen entries in order

- **WHEN** the Adjustments panel is displayed with a document open
- **THEN** the grid's entries, in order, are Solid Color, Gradient, Pattern, Brightness/Contrast, Levels, Curves, Exposure, Vibrance, Hue/Saturation, Color Balance, Black & White, Photo Filter, Channel Mixer, Color Lookup, Invert, Posterize, Threshold, Gradient Map, Selective Color, each rendered as an icon with a tooltip naming it

#### Scenario: Grid entries carry tooltips

- **WHEN** the pointer hovers a grid entry
- **THEN** a tooltip names the adjustment or fill type

### Requirement: Adjustment icon creates and selects an adjustment layer

Clicking an adjustment icon SHALL create an adjustment layer of that kind through the same path as the panel menu, select the new layer, and expose its controls in the Properties panel. Creating the layer SHALL be undoable.

#### Scenario: Clicking an adjustment icon creates the layer

- **WHEN** an adjustment icon is clicked with a document open
- **THEN** a new adjustment layer of that kind is added above the active layer, becomes the selection, and its controls appear in the Properties panel

#### Scenario: Creating an adjustment is undoable

- **WHEN** an adjustment icon is clicked and then Undo is invoked
- **THEN** the created adjustment layer is removed and the layer stack returns to its prior state

### Requirement: Panel menu preserved

The Adjustments panel menu SHALL keep its existing rows (the sixteen adjustment kinds in CS6 order, Add Mask by Default, Clip to Layer) and SHALL remain a working alternative create path. The menu rows use the existing hyphenated row texts (`Brightness-Contrast`, `Hue-Saturation`); the grid labels are the CS6 display names and need not match the row texts verbatim.

#### Scenario: Panel menu still lists the adjustment kinds

- **WHEN** the Adjustments panel menu is opened
- **THEN** it lists Brightness-Contrast, Levels, Curves, Exposure, Vibrance, Hue-Saturation, Color Balance, Black & White, Photo Filter, Channel Mixer, Color Lookup, Invert, Posterize, Threshold, Gradient Map, Selective Color, Add Mask by Default, and Clip to Layer, in that order

### Requirement: Fill icons route to the fill-layer commands

Clicking the Solid Color or Gradient icon SHALL invoke the existing New Fill Layer command for that type. The Pattern icon SHALL remain inert while the Pattern fill command is unimplemented, matching the Layer menu.

#### Scenario: Solid Color icon opens the solid-color flow

- **WHEN** the Solid Color icon is clicked
- **THEN** the Solid Color fill-layer command is dispatched

#### Scenario: Pattern icon is inert

- **WHEN** the Pattern icon is clicked
- **THEN** no layer is created and no command is dispatched

### Requirement: Grid enablement follows the document

Without an open document, the grid SHALL be disabled. With a document open, grid entries SHALL be enabled, except the inert Pattern entry which remains disabled.

#### Scenario: No document disables the grid

- **WHEN** no document is open and the Adjustments panel is displayed
- **THEN** every grid entry is disabled and clicking one does nothing

#### Scenario: A document enables the grid

- **WHEN** a document is open
- **THEN** the grid entries are enabled except the inert Pattern entry

### Requirement: Narrow panel layout

When the panel is too narrow to fit all entries on one row, the grid SHALL wrap to additional rows or scroll so every entry remains reachable; no entry SHALL be hidden.

#### Scenario: Narrow panel wraps the grid

- **WHEN** the panel width is reduced below the width of one full row
- **THEN** entries flow onto additional rows and all nineteen entries remain visible and clickable
