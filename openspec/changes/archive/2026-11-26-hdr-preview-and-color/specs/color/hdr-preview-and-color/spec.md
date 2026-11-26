## ADDED Requirements

### Requirement: 32-bit canvas preview controls

A 32-bit RGB or Grayscale document SHALL expose a view preview with Exposure And Gamma and Highlight Compression. The view status bar SHALL offer a resettable exposure offset. Both controls SHALL affect display without changing scene pixels.

#### Scenario: Preview or cancel a change [preview-options]

- **WHEN** a user changes the method or controls in 32-Bit Preview Options on a layered HDR document with an above-white channel and partial alpha
- **THEN** the canvas SHALL update from the unclipped native composite and Cancel SHALL restore the opening display settings without changing scene samples.

#### Scenario: Persist document preview [preview-persistence]

- **WHEN** an HDR document with accepted preview options is saved and reopened as PSD or PSB
- **THEN** the document settings SHALL return and any view offset SHALL reset.

#### Scenario: Carry preview metadata in TIFF [tiff-metadata]

- **WHEN** preview settings are written to and read from a TIFF Photoshop resource block
- **THEN** the Pictura settings SHALL survive without altering other resources. This does not establish a native HDR TIFF pixel round trip.

#### Scenario: Adobe preview compatibility [adobe-compatibility]

- **WHEN** a PSD, PSB or TIFF with preview settings is exchanged with Photoshop
- **THEN** application of the other application's settings SHALL remain unverified until the encoding is established and checked in both directions.

#### Scenario: Adjust only one view [view-offset]

- **WHEN** a user changes 32-Bit Exposure in the status bar
- **THEN** it SHALL affect that view and double-clicking the slider SHALL restore its default.

### Requirement: HDR color picker

The foreground and background picker in an HDR document SHALL accept float RGB and adjustable intensity, offer stepped preview swatches and a Relative to Document option, and allow adding a color to Swatches with explicit confirmation before lossy conversion.

#### Scenario: Select and reopen an HDR color [hdr-picker]

- **WHEN** a user accepts an HDR color from either picker entry point
- **THEN** reopening the picker SHALL retain its scene RGB values, including values above white. Cancel SHALL preserve the opening color.

#### Scenario: Synchronize color mutations [color-state]

- **WHEN** foreground and background are swapped or reset, or an existing 8-bit color operation changes either color
- **THEN** the float state SHALL follow the operation, and reopening the picker SHALL show the current color rather than a stale HDR value.

#### Scenario: Confirm lossy swatch addition [lossy-swatch]

- **WHEN** an HDR color cannot be retained by the swatch representation and the user adds it to Swatches
- **THEN** the user SHALL be warned before conversion and MAY cancel without adding a swatch.

### Requirement: Native Info readout

The Info panel SHALL show floating-point values from the scene composite when an RGB-compatible color readout selects 32-Bit on an HDR document.

#### Scenario: Read above-white values [float-info]

- **WHEN** a user hovers a partially transparent layered HDR pixel with an above-white composite channel with the Info readout set to 32-Bit and changes display exposure
- **THEN** Info SHALL report the same unclipped native composite sample.
