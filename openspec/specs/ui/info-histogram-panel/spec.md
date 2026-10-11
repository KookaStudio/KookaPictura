# info-histogram-panel Specification

## Purpose
Info panel readouts, a histogram view, and the Info and Histogram dock, without per-refresh recompositing.

## Requirements

### Requirement: Info panel readouts

The system SHALL show, for the active document, the cursor position in image
coordinates and the colour under the cursor in the selected colour mode. The size
block SHALL report the active selection's width and height, and SHALL be blank
when nothing is selected. The readouts SHALL update as the cursor moves and the
document changes. The document-dimensions row SHALL be removed.

#### Scenario: Cursor readouts update

- **WHEN** the pointer moves over the canvas
- **THEN** the Info panel shows the image coordinates and the colour at that point

#### Scenario: Selection readout

- **WHEN** a rectangular selection exists
- **THEN** the W/H block shows the selection's width and height

#### Scenario: No selection

- **WHEN** nothing is selected
- **THEN** the W/H block is blank

#### Scenario: No document

- **WHEN** no document is open
- **THEN** the readouts show placeholders and do not crash

### Requirement: Histogram view
The system SHALL display a 256-bin histogram of the composite image with a
selector for the red, green, blue, and luminance channels, and SHALL recompute it
when the document changes.

#### Scenario: Channel selection
- **WHEN** the user selects a channel in the Histogram panel
- **THEN** the plotted distribution is for that channel

#### Scenario: Recompute on edit
- **WHEN** the document changes
- **THEN** the histogram is recomputed from the new composite

### Requirement: Info and Histogram dock and toggle
The system SHALL host the Info and Histogram panels in registered docks with
stable `objectName`s and SHALL expose `Window > Panels > Info` and
`Window > Panels > Histogram` toggles.

#### Scenario: Toggle the Histogram panel
- **WHEN** the user toggles Histogram from the Window menu
- **THEN** the panel is shown or hidden

### Requirement: Panels do not re-composite per refresh

The Info panel's pixel sampling SHALL read the document's planar composite
directly rather than building a full-resolution image. The Histogram SHALL be
computed from a view-pyramid level — a bounded reduction of the composite of at
most 512×512 — so that its cost is independent of document size, and it MUST NOT
read `PictureView::image()` or otherwise force a full-resolution image rebuild.
Both SHALL remain visually equivalent to the previous full-resolution results.

#### Scenario: Sampling a large document performs no full composite

- **WHEN** a pixel is sampled in the Info panel on a 4000×4000 document whose
  composite is already current
- **THEN** the sample is read from the cached composite and no full-document
  composite is run

#### Scenario: Histogram refresh does not scan every pixel

- **WHEN** the Histogram panel refreshes on a 4000×4000 document
- **THEN** it bins a downsample of at most 512×512 and the plotted distribution
  remains visually equivalent

#### Scenario: Histogram refresh does not read the full-resolution image

- **WHEN** the Histogram panel refreshes after a region refresh
- **THEN** it reads a view-pyramid level, does not call `PictureView::image()`,
  and no full-resolution image rebuild runs

#### Scenario: Histogram cost is independent of document size

- **WHEN** the Histogram panel refreshes on a document at least twice as large in
  each dimension
- **THEN** the work it does is bounded by the chosen pyramid level and does not
  grow with the document's pixel count

### Requirement: CMYK colour readout

The Info panel SHALL show the CMYK components of the colour under the cursor in
the top-right block by default, derived from the sampled pixel with no engine call
beyond the sample. Each component SHALL be reported as a 0–100 percentage.
Off-canvas positions SHALL blank the colour blocks.

#### Scenario: Cursor over a white pixel [info_cmyk_white]

- **WHEN** the cursor is over a white pixel of a document
- **THEN** the top-right block shows `C : 0  M : 0  Y : 0  K : 0`

#### Scenario: Cursor leaves the canvas [info_cmyk_offcanvas]

- **WHEN** the cursor moves off the canvas
- **THEN** the colour blocks are blank rather than holding their last values

### Requirement: Ruler readout mode

When the Ruler tool is active, the Info panel SHALL swap the top-right block to
the ruler's angle (A) and length (L) with a protractor glyph, and SHALL report
the W/H block from the ruler's measurement rather than the selection. When no
measuring line exists, the A/L and W/H blocks SHALL be blank. Switching away from
the Ruler SHALL restore the top-right colour block and the selection-size W/H.

