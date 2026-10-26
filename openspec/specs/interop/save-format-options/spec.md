# interop/save-format-options Specification

## Purpose
The per-format options dialogs that `File ▸ Save As` opens for JPEG, PNG, BMP,
and TIFF, the controls they present, the settings the Qt-boundary encoder can
actually honor, and the last-used settings remembered across sessions.

## Requirements

### Requirement: Save As opens the destination format's options dialog

`File ▸ Save As` SHALL, once the destination path is chosen, open the options
dialog for the chosen raster format: JPEG Options for `.jpg`, `.jpeg`, and
`.jpe`; PNG Options for `.png`; BMP Options for `.bmp`; and TIFF Options for
`.tif` and `.tiff`. PSD, PSB, WebP, and Photoshop PDF SHALL NOT open a format
options dialog. Cancelling a format options dialog SHALL abort the save: no file
is written and the document's pixels, path, modified state, and history are
unchanged.

#### Scenario: JPEG opens JPEG Options [save-options-jpeg]

- **WHEN** Save As is confirmed with a `.jpg` destination
- **THEN** the JPEG Options dialog is shown before anything is written

#### Scenario: PNG opens PNG Options [save-options-png]

- **WHEN** Save As is confirmed with a `.png` destination
- **THEN** the PNG Options dialog is shown before anything is written

#### Scenario: BMP opens BMP Options [save-options-bmp]

- **WHEN** Save As is confirmed with a `.bmp` destination
- **THEN** the BMP Options dialog is shown before anything is written

#### Scenario: TIFF opens TIFF Options [save-options-tiff]

- **WHEN** Save As is confirmed with a `.tif` destination
- **THEN** the TIFF Options dialog is shown before anything is written

#### Scenario: Native and other formats open no format dialog [save-options-native]

- **WHEN** Save As is confirmed with a `.psd`, `.psb`, `.webp`, or `.pdf` destination
- **THEN** no format options dialog is shown

#### Scenario: Cancelling writes nothing [save-options-cancel]

- **WHEN** a format options dialog is cancelled
- **THEN** no file is written and the document's pixels, path, modified state, and history are unchanged

### Requirement: The dialogs present the CS6 controls and remember the last settings

Each format options dialog SHALL present that format's CS6 control set: JPEG
Options its Matte, Image Options (Quality 0–12 with a preset list), Format
Options (Baseline, Baseline Optimized, Progressive), Scans, Preview, and a
file-size readout; PNG Options its Compression (None/Fast … Smallest/Slow) and
Interlace; BMP Options its File Format, Depth, Compress (RLE), Flip Row Order,
and Advanced Modes; TIFF Options its Image Compression (with Quality for JPEG),
Pixel Order, Byte Order, Layer Compression, Save Image Pyramid, and Save
Transparency. A dialog SHALL seed its controls from the last accepted settings
for that format, persisted in the session store, and SHALL use the CS6 default
when none are stored. On accept, the settings SHALL become the remembered
settings for that format.

#### Scenario: First use shows the CS6 defaults [save-options-defaults]

- **WHEN** a format options dialog is opened with no stored settings for that format
- **THEN** its controls hold the CS6 defaults (JPEG quality 10 and Baseline, PNG None/Fast, TIFF LZW, BMP Windows at 24-bit)

#### Scenario: Accepted settings seed the next dialog [save-options-remembered]

- **WHEN** settings are accepted in a format options dialog and the dialog is opened again
- **THEN** its controls hold the accepted settings

### Requirement: Options the encoder can honor take effect

The options the Qt-boundary encoder can honor SHALL take effect in the written
file: the JPEG Quality (0–12), the JPEG Format Options (Baseline, Baseline
Optimized, Progressive), the PNG Compression level, the TIFF Image Compression,
and the TIFF Save Transparency (off flattens onto opaque white). A control the
encoder cannot honor SHALL be shown disabled with a tooltip naming the ceiling
and SHALL NOT be represented as taking effect. The JPEG Matte SHALL be shown
disabled, and a transparent pixel saved as JPEG SHALL be flattened onto opaque
white.

#### Scenario: Lower JPEG quality writes a smaller file [jpeg-quality]

- **WHEN** the same document is saved as JPEG at quality 12 and at quality 2
- **THEN** the quality-2 file is smaller

#### Scenario: Progressive writes a progressive JPEG [jpeg-progressive]

- **WHEN** the JPEG Format Option is Progressive
- **THEN** the written file carries a progressive (SOF2) frame, and Baseline writes a baseline (SOF0) frame

#### Scenario: The JPEG Matte control is disabled [jpeg-matte-disabled]

- **WHEN** the JPEG Options dialog is opened
- **THEN** the Matte control is disabled with a tooltip, and a transparent pixel saved as JPEG is flattened onto opaque white

#### Scenario: PNG compression changes the file size [png-compression]

- **WHEN** the same document is saved as PNG at the lowest and highest compression levels
- **THEN** the two files differ in size

#### Scenario: TIFF compression is written [tiff-compression]

- **WHEN** TIFF Image Compression is None and LZW
- **THEN** the written TIFF's Compression tag (259) is None (1) and LZW (5) respectively

#### Scenario: TIFF Save Transparency off flattens [tiff-transparency]

- **WHEN** Save Transparency is off for a document with a transparent pixel
- **THEN** the written TIFF's pixel is opaque

#### Scenario: Unsupported controls are disabled [unsupported-disabled]

- **WHEN** a dialog is opened
- **THEN** every control the encoder cannot honor is disabled and carries a tooltip naming the ceiling

### Requirement: The JPEG Options dialog previews and estimates the file size

The JPEG Options dialog SHALL show a file-size estimate for the current settings
that updates when the settings change, and a Preview control that shows the
encoded result. Showing the preview and updating the estimate SHALL NOT change
the document, its path, its modified state, or its history.

#### Scenario: The size readout follows the quality [jpeg-size-estimate]

- **WHEN** the quality is lowered in the JPEG Options dialog
- **THEN** the file-size readout decreases

#### Scenario: Preview does not change the document [jpeg-preview-no-mutation]

- **WHEN** Preview is toggled in the JPEG Options dialog
- **THEN** the document's pixels, path, modified state, and history are unchanged
