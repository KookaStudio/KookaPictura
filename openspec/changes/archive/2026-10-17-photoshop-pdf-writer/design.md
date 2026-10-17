# Design: photoshop-pdf-writer

Contract: `docs/10-workflow-io/save-and-save-as.md` *Save in Photoshop PDF
format*, *Parameters & ranges*, *Save dispatch*; `docs/01-architecture/file-formats.md`
row PDF. Issue #626. Owner decisions on #626: the wider scope (all of the spec
except JPEG2000 encoding), Pictura's own PSD as the editing payload (marked as
an approximation), the path becomes `.pdf`, every PDF tool installed in CI, the
RustCrypto crates for security, and PhotoCraft's synthetic CC0 CMYK profile as the
CMYK destination.

## D1. Where the writer lives

`pictura_codec::pdf` (Qt-free), beside `svg` and `webp`. The doc names a
`pictura_io::pdf` built on `printpdf`/`lopdf`; no `pictura_io` crate exists and
the writer needs only a byte serializer, so neither candidate is added. Module
split (each well under 800 lines):

| Module | Role |
|---|---|
| `pdf/mod.rs` | `write_pdf`, `PdfEnv`, object layout, catalog/page/info |
| `pdf/options.rs` | `PdfOptions` and its enums, defaults, `normalize` gating, `locked` |
| `pdf/joboptions.rs` | `.joboptions` parse / canonical emit, `get`/`set` by key |
| `pdf/presets.rs` | built-in presets, summary lines |
| `pdf/object.rs` | object model + serializer (names, strings, dicts, streams, xref) |
| `pdf/image.rs` | composite → samples per output colour space, alpha, 16-bit |
| `pdf/resample.rs` | Average / Subsampling / Bicubic downsampling |
| `pdf/ccitt.rs` | CCITT Group 4 (T.6) encoder for 1-bit images |
| `pdf/crypt.rs` | standard security handler R2/R3/R4/R6 |
| `pdf/linearize.rs` | Fast Web View layout and hint stream |
| `pdf/xmp.rs` | XMP packet (PDF/A, PDF/X-4) |
| `pdf/read.rs` | find Pictura's embedded PSD in a Pictura-written PDF |

The engine takes the rendered composite and the document; it never calls Qt.

## D2. JPEG through the Qt boundary

