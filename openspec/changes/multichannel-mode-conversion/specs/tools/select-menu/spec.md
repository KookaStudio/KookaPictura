## MODIFIED Requirements

### Requirement: Load Selection sources

`Select > Load Selection` SHALL open a dialog with Document, Channel, Invert, and
Operation controls. Document SHALL default to the active document and SHALL list
the active document and every other open document with equal pixel width and
height, with no `New` entry. Channel SHALL list, in order, the chosen document's
colour rows as the Channels panel lays them out (the composite row where the
mode has one, then its colour plates), every existing raster layer mask, and
every extra channel under its stored name. The composite row SHALL derive
coverage as the luminance of all its colour channels; each plate row SHALL use
the Channels panel's mode-specific derivation for that row (the colour plane for
RGB, the naive ink level for CMYK and legacy Multichannel, and the luminance for Lab). Channel-owned Multichannel SHALL have no fabricated colour rows: each stored plate is listed once as an extra channel and loads via the existing alpha/spot channel selection handling.
Invert SHALL default off. Operation SHALL offer New, Add, Subtract, and
Intersect and SHALL default to New. The default Channel SHALL be the first extra
channel when the document has one, otherwise the first listed colour row (the
composite row where the mode has one). Loading SHALL derive coverage from the
chosen source, invert it when Invert is checked, and combine it with the current
selection using the chosen operation. Loading SHALL record exactly one undo
state in the active document. Load SHALL remain available for a document that
has no extra channel and no raster layer mask. A source whose coverage length
does not equal `width * height` MUST be refused without recording. A
save-then-load round trip at the same dimensions SHALL reproduce the coverage
byte for byte.

#### Scenario: Load inverts before combining

- **WHEN** a channel holding coverage 64 is loaded with Invert checked and Add into a selection of 128
- **THEN** the resulting coverage is 191

#### Scenario: Load from another document

- **WHEN** Load Selection reads an extra channel from another open document of the same size
- **THEN** the active document's selection becomes the source coverage and exactly one state is recorded in the active document

#### Scenario: Load lists existing channels

- **WHEN** the document has saved selections and Load Selection is invoked
- **THEN** the dialog lists each extra channel by its stored name and loads the chosen one

#### Scenario: Load lists colour rows in channel order [sm_load_lists_colour]

- **WHEN** Load Selection is invoked for an RGB document with one extra channel and one raster layer mask
- **THEN** the Channel list reads RGB, Red, Green, Blue, the layer mask, then the extra channel

#### Scenario: Load loads a colour plate [sm_load_colour_plane]

- **WHEN** the Red, Green, or Blue row of a flat (100, 150, 200) image is loaded with New
- **THEN** every pixel's selection coverage is 100, 150, or 200 respectively

#### Scenario: The composite row loads its luminance [sm_load_composite]

- **WHEN** the RGB row of a flat (100, 150, 200) image is loaded with New
- **THEN** every pixel's selection coverage is 140

#### Scenario: A document with no saved channels still loads [sm_load_no_saved]

- **WHEN** Load Selection is invoked for a document with no extra channel and no raster layer mask
- **THEN** the Channel list holds that document's colour rows and OK is enabled

#### Scenario: A single-row mode lists one composite row [sm_load_single_row_mode]

- **WHEN** Load Selection is invoked for a Grayscale document with no saved channels
- **THEN** the Channel list holds one `Gray` row and loading it selects the image's luminance

#### Scenario: The default source is the first extra channel [sm_load_default]

- **WHEN** Load Selection opens for a document that has an extra channel and a raster layer mask
- **THEN** the Channel selection is the first extra channel rather than the composite row

#### Scenario: A composite-less mode defaults to its first colour row [sm_load_multichannel_default]

- **WHEN** Load Selection opens for a legacy Multichannel document with no extra channel
- **THEN** the default target is the first colour row (Cyan), `color:0`

#### Scenario: Cross-document colour row load [sm_load_colour_cross_doc]

- **WHEN** Load Selection reads a colour row from another open document of the same size
- **THEN** the active document's selection becomes that row's derived coverage and exactly one state is recorded in the active document

#### Scenario: Wrong-length channel is refused

- **WHEN** the chosen source's data length is not `width * height`
- **THEN** the command is refused and no history state is recorded

#### Scenario: Round trip is exact

- **WHEN** a selection is saved to a channel and loaded back
- **THEN** the resulting selection equals the saved selection byte for byte and one undo state is recorded

#### Scenario: Load actual authored Multichannel plates [multi_load_selection]
- **WHEN** Load Selection lists the stored channels of converted Grayscale and CMYK documents with an existing saved selection
- **THEN** each actual plate and extra channel appears once, each plate loads its stored samples through spot handling, and the saved selection remains unchanged
