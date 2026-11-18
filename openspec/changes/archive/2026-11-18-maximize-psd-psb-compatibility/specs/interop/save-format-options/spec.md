# Save Format Options Delta

## MODIFIED Requirements

### Requirement: Save As opens the destination format's options dialog

`File ▸ Save As` SHALL, once the destination path is chosen, open the options
dialog for the chosen raster format: JPEG Options for `.jpg`, `.jpeg`, and
`.jpe`; PNG Options for `.png`; BMP Options for `.bmp`; TIFF Options for `.tif`
and `.tiff`; GIF Options for `.gif`; Photoshop EPS Options for `.eps`;
DCS Format for the DCS filter; Photoshop Raw Options for `.raw`; Targa Options
for `.tga`, `.vda`, `.icb`, and `.vst`; and Cineon Options for `.cin`.

Photoshop EPS and Photoshop DCS share the `.eps` suffix. When the selected Save
As filter names DCS, the DCS Format dialog SHALL open; otherwise a `.eps`
destination SHALL open the EPS Options dialog. A destination whose suffix names
no Save As format, or that has no suffix, SHALL be refused: no file is written
and the save reports failure, rather than writing another format's bytes under
that name.

WebP and Photoshop PDF SHALL NOT open a format options dialog. PSD and PSB SHALL
open a compatibility prompt only when Ask applies as specified in
`interop/psd-psb-compatibility`.
Cancelling a format options dialog SHALL abort the save: no file is written and
the document's pixels, path, modified state, and history are unchanged.

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

#### Scenario: GIF opens GIF Options [save-options-gif]

- **WHEN** Save As is confirmed with a `.gif` destination
- **THEN** the GIF Options dialog is shown before anything is written

#### Scenario: EPS opens EPS Options [save-options-eps]

- **WHEN** Save As is confirmed with a `.eps` destination and the EPS filter
- **THEN** the Photoshop EPS Options dialog is shown before anything is written

#### Scenario: The DCS filter opens DCS Format [save-options-dcs]

- **WHEN** Save As is confirmed with a `.eps` destination and the DCS filter
- **THEN** the DCS Format dialog is shown before anything is written

#### Scenario: EPS and DCS are distinguished by the filter [save-options-eps-dcs-hint]

- **WHEN** the user chooses the DCS filter but types a `.eps` name
- **THEN** DCS Format opens, and the remembered settings for the DCS kind seed it

#### Scenario: Raw opens Photoshop Raw Options [save-options-raw]

- **WHEN** Save As is confirmed with a `.raw` destination
- **THEN** the Photoshop Raw Options dialog is shown before anything is written

#### Scenario: TGA opens Targa Options [save-options-tga]

- **WHEN** Save As is confirmed with a `.tga`, `.vda`, `.icb`, or `.vst` destination
- **THEN** the Targa Options dialog is shown before anything is written

#### Scenario: Cineon opens Cineon Options [save-options-cineon]

- **WHEN** Save As is confirmed with a `.cin` destination
- **THEN** the Cineon Options dialog is shown before anything is written

#### Scenario: Native and other formats open no format dialog [save-options-native]

- **WHEN** Save As targets PSD or PSB with Always or Never, or targets WebP or PDF
- **THEN** no format options dialog is shown

#### Scenario: Cancelling writes nothing [save-options-cancel]

- **WHEN** a format options dialog is cancelled
- **THEN** no file is written and the document's pixels, path, modified state, and history are unchanged

#### Scenario: An unknown suffix is refused [save-options-unknown-suffix]

- **WHEN** a save targets a suffix that names no Save As format, or a path with no suffix
- **THEN** the save reports failure, no file is written, and no PSD bytes are written under that name