#### Scenario: A measuring line fills A/L and W/H [info_ruler_measurement]

- **WHEN** the Ruler tool is active and a measuring line runs from (2, 3) to (2, 13)
- **THEN** the top-right block shows A `-90.0°` and L `10.0`, and the W/H block
  reads the ruler deltas

#### Scenario: No measuring line [info_ruler_empty]

- **WHEN** the Ruler tool is active and no measuring line has been drawn
- **THEN** the A/L and W/H blocks are blank

#### Scenario: Leaving ruler mode restores document dimensions [info_ruler_off]

- **WHEN** the active tool changes away from the Ruler
- **THEN** the top-right block is the CMYK readout again and the W/H block follows
  the selection

### Requirement: Info readout grid

The Info panel SHALL present four readout blocks in a 2×2 grid, each block an
auto-raise `QToolButton` (opening its menu on a single click) beside rows of
`Key : value`. The top-left and top-right blocks SHALL be the colour blocks; the
bottom-left SHALL report the cursor position as X/Y; the bottom-right SHALL
report the size as W/H. The grid SHALL use zero horizontal and vertical spacing.
Each block SHALL draw a single 1px separator on its right edge in grid column 0
and on its bottom edge in grid row 0, so the four blocks share one inner cross
and the grid draws no outer border — the panel pane is the outer edge. The
separator colour SHALL come from the theme's frame shade (the pane colour
darkened), never a hard-coded constant, so the lines stay darker than the pane
at every brightness level. Each colour block's bit-depth footer SHALL sit in the
content area to the right of the icon and SHALL be left- and bottom-aligned. A
1px rule in the same frame shade SHALL separate the grid from the `Doc:` line.
A block's menu SHALL open beside its button — to the right, or to the left when
the right side does not fit on the screen — and SHALL never cover the button
that opened it.

#### Scenario: The grid shows four blocks

- **WHEN** a document is open
- **THEN** the panel shows a colour block with an R/G/B readout, a colour block
  with a C/M/Y/K readout, an X/Y position block, and a W/H size block

#### Scenario: No document

- **WHEN** no document is open
- **THEN** every block's values are blank and the panel does not crash

#### Scenario: Shared table separators

- **WHEN** the panel is shown
- **THEN** the four blocks are divided by single theme-shade lines forming the
  inner cross, with no outer border, and a same-shade rule above the `Doc:` line

#### Scenario: Bit-depth footer placement

- **WHEN** a colour block shows its bit-depth footer
- **THEN** the footer is left- and bottom-aligned in the content area to the
  right of the icon

#### Scenario: Menu opens beside its button [info_menu_beside_button]

- **WHEN** a readout button's menu opens
- **THEN** the menu sits to the right of the button (or to its left when the
  right side does not fit) and does not overlap the button

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
percentages (0–400). Opacity SHALL show one `Op` row equal to the sampled
alpha as a percentage (`qAlpha / 255 × 100`). Off-canvas positions SHALL blank
the colour blocks.

#### Scenario: Switching the top-left block to CMYK [info_color_mode_cmyk]

- **WHEN** the top-left block's menu selection changes to CMYK
- **THEN** its rows become `C`,`M`,`Y`,`K` and the values are 0–100 percentages

#### Scenario: Grayscale uses the grey value [info_color_mode_gray]

- **WHEN** the top-left block is set to Grayscale over a white pixel
- **THEN** its single `K` row reads `255`

#### Scenario: Total Ink and Opacity rows

- **WHEN** the top-left block is set to Total Ink and then Opacity over an
  opaque white pixel
- **THEN** Total Ink reads `Ink : 0` and Opacity reads `Op : 100`

#### Scenario: Top-left defaults to Actual Color

- **WHEN** the panel is first constructed
- **THEN** the top-left block's mode is Actual Color and the top-right block's
  mode is CMYK

### Requirement: Measurement-unit menu

