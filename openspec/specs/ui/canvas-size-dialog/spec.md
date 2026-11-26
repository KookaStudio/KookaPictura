# ui/canvas-size-dialog Specification

## Purpose
TBD - created by archiving change canvas-size-units-and-fill. Update Purpose after archive.

## Requirements

### Requirement: Canvas Size supports picas and columns

Canvas Size SHALL offer Picas and Columns for Width and Height in absolute
and Relative mode. Picas SHALL be 12 points at the selected point definition.
Column spans SHALL use the saved width and gutter. In absolute mode the first
column uses the width, each additional column uses width plus gutter, and a
fraction below one scales the width. In Relative mode column deltas SHALL be
signed multiples of width plus gutter. The dialog SHALL convert the displayed
quantity to pixels using document resolution and retain the pixel result
when switching from Pixels into Picas or Columns.

#### Scenario: New unit choices use document resolution [cs_units_picas]

- **GIVEN** a 600 by 300 pixel document at 300 pixels per inch with Traditional points
- **WHEN** the user changes both unit menus to Picas and then enters six picas for width
- **THEN** switching units preserves the opening dimensions and the edited width resolves to 299 pixels

#### Scenario: Absolute columns use the saved gutter [cs_units_columns]

- **GIVEN** a 600 by 300 pixel document at 300 pixels per inch, a one-inch column width and a quarter-inch gutter
- **WHEN** the user selects Columns and enters two columns for width and half a column for height
- **THEN** the width resolves to 675 pixels and height to 150 pixels

#### Scenario: Relative columns measure from the current canvas [cs_units_relative]

- **GIVEN** a 600 by 500 pixel document at 300 pixels per inch, a one-inch column width and a quarter-inch gutter
- **WHEN** the user selects Columns for both axes and enables Relative
- **THEN** both fields start at zero and the dimensions stay 600 by 500
- **WHEN** the user enters one column for width and minus one for height
- **THEN** the new dimensions resolve to 975 by 125 pixels

#### Scenario: Large physical-unit restatements preserve pixels [cs_units_large_restatement]

- **GIVEN** a document whose pixel width requires more than 300000 Picas at its resolution
- **WHEN** the user switches width from Pixels to Picas and back
- **THEN** the pixel width remains unchanged

### Requirement: Canvas extension color requires a Background layer

Canvas Size SHALL enable the extension-color menu and swatch only when the
original document contains a top-level, non-group Background layer. The resize
transaction SHALL use that original state to decide whether to apply extension
fill, including when invoked without the dialog. Without an original Background,
uncovered added canvas SHALL be transparent, and existing layer content outside
the old canvas SHALL remain available and visible when exposed. A merged-only
document SHALL retain its materialized pixels without filling the added canvas
merely because resize materialized its composite as a Background.

#### Scenario: An actual Background enables both controls [cs_fill_background]

- **GIVEN** an opaque image opened as a Background
- **WHEN** the user opens Canvas Size, selects Foreground as the fill, and grows the canvas
- **THEN** the extension-color menu and swatch are enabled, and the new border uses the foreground color

#### Scenario: An ordinary opaque layer does not enable fill [cs_fill_no_background]

- **GIVEN** a document with a fully opaque ordinary layer and no Background
- **WHEN** the user opens Canvas Size and grows the canvas
- **THEN** both extension-color controls are disabled and the new border is transparent

#### Scenario: A non-pixel Background uses the same fill predicate [cs_fill_nonpixel_background]

- **GIVEN** a top-level non-group layer carrying the Background flag even though its kind is not Pixel
- **WHEN** Canvas Size checks whether extension fill is available
- **THEN** the dialog and resize transaction both treat the layer as a Background

#### Scenario: Merged-only pixels do not authorize extension fill [cs_fill_merged_only]

- **GIVEN** a document with no layers and a matching merged composite
- **WHEN** Canvas Size grows it with a supplied extension color
- **THEN** the original merged pixels remain visible and uncovered added canvas is transparent

#### Scenario: Growth reveals retained off-canvas layer pixels [cs_fill_retained]

- **GIVEN** a document without a Background and an ordinary layer extending beyond the canvas
- **WHEN** Canvas Size grows the canvas to expose part of that layer
- **THEN** the exposed layer pixels remain visible and the uncovered added canvas is transparent
