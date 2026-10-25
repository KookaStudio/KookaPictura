# ui/adjustments-panel Specification

## Purpose
The Adjustments panel surface: an always-visible icon grid that creates adjustment and fill layers in one click and hands editing to the Properties panel, matching the CS6 panel.

## Requirements

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

The Adjustments panel menu SHALL keep its existing rows (the sixteen adjustment
kinds in CS6 order, Add Mask by Default, Clip to Layer) and SHALL remain a
working alternative create path. The menu rows use the existing hyphenated row
texts (`Brightness-Contrast`, `Hue-Saturation`); the grid labels are the CS6
display names and need not match the row texts verbatim. **Add Mask by Default
SHALL be an enabled checkable preference** whose state reflects the session and
whose toggle governs whether a newly created adjustment layer receives a mask.
`Clip to Layer` SHALL remain a checkable row.

#### Scenario: Panel menu still lists the adjustment kinds

- **WHEN** the Adjustments panel menu is opened
- **THEN** it lists Brightness-Contrast, Levels, Curves, Exposure, Vibrance, Hue-Saturation, Color Balance, Black & White, Photo Filter, Channel Mixer, Color Lookup, Invert, Posterize, Threshold, Gradient Map, Selective Color, Add Mask by Default, and Clip to Layer, in that order

#### Scenario: Add Mask by Default is a working toggle [adj_add_mask_default_toggle]

- **WHEN** the user toggles Add Mask by Default in the Adjustments panel menu
- **THEN** the row's check state follows the preference and the next created adjustment layer honors it

### Requirement: Fill icons route to the fill-layer editors

Clicking the Solid Color, Gradient, or Pattern icon SHALL create a fill layer
of that type with the current defaults and then open its type editor, leaving
the default layer in place if the editor is cancelled. The Pattern icon SHALL be
enabled when the open document contains at least one pattern resource and
disabled with a "No patterns are loaded" reason when it contains none, matching
the Layer menu.

#### Scenario: Solid Color icon opens the solid-color flow

- **WHEN** the Solid Color icon is clicked
- **THEN** a Solid Color fill layer is created with the default content and its editor opens

#### Scenario: Pattern icon is inert

- **WHEN** the open document contains no pattern resources and the Pattern icon is clicked
- **THEN** no layer is created, no command is dispatched, and the icon is disabled with the no-patterns reason

#### Scenario: Pattern follows the document's patterns [adj_pattern_icon]

- **WHEN** the Pattern icon is shown for a document with patterns and for one without
- **THEN** it is enabled and opens the Pattern Fill editor on the first, and disabled with the no-patterns reason on the second

### Requirement: Grid enablement follows the document

Without an open document, the grid SHALL be disabled. With a document open, grid
entries SHALL be enabled, except the Pattern entry, which SHALL be enabled only
when the document contains pattern resources.

#### Scenario: No document disables the grid

- **WHEN** no document is open
- **THEN** every grid entry is disabled

#### Scenario: A document enables the grid

- **WHEN** a document is open
- **THEN** every grid entry is enabled except Pattern when the document has no pattern resources

### Requirement: Narrow panel layout

When the panel is too narrow to fit all entries on one row, the grid SHALL wrap to additional rows or scroll so every entry remains reachable; no entry SHALL be hidden.

#### Scenario: Narrow panel wraps the grid

- **WHEN** the panel width is reduced below the width of one full row
- **THEN** entries flow onto additional rows and all nineteen entries remain visible and clickable

### Requirement: Adjustment mask defaults

The system SHALL provide an `Add Mask by Default` preference, defaulting to
enabled and persisted in the session store. When it is enabled, a newly created
adjustment layer SHALL receive a layer mask: the active selection's mask when a
selection exists, otherwise a white document-sized mask. When it is disabled,
the new adjustment layer SHALL have no mask. After a successful creation with a
mask, the new layer's mask SHALL become the view's mask edit target, with mask
solo mode off, so painting and filling edit the mask immediately; without a
mask, the layer thumbnail SHALL be the active target.

#### Scenario: A selection becomes the adjustment mask [adj_mask_from_selection]

- **WHEN** Add Mask by Default is enabled, a selection is active, and an adjustment layer is created
- **THEN** the layer carries a mask built from the selection and that mask is the active target

#### Scenario: No selection creates a white adjustment mask [adj_mask_white]

- **WHEN** Add Mask by Default is enabled, no selection is active, and an adjustment layer is created
- **THEN** the layer carries a white document-sized mask and that mask is the active target, not mask solo mode

#### Scenario: Disabling the preference creates no mask [adj_mask_off]

- **WHEN** Add Mask by Default is disabled and an adjustment layer is created
- **THEN** the layer has no mask and its layer thumbnail is the active target