`raster-export` forbids an engine raster encoder (WebP excepted). `write_pdf`
takes `PdfEnv { jpeg: &dyn Fn(&JpegRequest) -> Option<Vec<u8>>, now, entropy }`
the way `encode_document_svg` takes its `raster` closure. The app passes a
closure over a new `encode_jpeg` C++ edge (`QImageWriter` into a `QBuffer`,
Grayscale8 / RGB888 / CMYK8888). Engine tests pass a stub. A `None` from the
closure fails the save (no silent ZIP fallback). CMYK JPEG polarity is pinned
by the render oracle (`/Decode` is emitted to match Qt's output).

## D3. Options are one type, serialized as `.joboptions`

No serde in the workspace, so `PdfOptions` owns a hand-written serializer to a
canonical Distiller-style dictionary, one `/Key value` per line inside
`<< … >> setdistillerparams`. The same text is the preset file format, the
session's "last used" value, and the C++↔Rust interchange: the dialog keeps the
canonical text and edits it through `pdf_option_get` / `pdf_option_set`, so the
C++ side never parses PostScript. Distiller keys are used where one exists
(`CompatibilityLevel`, `DownsampleColorImages`, `ColorImageDownsampleType`,
`ColorImageResolution`, `ColorImageDownsampleThreshold` (a ratio, shown as ppi),
`EncodeColorImages`, `ColorImageFilter`, `ColorImageDict /QFactor`,
`JPEG2000ColorImageDict /TileWidth`, `ColorConversionStrategy`,
`DestinationProfileName`, `CheckCompliance`, `PDFXOutputIntentProfile`,
`PDFXOutputCondition`, `PDFXOutputConditionIdentifier`, `PDFXRegistryName`,
`Optimize`, `Description`); Photoshop-only settings use `Pictura*` keys. The
parser accepts real Acrobat files: comments, nested dicts/arrays, `(…)` and
`<FEFF…>` strings, `/Description << /ENU … >>`, unknown keys ignored.
Security keys and Save options are dropped when a preset is written
("presets do not store passwords").

JPEG Image Quality ↔ QFactor follows Distiller: Maximum 0.15, High 0.4,
Medium 0.76, Low 1.3, Minimum 2.4; the Qt quality for each (95/85/70/50/25)
is inferred.

## D4. Compatibility and standards gating (single source of truth)

`PdfOptions::normalize()` applies the rules below and `locked(key)` reports
which settings a level or standard forces; the dialog disables exactly those.

| Level | Header | Transparency | 16-bit | Encryption |
|---|---|---|---|---|
| Acrobat 4 | 1.3 | flattened on white | forced 8 | RC4 40-bit (V1 R2) |
| Acrobat 5 | 1.4 | soft mask | forced 8 | RC4 128-bit (V2 R3) |
| Acrobat 6 | 1.5 | soft mask | allowed | RC4 128-bit (V2 R3) |
| Acrobat 7 | 1.6 | soft mask | allowed | AES 128-bit (V4 R4) |
| Acrobat X | 1.7 + ADBE ext 8 | soft mask | allowed | AES 256-bit (V5 R6) |

The doc's range reads "Acrobat 3 (1.3) … Acrobat X (1.7)" and "forced to
8-bit below PDF 1.4". Acrobat 3 wrote PDF 1.2, and ISO 32000 adds 16 bits per
component in PDF 1.5, so the list starts at Acrobat 4 (1.3) and 16-bit needs
1.5; both are resolutions toward valid files, recorded here.

Standards: PDF/X-1a:2001 forces 1.3, CMYK (or gray) device colour via Convert
To Destination, no transparency, no security; PDF/X-3:2002 forces 1.3, no
transparency, no security; PDF/X-4:2008 forces at least 1.6 and no security;
PDF/A-1b:2005 forces 1.4, no transparency, no security, no JPEG2000. Every
PDF/X writes `GTS_PDFX` output intent with the destination profile, `/Trapped`,
`/Title`, dates, `GTS_PDFXVersion`, `TrimBox`; X-4 and PDF/A write XMP
(`pdfxid` / `pdfaid`). PDF/A's output intent is `GTS_PDFA1` with sRGB (RGB
output) or the CMYK profile. Validation by Acrobat Preflight / veraPDF is a
manual row; the structural rules are unit-tested.

JPEG2000: neither Qt nor the dependency tree encodes it. The option is listed
and disabled with a tooltip (ponytail; follow-up issue).

## D5. Output colour by mode

The working model is 8-bit Gray/RGB; `source_mode`/`source_depth` name the
output, as in `write_psd`.

| Output | PDF colour space | Samples |
|---|---|---|
| Bitmap (`writes_bitmap`) | DeviceGray, 1 bpc | CCITT G4 (`K -1`); None → raw 1-bit |
| Grayscale | DeviceGray / ICCBased(N 1) | composite gray |
| RGB, Duotone | DeviceRGB / ICCBased(N 3) | composite RGB (Duotone flattens, ponytail) |
| CMYK source | DeviceCMYK | `255 - psd_plane` (PSD stores 255 = no ink) |
| Lab source | `/Lab` D50, Range `[-128 127 -128 127]` | `rgb_to_lab` bytes |
| Indexed (`writes_indexed`) | `/Indexed /DeviceRGB 255 <palette>` | index plane; JPEG expands to RGB |
| Multichannel | — | refused: "Photoshop PDF does not support Multichannel" |

Convert To Destination converts the composite (source: the document profile,
sRGB when untagged, Gray Gamma 2.2 for gray) to the destination through lcms2
(`pictura_color::convert_space`). Destinations: the document profile, sRGB,
Adobe RGB, ProPhoto RGB, Gray Gamma 2.2, Photocraft Coated CMYK. ICC tagging: with
No Conversion the Save option *ICC Profile* decides; with a conversion the
*Profile Inclusion Policy* (Include Destination Profile / Don't Include
Profiles) decides. The app's CMYK and Lab sources are profile-free, so they are
never ICC-tagged.

