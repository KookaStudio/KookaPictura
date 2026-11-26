# Spec Delta

## MODIFIED Requirements

### Requirement: Save As opens the destination format's options dialog

`File ▸ Save As` SHALL, once the destination path is chosen, open the options
dialog for the chosen raster format: JPEG Options for `.jpg`, `.jpeg`, and
`.jpe`; PNG Options for `.png`; BMP Options for `.bmp`; TIFF Options for `.tif`
and `.tiff`; GIF Options for `.gif`; Photoshop EPS Options for `.eps`;
DCS Format for the DCS filter; Photoshop Raw Options for `.raw`; Targa Options
for `.tga`, `.vda`, `.icb`, and `.vst`; Cineon Options for `.cin`; and PICT
Options for `.pct`, `.pic`, and `.pict`.

Photoshop EPS and Photoshop DCS share the `.eps` suffix. When the selected Save
As filter names DCS, the DCS Format dialog SHALL open; otherwise a `.eps`
destination SHALL open the EPS Options dialog. A destination whose suffix names
no Save As format, or that has no suffix, SHALL be refused: no file is written
and the save reports failure, rather than writing another format's bytes under
that name.

PSD, PSB, WebP, and Photoshop PDF SHALL NOT open a format options dialog.
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

#### Scenario: PICT opens PICT Options [save-options-pict]

- **WHEN** Save As is confirmed with a `.pct`, `.pic`, or `.pict` destination
- **THEN** the PICT Options dialog is shown before anything is written

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
Resolution and Compress (RLE); Cineon Options its fixed 10-bit RGB conversion;
and PICT Options its Resolution (16 or 32 bits per pixel) and Compression
(None, JPEG Low, JPEG Medium, JPEG High, JPEG Maximum).

A dialog SHALL seed its controls from the last accepted settings for that format,
persisted in the session store, and SHALL use the CS6 default when none are
stored. On accept, the settings SHALL become the remembered settings for that
format. The EPS and DCS kinds SHALL be remembered separately even though both
write `.eps`.

#### Scenario: First use shows the CS6 defaults [save-options-defaults]

- **WHEN** a format options dialog is opened with no stored settings for that format
- **THEN** its controls hold the CS6 defaults (JPEG quality 10 and Baseline, PNG None/Fast, TIFF LZW, BMP Windows at 24-bit, GIF Normal, EPS Binary with TIFF preview, DCS 2.0 with TIFF 8-bit preview, Raw header 0 and interleaved, TGA 24-bit with no RLE, Cineon 10-bit, PICT 32-bit with no compression)

#### Scenario: Accepted settings seed the next dialog [save-options-remembered]

- **WHEN** settings are accepted in a format options dialog and the dialog is opened again
- **THEN** its controls hold the accepted settings

#### Scenario: PICT settings are remembered [save-options-pict-remembered]

- **WHEN** PICT Options is accepted with 16-bit resolution and JPEG High, and reopened
- **THEN** it shows 16-bit resolution and JPEG High

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
Compress (RLE), the Cineon 10-bit RGB conversion, and the PICT Resolution and
Compression.

A control the encoder cannot honor SHALL be shown disabled with a tooltip naming
the ceiling and SHALL NOT be represented as taking effect. The JPEG Matte SHALL
be shown disabled, and a transparent pixel saved as JPEG SHALL be flattened onto
opaque white. The Photoshop Raw File Type and File Creator controls SHALL be
shown disabled on Linux with a tooltip. Named spot inks in DCS 2.0 are not
supported by the document model and SHALL be reported as a feature-loss warning
rather than written. Raw and Cineon SHALL carry 8-bit document samples; the
Cineon writer packs them into 10-bit code values. A PICT JPEG compression SHALL
flatten alpha over opaque white; 32-bit PICT without compression SHALL keep
alpha.

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

#### Scenario: PICT resolution and compression are written [pict-options]

- **WHEN** PICT Options chooses 16 or 32 bits per pixel and None or a JPEG level
- **THEN** the written file carries the matching direct-pixel record or `$8200` JPEG record, and a 32-bit None save keeps alpha

#### Scenario: Unsupported controls are disabled [unsupported-disabled]

- **WHEN** a dialog is opened
- **THEN** every control the encoder cannot honor is disabled and carries a tooltip naming the ceiling

#### Scenario: Raw file type controls are disabled on Linux [raw-mac-disabled]

- **WHEN** Photoshop Raw Options is opened on Linux
- **THEN** the File Type and File Creator controls are disabled with a tooltip