The position (X/Y) block's button SHALL open a measurement-unit menu offering
Pixels, Inches, Centimeters, Millimeters, Points, Picas, and Percent. The chosen
unit SHALL apply to both the position and size blocks; the W/H block SHALL NOT
have its own menu and SHALL inherit the X/Y unit. Conversion SHALL use 72 PPI:
inches = pixels / 72, centimeters = inches × 2.54, millimeters = inches × 25.4,
points = inches × 72, picas = inches × 6, and percent = the value against the
relevant document dimension (X and W against the width, Y and H against the
height). Non-pixel units SHALL be formatted to one or two decimals; Pixels SHALL
be whole.

#### Scenario: Milliseconds to millimetres [info_unit_mm]

- **WHEN** the position block is set to Millimeters with the cursor at 2 px
- **THEN** X and Y each read `0.71` (2 / 72 × 25.4)

#### Scenario: Each block keeps its own unit [info_unit_per_block]

- **WHEN** the X/Y block is set to Millimeters with a selection present
- **THEN** the position values convert and the W/H block is reformatted in the
  same unit

#### Scenario: W/H has no menu [info_unit_size_no_menu]

- **WHEN** the size block is inspected
- **THEN** its button carries no menu and its unit follows the position block

### Requirement: Document size line

The Info panel SHALL show a `Doc:` line reporting the document's pixel-plane
memory footprint and its on-disk file size, read from the `document_size_bytes`
bridge (`[memory, disk]`). The memory SHALL sum the composite, every layer's
channel planes (recursing groups), layer masks, and the document extra channels;
the disk size SHALL be the file's byte length, or 0 for an unsaved document.

#### Scenario: An open document shows a Doc line [info_doc_line]

- **WHEN** a document is open
- **THEN** the panel shows a non-empty `Doc:` line

### Requirement: Colour bit depth

Each colour block SHALL keep an independent bit depth of 8, 16, or 32,
selectable from the depth entries of its colour menu, and its footer SHALL read
`8-bit`, `16-bit`, or `32-bit`. Depth SHALL scale the channel values of Actual
Color, Proof Color, RGB, and Grayscale: 8-bit raw 0–255, 16-bit `v × 257`
(0–65535), and 32-bit `v / 255.0` to three decimals. HSB, CMYK, Lab, Total Ink,
and Opacity SHALL be depth-independent. `ponytail:` the engine samples 8-bit, so
the 16- and 32-bit readouts are scaled from the 8-bit sample rather than read
from a true deep-colour document.

#### Scenario: 16-bit scales an RGB channel [info_bit_depth_16]

- **WHEN** the top-left block is at 16-bit over a white pixel
- **THEN** the `R` value is `65535` and the footer reads `16-bit`

#### Scenario: Depth does not change percentages [info_bit_depth_independent]

- **WHEN** a block is set to CMYK at 16-bit
- **THEN** the C/M/Y/K values stay 0–100 percentages and only the footer changes

### Requirement: Info readout menu first-open geometry

A readout block's menu SHALL be sized from its size hint before it is shown, so
the first activation opens the menu fully at its final size rather than partially,
and later activations are unchanged. The existing beside-the-button placement
SHALL be preserved.

#### Scenario: The first click opens the menu fully [info_menu_first_open]

- **WHEN** a readout block's menu is opened for the first time
- **THEN** it is sized from its size hint and appears fully at its final size
  beside the button rather than partially open

### Requirement: Info panel tool hints

The Info panel's tool hint SHALL render each hint on its own line, with the first
letter of each hint capitalised, at a font two pixels smaller than the panel's
base font.

#### Scenario: Hints render one per line [uiih_hints_lines]

- **WHEN** a tool with several hints is active
- **THEN** each hint appears on its own line, capitalised, at the reduced font size

### Requirement: Info panel icon menus

An Info panel icon menu SHALL open fully laid out on its first show, positioned
beside its button and fully on screen, without clipping its items.

#### Scenario: First open is not clipped [uiih_menu_first_open]

- **WHEN** an icon menu button is opened for the first time
- **THEN** the whole menu is shown, laid out and unclipped

### Requirement: Histogram All Channels

The Histogram panel SHALL offer an `All Channels` entry, selected by default,
that draws the red, green, and blue channel curves as an overlaid composite.

#### Scenario: All Channels is the default [uiih_histogram_all_channels]

- **WHEN** the Histogram panel is shown
- **THEN** `All Channels` is selected and the red, green, and blue curves are drawn together

#### Scenario: A single channel can be chosen [uiih_histogram_single]

- **WHEN** a single channel is chosen
- **THEN** only that channel's curve is drawn
