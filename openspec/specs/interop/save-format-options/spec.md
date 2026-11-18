# interop/save-format-options Specification

## Purpose
The per-format options dialogs that `File ▸ Save As` opens for JPEG, PNG, BMP,
TIFF, GIF, EPS, DCS, Photoshop Raw, TGA and Cineon, the controls they present,
the settings the Save As encoders can honor, and the last-used settings
remembered across sessions.

## Requirements

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

### Requirement: The dialogs present the CS6 controls and remember the last settings

Each format options dialog SHALL present that format's CS6 control set: JPEG
Options its Matte, Image Options (Quality 0–12 with a preset list), Format
Options (Baseline, Baseline Optimized, Progressive), Scans, Preview, and a
file-size readout; PNG Options its Compression (None/Fast … Smallest/Slow) and
Interlace; BMP Options its File Format, Depth, Compress (RLE), Flip Row Order,
and Advanced Modes; TIFF Options its Image Compression (with Quality for JPEG),
Pixel Order, Byte Order, Layer Compression, Save Image Pyramid, and Save
Transparency; GIF Options its Row Order; Photoshop EPS Options its Preview,
Encoding (with JPEG quality), Include Halftone Screen, Include Transfer
Function, Transparent Whites, PostScript Color Management, Include Vector Data,
and Image Interpolation; DCS Format its version (1.0 or 2.0) and Preview;
Photoshop Raw Options its Header size and Interleave; Targa Options its
Resolution and Compress (RLE); and Cineon Options its fixed 10-bit RGB
conversion.

A dialog SHALL seed its controls from the last accepted settings for that format,
persisted in the session store, and SHALL use the CS6 default when none are
stored. On accept, the settings SHALL become the remembered settings for that
format. The EPS and DCS kinds SHALL be remembered separately even though both
write `.eps`.

#### Scenario: First use shows the CS6 defaults [save-options-defaults]

- **WHEN** a format options dialog is opened with no stored settings for that format
- **THEN** its controls hold the CS6 defaults (JPEG quality 10 and Baseline, PNG None/Fast, TIFF LZW, BMP Windows at 24-bit, GIF Normal, EPS Binary with TIFF preview, DCS 2.0 with TIFF 8-bit preview, Raw header 0 and interleaved, TGA 24-bit with no RLE, Cineon 10-bit)

#### Scenario: Accepted settings seed the next dialog [save-options-remembered]

- **WHEN** settings are accepted in a format options dialog and the dialog is opened again
- **THEN** its controls hold the accepted settings

### Requirement: Options the encoder can honor take effect

The options the Save As encoder can honor SHALL take effect in the written file:
the JPEG Quality (0–12), the JPEG Format Options (Baseline, Baseline Optimized,
Progressive), the PNG Compression level and Interlace, the BMP File Format,
Depth, Compress (RLE), Flip Row Order and Advanced Modes, the TIFF Image
Compression (None, RLE, LZW, ZIP, JPEG), Pixel Order, Byte Order, Save Image
Pyramid, Layer Compression and Save Transparency (off flattens onto opaque
white), the GIF Row Order, the EPS Encoding, Preview, JPEG quality, Include
Halftone Screen, Include Transfer Function, Transparent Whites, PostScript Color
Management, Include Vector Data and Image Interpolation, the DCS version and
Preview, the Photoshop Raw Header size and Interleave, the Targa Resolution and
Compress (RLE), and the Cineon 10-bit RGB conversion.

A control the encoder cannot honor SHALL be shown disabled with a tooltip naming
the ceiling and SHALL NOT be represented as taking effect. The JPEG Matte SHALL
be shown disabled, and a transparent pixel saved as JPEG SHALL be flattened onto
opaque white. The Photoshop Raw File Type and File Creator controls SHALL be
shown disabled on Linux with a tooltip. Named spot inks in DCS 2.0 are not
supported by the document model and SHALL be reported as a feature-loss warning
rather than written. Raw and Cineon SHALL carry 8-bit document samples; the
Cineon writer packs them into 10-bit code values.

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

