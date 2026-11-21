# Info and Histogram Panel Spec Delta

## ADDED Requirements

### Requirement: Info numeric readouts use the system numeric locale

The Info panel SHALL render numeric values using the system numeric locale
independently of the UI language. It SHALL preserve the established precision
and units for measurements, colour channels, and document sizes and SHALL omit
digit grouping in the compact readouts. The Info custom-scale readout SHALL
format its typed numeric scale values with the system numeric locale while
preserving their units and canonical stored values.

#### Scenario: Fractional colour and file size readouts use the locale [loc_info_numbers]

- **WHEN** the Info panel displays a fractional 32-bit colour channel value
  and a fractional document size under a comma-decimal numeric locale
- **THEN** both display a comma decimal while keeping their previous precision
  and size units

#### Scenario: Custom scale uses the numeric locale [loc_info_scale]

- **WHEN** the Info panel displays a fractional custom scale under a
  comma-decimal locale
- **THEN** its scale values use a comma decimal with the existing units and
  precision, and the stored scale remains unchanged

## MODIFIED Requirements

### Requirement: Ruler readout mode

When the Ruler tool is active, the Info panel SHALL swap the top-right block to
the ruler's angle (A) and length (L) with a protractor glyph, and SHALL report
the W/H block from the ruler's measurement rather than the selection. When no
measuring line exists, the A/L and W/H blocks SHALL be blank. Switching away from
the Ruler SHALL restore the top-right colour block and the selection-size W/H.
The angle and length SHALL use the system numeric locale.

#### Scenario: A measuring line fills A/L and W/H [info_ruler_measurement]

- **WHEN** the Ruler tool is active, a measuring line runs from (2, 3) to
  (2, 13), and the numeric locale uses a decimal period
- **THEN** the top-right block shows A `-90.0°` and L `10.0`, and the W/H block
  reads the ruler deltas

#### Scenario: No measuring line [info_ruler_empty]

- **WHEN** the Ruler tool is active and no measuring line has been drawn
- **THEN** the A/L and W/H blocks are blank

#### Scenario: Leaving ruler mode restores document dimensions [info_ruler_off]

- **WHEN** the active tool changes away from the Ruler
- **THEN** the top-right block is the CMYK readout again and the W/H block follows
  the selection

#### Scenario: Ruler angle and length use the numeric locale [loc_info_ruler]

- **WHEN** a measuring line runs from (2, 3) to (2, 13) under a comma-decimal
  numeric locale
- **THEN** the top-right block shows A `-90,0°` and L `10,0`, and W/H still
  represents the same ruler deltas

### Requirement: Measurement-unit menu

The position (X/Y) block's button SHALL open a measurement-unit menu offering
Pixels, Inches, Centimeters, Millimeters, Points, Picas, and Percent. The chosen
unit SHALL apply to both the position and size blocks. The W/H block SHALL NOT
have its own menu and SHALL inherit the X/Y unit. Conversion SHALL use the
active document's resolution (`document_ppi`, 72 when the document records
none): inches = pixels / ppi, centimeters = inches × 2.54, millimeters =
inches × 25.4, points = inches × 72, picas = inches × 6, and percent = the value
against the relevant document dimension (X and W against the width, Y and H
against the height). Non-pixel units SHALL be formatted to one or two decimals
using the system numeric locale, independent of the interface language. Pixels
SHALL be whole.

#### Scenario: Milliseconds to millimetres [info_unit_mm]

- **WHEN** the position block is set to Millimeters with the cursor at 2 px on
  a 72 ppi document and the numeric locale uses a decimal period
- **THEN** X and Y each read `0.71` (2 / 72 × 25.4)

#### Scenario: Each block keeps its own unit [info_unit_per_block]

- **WHEN** the X/Y block is set to Millimeters with a selection present
- **THEN** the position values convert and the W/H block is reformatted in the
  same unit

#### Scenario: W/H has no menu [info_unit_size_no_menu]

- **WHEN** the size block is inspected
- **THEN** its button carries no menu and its unit follows the position block

#### Scenario: Physical units use the document resolution

- **WHEN** a 300 ppi document shows the position block in Inches with the
  cursor at 300 px and the numeric locale uses a decimal period
- **THEN** X reads `1.00`

#### Scenario: Info measurements use the numeric locale [loc_info_values]

- **WHEN** the cursor is at 2 px on each axis of a 72 ppi document with an
  active selection, the position block uses Millimeters, and the numeric
  locale uses a decimal comma
- **THEN** X and Y each show `0,71`, and W/H use the same unit and decimal
  separator for the selection's dimensions
