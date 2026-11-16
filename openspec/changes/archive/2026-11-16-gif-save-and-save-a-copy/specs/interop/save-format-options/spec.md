# Spec Delta

## MODIFIED Requirements

### Requirement: Save As opens the destination format's options dialog

`File ▸ Save As` SHALL, once the destination path is chosen, open the options
dialog for the chosen raster format: JPEG Options for `.jpg`, `.jpeg`, and
`.jpe`; PNG Options for `.png`; BMP Options for `.bmp`; TIFF Options for `.tif`
and `.tiff`; and GIF Options for `.gif`, which is the Indexed Color dialog whose
controls are specified in `interop/gif-save`. PSD, PSB, WebP, and Photoshop PDF
SHALL NOT open a format options dialog. Cancelling a format options dialog SHALL
abort the save: no file is written and the document's pixels, path, modified
state, and history are unchanged.

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

#### Scenario: GIF opens the Indexed Color dialog [save-options-gif]

- **WHEN** Save As is confirmed with a `.gif` destination
- **THEN** the Indexed Color dialog is shown before anything is written

#### Scenario: Native and other formats open no format dialog [save-options-native]

- **WHEN** Save As is confirmed with a `.psd`, `.psb`, `.webp`, or `.pdf` destination
- **THEN** no format options dialog is shown

#### Scenario: Cancelling writes nothing [save-options-cancel]

- **WHEN** a format options dialog is cancelled
- **THEN** no file is written and the document's pixels, path, modified state, and history are unchanged