#### Scenario: PNG interlace is written [png-interlace-option]

- **WHEN** PNG Interlace is Interlaced
- **THEN** the written PNG's IHDR declares Adam7 interlacing, and None declares no interlace

#### Scenario: BMP options are written [bmp-options]

- **WHEN** BMP Options chooses OS/2 or Windows, a depth, RLE, or Flip Row Order
- **THEN** the written BMP header and pixel data carry that format, depth, RLE encoding, and row order

#### Scenario: TIFF compression is written [tiff-compression]

- **WHEN** TIFF Image Compression is None, RLE, LZW, ZIP, or JPEG
- **THEN** the written TIFF's Compression tag (259) is 1, 32773, 5, 8, or 7 respectively

#### Scenario: TIFF pixel and byte order are written [tiff-pixel-byte-order]

- **WHEN** TIFF Pixel Order is Per Channel and Byte Order is Macintosh
- **THEN** the written TIFF's PlanarConfiguration tag (284) is 2 and its header is big-endian

#### Scenario: TIFF Save Image Pyramid is written [tiff-pyramid-option]

- **WHEN** Save Image Pyramid is on
- **THEN** the written TIFF carries reduced-resolution SubIFDs and remains readable as the full-size image

#### Scenario: TIFF Layer Compression is written [tiff-layer-compression-option]

- **WHEN** a layered document is saved as TIFF with Layer Compression RLE or ZIP
- **THEN** the embedded Photoshop layer section exists and its layer channels use the chosen compression, and Discard Layers writes a flat TIFF

#### Scenario: TIFF Save Transparency off flattens [tiff-transparency]

- **WHEN** Save Transparency is off for a document with a transparent pixel
- **THEN** the written TIFF's pixel is opaque

#### Scenario: EPS options are written [eps-options]

- **WHEN** EPS Encoding, Preview, JPEG quality, halftone screen, transfer function, transparent whites, PostScript color management, vector data, or interpolation is chosen
- **THEN** the written EPS reflects each chosen setting in its DSC header, preview block, or PostScript image invocation

#### Scenario: Raw options are written [raw-options]

- **WHEN** Photoshop Raw Header is set and Interleave is Interleaved or Non-interleaved
- **THEN** the written file starts with that many placeholder bytes and orders the channel samples accordingly

#### Scenario: TGA options are written [tga-options]

- **WHEN** Targa Resolution is 16, 24, or 32 and Compress (RLE) is on or off
- **THEN** the written TGA header carries that pixel depth and image type and its pixels decode at that depth

#### Scenario: Cineon output is 10-bit RGB [cineon-options]

- **WHEN** a document is saved as Cineon
- **THEN** the written file carries three 10-bit channels packed into 32-bit words and no 8-bit samples

#### Scenario: Unsupported controls are disabled [unsupported-disabled]

- **WHEN** a dialog is opened
- **THEN** every control the encoder cannot honor is disabled and carries a tooltip naming the ceiling

#### Scenario: Raw file type controls are disabled on Linux [raw-mac-disabled]

- **WHEN** Photoshop Raw Options is opened on Linux
- **THEN** the File Type and File Creator controls are disabled with a tooltip

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

### Requirement: Save As dispatch and feature warnings

A Save As to a format with an engine writer SHALL NOT fall through to the PSD
background save: `PictureView::save`, `saveRunsOnWorker`, `save_start_rust` and
`document_api::saveStart` SHALL agree on the format set, and an unknown or
missing suffix SHALL be refused. A layered document saved to a format that
flattens layers SHALL show the existing feature-loss warning before writing.

#### Scenario: New and unknown formats never reach the PSD worker [save-options-no-psd-worker]

- **WHEN** a Save As targets GIF, TGA, Raw, Cineon or an unregistered suffix
- **THEN** the new-format writer runs or the save is refused, and the PSD background worker is not used

