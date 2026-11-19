# Spec Delta

## MODIFIED Requirements

### Requirement: Load with invert and operation

Loading a selection from an extra channel, a raster layer mask, or a colour row
SHALL derive the source coverage, replace it with `255 - coverage` when Invert
is checked, and then combine it with the current selection using the chosen
operation from the selection-model algebra. A colour row SHALL use the
Channels-panel row derivation. For `Select > Load Selection`, every colour
channel counts as visible: the composite row is the luminance of all its colour
channels, and a plate row is the Channels panel's mode-specific derivation for
that row (the colour plane for RGB, the naive ink level for CMYK and
Multichannel, and the luminance for Lab). The Channels panel's own composite row
SHALL continue to follow its eye column. With no current selection, New and Add SHALL
produce the effective source coverage, while Subtract and Intersect SHALL
produce an empty selection. The load SHALL record exactly one undo state.

#### Scenario: Invert is applied before the combine

- **WHEN** coverage 64 is loaded with Invert and Add into a selection of 128
- **THEN** the effective source is 191 and the result is `max(128, 191) = 191`

#### Scenario: No selection with each operation

- **WHEN** a channel is loaded with no current selection
- **THEN** New and Add load the effective source, while Subtract and Intersect load an empty selection

#### Scenario: Load is one state

- **WHEN** a channel is loaded with Add, Subtract, or Intersect
- **THEN** the history gains exactly one "Load Selection" state

#### Scenario: A colour plate loads its plane [sc_load_colour_plane]

- **WHEN** the Red, Green, or Blue row of a flat (100, 150, 200) image is loaded
- **THEN** the source coverage is 100, 150, or 200 per pixel respectively

#### Scenario: The composite row loads its luminance [sc_load_colour_composite]

- **WHEN** the RGB row of a flat (100, 150, 200) image is loaded
- **THEN** the source coverage is the luminance of (100, 150, 200), i.e. 140

#### Scenario: A colour row combines like any other source [sc_load_colour_combine]

- **WHEN** a selection of 200 exists and the Red row holding 100 is loaded with Subtract
- **THEN** the resulting selection coverage is 100 and one "Load Selection" state is recorded

#### Scenario: A derived-mode plate row uses the panel's naive ink [sc_load_naive_mode_rows]

- **WHEN** a CMYK or Multichannel plate row is loaded
- **THEN** the source coverage equals the naive ink derivation the Channels panel uses for that row