## D6. Preserve Photoshop Editing Capabilities (approximation)

The page carries `/PieceInfo << /Pictura << /LastModified … /Private << /PSD n 0 R >> >> >>`
and `/LastModified`; stream `n` is `write_psd` of the saved document (Flate,
encrypted with everything else). ponytail: Adobe's private layout is
unpublished; Photoshop is not claimed to reopen these layers. *Layers* off
embeds the flattened document; *Alpha Channels* off drops `doc.channels`;
*Spot Colors* is disabled (the model does not tell spot from alpha channels,
ponytail); *Notes* off drops the notes. `pdf/read.rs` finds the `/Pictura`
PieceInfo `PSD` stream of an unencrypted Pictura PDF, so `File > Open` and
Revert work on the new `.pdf` path; any other PDF is refused and stays #627.

## D7. Page, notes, thumbnails, metadata

MediaBox (and TrimBox for PDF/X) is `width × 72 / ppi` by `height × 72 / ppi`
points (72 ppi when the resolution resource is absent); the image is drawn
full-page by one content stream `q W 0 0 H 0 0 cm /Im0 Do Q`, followed by the
live text (D13). Downsampling
changes the image's pixel size, never the page. Each note becomes a `/Text`
annotation at its point (y flipped). *Embed Page Thumbnails* adds `/Thumb`, an
RGB image whose long side is at most 106 px. `/Info` carries `Producer`,
`Creator`, `CreationDate`, `ModDate`, `Title` (file stem); `PdfEnv.now` makes
dates deterministic in tests.

## D8. Downsampling