#### Scenario: New formats warn before flattening [save-options-feature-warning]

- **WHEN** a layered document is saved as GIF or TGA
- **THEN** the existing feature-loss warning is shown before anything is written

### Requirement: GIF Save As previews Indexed Color without converting

When the document is not already in Indexed mode, `File ▸ Save As` for GIF SHALL
first present the Indexed Color dialog (palette, colors, dither, amount, and
preview), then GIF Options with its Row Order control. Accepting the dialogs
SHALL NOT change the document's mode, depth, pixels, path, modified state, or
history: the GIF SHALL be encoded from a snapshot using the accepted palette
settings and row order. Cancelling either dialog SHALL leave the document
unchanged and write no file. A document already in Indexed mode SHALL skip the
Indexed Color dialog, use its retained palette, and apply the remembered row
order.

#### Scenario: GIF Save As keeps the document mode [save-options-gif-indexed]

- **WHEN** an RGB document is saved as GIF and both dialogs are accepted
- **THEN** the document remains RGB and the written GIF uses the accepted palette and row order

#### Scenario: Cancelling the GIF flow leaves the mode unchanged [save-options-gif-cancel]

- **WHEN** either the Indexed Color dialog or GIF Options is cancelled
- **THEN** the document's mode, pixels, path, modified state, and history are unchanged and no file is written

#### Scenario: Row order is written [gif-row-order]

- **WHEN** GIF Row Order is Interlaced
- **THEN** the written GIF's image descriptor carries the interlace flag, and Normal leaves it clear

### Requirement: DCS Save As writes a separation file set

DCS 1.0 SHALL write the composite master file at the chosen `.eps` path plus one
plate file per process color (`.C`, `.M`, `.Y`, `.K`). DCS 2.0 SHALL write one
`.eps` file carrying the process-color separations. A document that is not CMYK
or Multichannel SHALL be converted to CMYK for the separations with the existing
feature-loss warning shown before writing. Document extra channels whose ink
names the document model cannot represent SHALL NOT be written as named spot
inks; the save SHALL warn that the extra channels are omitted. The complete file
set SHALL be written as a unit: a failure or cancellation leaves no partial set
and does not change the document's path, modified state, or history.

#### Scenario: DCS 1.0 writes the plate set [save-options-dcs-1-files]

- **WHEN** DCS 1.0 is accepted for `art.eps`
- **THEN** `art.eps`, `art.C`, `art.M`, `art.Y`, and `art.K` exist and each plate holds one process-color image

#### Scenario: DCS 2.0 writes one file [save-options-dcs-2-single]

- **WHEN** DCS 2.0 is accepted for `art.eps`
- **THEN** only `art.eps` is written and it carries the four process-color separations

#### Scenario: Extra channels are not mislabelled as spots [dcs-spot-ceiling]

- **WHEN** the document has extra channels and DCS is saved
- **THEN** the save warns that named spot inks are not supported and writes process CMYK only

#### Scenario: A failed DCS save leaves no partial set [dcs-atomic-set]

- **WHEN** writing one plate of a DCS 1.0 set fails
- **THEN** no plate or master from that save remains and the document is unchanged

### Requirement: Plain Save reuses the remembered format kind

When a document was last saved through the EPS or DCS filter, a subsequent
plain `File ▸ Save` to its `.eps` path SHALL use the same kind and its
remembered settings, rather than guessing EPS from the shared suffix. A plain
Save to any other Save As format SHALL use that format's remembered settings.

#### Scenario: Plain Save keeps EPS and DCS apart [save-options-plain-kind]

- **WHEN** a document is saved through the DCS filter and then saved again without Save As
- **THEN** the second save writes the DCS file set with the DCS settings, not an EPS file

#### Scenario: Plain Save uses remembered options [save-options-plain-remembered]

- **WHEN** a `.tga` document is saved again without opening a dialog
- **THEN** the written TGA uses the last accepted Targa Options
