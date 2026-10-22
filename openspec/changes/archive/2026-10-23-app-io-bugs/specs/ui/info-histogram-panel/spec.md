# Spec Delta: ui/info-histogram-panel

## MODIFIED Requirements

### Requirement: Measurement-unit menu

The position (X/Y) block's button SHALL open a measurement-unit menu offering
Pixels, Inches, Centimeters, Millimeters, Points, Picas, and Percent. The chosen
unit SHALL apply to both the position and size blocks; the W/H block SHALL NOT
have its own menu and SHALL inherit the X/Y unit. Conversion SHALL use the
active document's resolution (`document_ppi`, 72 when the document records
none): inches = pixels / ppi, centimeters = inches × 2.54, millimeters =
inches × 25.4, points = inches × 72, picas = inches × 6, and percent = the value
against the relevant document dimension (X and W against the width, Y and H
against the height). Non-pixel units SHALL be formatted to one or two decimals;
Pixels SHALL be whole.

#### Scenario: Milliseconds to millimetres [info_unit_mm]

- **WHEN** the position block is set to Millimeters with the cursor at 2 px on a 72 ppi document
- **THEN** X and Y each read `0.71` (2 / 72 × 25.4)

#### Scenario: Each block keeps its own unit [info_unit_per_block]

- **WHEN** the X/Y block is set to Millimeters with a selection present
- **THEN** the position values convert and the W/H block is reformatted in the
  same unit

#### Scenario: W/H has no menu [info_unit_size_no_menu]

- **WHEN** the size block is inspected
- **THEN** its button carries no menu and its unit follows the position block

#### Scenario: Physical units use the document resolution

- **WHEN** a 300 ppi document shows the position block in Inches with the cursor at 300 px
- **THEN** X reads `1.00`

### Requirement: Colour-mode menu

Each colour block's button SHALL open a menu listing, in order, Actual Color,
Proof Color, a separator, Grayscale, RGB, HSB, CMYK, Lab, a separator, Total
Ink, Opacity, a separator, and the 8-bit, 16-bit, and 32-bit depth entries. The
colour-mode entries and the three depth entries SHALL be checkable, and their
checked state SHALL reflect the block's current mode and depth when the menu
opens. The top-left block SHALL default to Actual Color, the top-right block to
CMYK, both at 8-bit, and the two blocks' modes and depths SHALL be independent.

Selecting a mode SHALL rebuild that block's key rows and format its values.
Actual Color and Proof Color SHALL show R, G, B; Proof Color SHALL read the same
sRGB values as Actual Color. Grayscale SHALL show one `K` row (`qGray`). RGB
SHALL show R, G, B. HSB SHALL show `H` (0–360), `S`, `B` (0–100%). CMYK SHALL
show `C`,`M`,`Y`,`K` (0–100%). Lab SHALL show `L` (0–100), `a`,`b` (rounded
integers). Total Ink SHALL show one `Ink` row equal to the sum of the CMYK
percentages (0–400). Opacity SHALL show one `Op` row equal to the active
layer's opacity as a percentage (`opacity / 255 × 100`). It SHALL blank the row
when the Background layer is active or when no layer is active; the
sampled composite alpha SHALL NOT drive it. Off-canvas positions SHALL blank
the colour blocks.

#### Scenario: Switching the top-left block to CMYK [info_color_mode_cmyk]

- **WHEN** the top-left block's menu selection changes to CMYK
- **THEN** its rows become `C`,`M`,`Y`,`K` and the values are 0–100 percentages

#### Scenario: Grayscale uses the grey value [info_color_mode_gray]

- **WHEN** the top-left block is set to Grayscale over a white pixel
- **THEN** its single `K` row reads `255`

#### Scenario: Total Ink and Opacity rows

- **WHEN** the top-left block is set to Total Ink and then Opacity over an
  opaque white pixel with a fully opaque active layer
- **THEN** Total Ink reads `Ink : 0` and Opacity reads `Op : 100`

#### Scenario: Top-left defaults to Actual Color

- **WHEN** the panel is first constructed
- **THEN** the top-left block's mode is Actual Color and the top-right block's
  mode is CMYK

#### Scenario: Opacity reads the active layer

- **WHEN** a 50%-opacity pixel layer is active over an opaque pixel and the top-left block is set to Opacity
- **THEN** it reads `Op : 50`, independent of the sampled alpha

#### Scenario: The Background has no opacity readout

- **WHEN** the Background layer is active and the top-left block is set to Opacity
- **THEN** the `Op` row is blank