Applies when the document ppi exceeds the threshold ("for images above");
target size is `round(w × target / ppi)`. Average is a box over each target
pixel's source footprint; Subsampling takes the footprint's centre sample;
Bicubic is the Keys `a = -0.75` kernel already used by Image Size, evaluated
at the footprint centre, on u16 samples so 16-bit output downsamples too. The
soft mask downsamples with the image; Bitmap images are not downsampled
(ponytail: CS6's separate monochrome settings are not modelled).

## D9. Security

Primitives: `md-5`, `aes`, `sha2`; RC4 and AES-CBC with PKCS#5 padding are
written in `crypt.rs` over them. ISO 32000-1 §7.6.3 algorithms 2–7 (R2–R4) and ISO 32000-2 algorithms
2.A/2.B/8–10 (R6). Strings and streams are encrypted per object (RC4 or AES-CBC
with a 16-byte IV prefix and PKCS#5 padding); the `/Encrypt` dictionary and
the document ID are not; *Enable plaintext metadata* leaves the XMP stream
unencrypted (`/EncryptMetadata false`, V4/V5 only). Permissions map to `/P`
bits: printing (None / Low Resolution / High Resolution → bits 3, 12), changes
(None / page assembly / form fill and signing / commenting, form fill and
signing / any except extracting pages → bits 4, 6, 9, 11), copying (bit 5),
screen-reader access (bit 10). No permissions password means unrestricted
permissions with the owner password equal to the user password. Randomness
(file ID, AES IVs, R6 salts and file key) comes from `PdfEnv.entropy`: the app
seeds it from the OS through std's `RandomState`, tests fix it (rule 6).

## D10. Fast Web View

ISO 32000-1 Annex F for a one-page file: linearization dictionary, first-page
cross-reference section and trailer, the first-page objects (catalog, page,
content, image, soft mask, thumbnail, annotations), the primary hint stream
(page offset and shared object hint tables), then the remaining objects (info,
metadata, embedded PSD) and the main cross-reference section. Offsets are
resolved by laying out twice with fixed-width numbers. `qpdf
--check-linearization` is the oracle.

## D11. App integration

- Save As gains `Photoshop PDF (*.PDF *.PDP)`. A `.pdf`/`.pdp` path opens the
  Save Adobe PDF dialog (seeded with the document's last PDF options, else the
  session's last used, else `[Photoshop Default]`) before writing; Cancel
  writes nothing. Save on a `.pdf` path opens the dialog again.
- A layered document with Preserve Editing off gets the existing "cannot hold
  all features" warning.
- Writes are temp-file + rename; the path becomes the `.pdf` and the document
  is unmodified on success. *View PDF After Saving* opens the file with
  `QDesktopServices` (skipped offscreen).
- The dialog: Save options group (Layers, Notes, Alpha Channels, Spot Colors,
  ICC Profile, Use Proof Setup — disabled, no Proof Setup yet), Adobe PDF
  Preset, Standard, Compatibility, a left page list General / Compression /
  Output / Security / Summary, Save Preset…, Save PDF / Cancel.
- `Edit > Adobe PDF Presets…` (the existing leaf, now live): list with
  built-ins in brackets, Description, New / Edit / Delete / Load / Save As.
  User presets are `.joboptions` files in `<state dir>/PDF Presets/`;
  built-ins cannot be edited or deleted.

## D13. Type layers as live text

A Photoshop PDF keeps type as text: Acrobat selects, searches, and edits it.
Drawing the composite alone made every type layer pixels, so the page now
holds the composite *without* the qualifying type layers as its image, and
each of those layers is set over it in its embedded font.

- **Which layers.** Walking from the top of the stack down, a visible type
  layer qualifies while everything visible above it also qualified, reached
  through neutral groups (pass-through or Normal, 100% opacity and fill, no
  mask, vector mask, or effects). The layer itself is Normal, unclipped,
  unmasked, without effects, unwarped, and one style run
  (`pictura_codec::type_is_plain`). Drawing such a layer after the image
  changes no pixel beneath it, so the page renders like the composite. The
  first layer that fails stops the walk: it and everything below stay in the
  image. ponytail: type under other content, non-Normal blends, effects, and
  multi-style text rasterize; carrying them would need the content above split
  into transparent image bands.
- **Glyphs.** `pictura_render::pdf_live_type` lays the layer out with the same
  code the canvas renders it with (`type_pens`, shared with `render_mapped`):
  glyph ids and pens in the untransformed frame, then the `TySh` matrix and
  H/V scale go into each run's text matrix. Glyphs on one baseline form one
  `TJ` whose adjustments carry the layout's kerning, tracking, and alignment
  against the font's stated widths.
- **Fonts.** The face the layer resolves to (`fonts::face_for`: the
  registered host face, else bundled Liberation Sans) is embedded *whole* as a
  `Type0` / `Identity-H` font: CIDFontType2 + `FontFile2` for TrueType,
  CIDFontType0 + `FontFile3 /OpenType` for CFF (PDF 1.6+ only; earlier levels
  keep CFF type in the image). A whole face, not a subset, is what lets an
  editor type characters the document never used. `ToUnicode` maps each glyph
  to its shaping cluster's characters, so ligatures extract correctly. A font
  whose `OS/2 fsType` is Restricted is not embedded; its layer stays in the
  image. Before a dialog save the shell registers every type layer's family
  (`pdf_type_fonts`) so a document opened untouched sets type in its own face.
- **Colour and opacity.** The fill follows the image's path (D5): the same
  destination conversion, CMYK ink, Lab values, or ICC space (as `/CS0`); an
  indexed image's text is DeviceRGB, Bitmap's black or white. Opacity × fill
  below 100% becomes an `ExtGState` `/ca` only where transparency is kept
  (D4); otherwise the layer stays in the image.
- **Unchanged.** The thumbnail and the editing payload (D6) take the full
  composite and document. With live text the image is encoded from the
  background's own samples (retained source planes still hold the type), so
  an Indexed source with type is written as RGB.

## D12. Oracles

- `pdftoppm` (poppler) and `gs` render the PDF; the render is compared with the
  CPU composite (exact for ZIP/None, mean-tolerance for JPEG).
- `qpdf --check`, `--check-linearization`, `--show-encryption`, and
  `--password=… --decrypt` check structure, linearization, and encryption.
- `pdfinfo` checks page size, version, and the encrypted flag.
- Tests self-skip when a tool is absent; CI installs all three packages.
