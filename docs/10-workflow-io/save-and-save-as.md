# Save and Save As

- **Spec ID:** `WF-003`
- **Status:** `Draft`
- **Parity tier:** `Core` (PSD/PSB, TIFF, JPEG, PNG, GIF, PDF, EPS, BMP, Targa, Photoshop Raw, DCS); `Extended-only` (Cineon, DICOM, and other Extended/legacy formats)
- **New in CS6:** `Changed` — the save format set and option dialogs are substantially the CS5 set; CS6's additions are the background-save and auto-recovery preferences (`WF-001`) and the broadened format-support list. PSD/PSB **Maximize Compatibility** and the PDF export pipeline are unchanged.
- **Depends on:** `ARCH-011` file-formats, `ARCH-008` document-model, `ARCH-007` color-management, `WF-001` document-lifecycle, `WF-002` open-and-new, `WF-004` export-formats, `WF-005` web-export-and-slices.

> All crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. CS6 behavior is taken from the fetched CS6 Help PDF
> unless marked *(inferred)*. Where option labels are quoted, they are the Help
> labels.

## CS6 behavior

### Save vs. Save As vs. Save a Copy

| Command | Behavior |
|---|---|
| `File > Save` | Writes changes to the **current** file, remaining in the **current format**. |
| `File > Save As` | Writes to a different name, location, or format; chooses the format from the **Format** menu. |
| `File > Save a Copy` | The **As A Copy** option: writes a copy **while keeping the current file open on your desktop**; the document's own format/path does not change. |

`Save As` shows a warning at the bottom of the dialog when the chosen format
cannot hold all of the document's features; Help advises saving a copy in PSD or
another full-capability format in that case. A format-specific options dialog
appears after `Save` for formats that need one.

### File saving options (Save As dialog)

Available per image and format:

- **As A Copy** — copy, keep the current file open.
- **Alpha Channels** — keep alpha channels; disabling removes them from the saved file.
- **Layers** — preserve all layers; when disabled/ unavailable, visible layers are
  flattened or merged per the format.
- **Notes** — save notes.
- **Spot Colors** — save spot channels; disabling removes them.
- **Use Proof Setup / ICC Profile (Windows) / Embed Color Profile (macOS)** —
  create a color-managed document (ICC embedding).
- **Thumbnail (Windows)** / **Image Previews (macOS)** — save thumbnail/preview data.
- **Use Lower Case Extension (Windows)** / **File Extension (macOS)** — extension case/append.

The image-preview and file-extension controls appear only when the corresponding
File Handling preference is **Ask When Saving** (`WF-001`).

### File saving preferences (File Handling)

Image Previews (Never Save / Always Save / Ask When Saving; on macOS select one or
more preview types: Icon, Full Size, Macintosh Thumbnail, Windows Thumbnail), File
Extension (Windows), Append File Extension + Use Lower Case (macOS), Save As to
Original Folder, Save in Background (CS6), Automatically Save Recovery Information
(CS6). See `WF-001`.

### Large documents

Photoshop supports up to 300,000 px per dimension and offers three formats for
images over 30,000 px:

- **Large Document Format (PSB)** — any file size; all Photoshop features
  preserved; PSB-readable only by Photoshop CS and later; some plug-in filters are
  unavailable above 30,000 px.
- **Photoshop Raw** — any pixel/file size, but **no layers** (flattened).
- **TIFF** — up to 4 GB.

PSD itself is a 2 GB format; TIFF's 4 GB and PSB's unlimited sizes are the overflow
paths.

### Version compatibility

- Saving to an **earlier Photoshop version** discards features that version does
  not support (Help's note under Maximize Compatibility).
- PSD/PSB **Maximize Compatibility** (Always / Ask / Never) writes a merged
  composite so older Photoshop and non-Photoshop applications can read the file
  (`WF-001`).
- The `Format` list determines the target family; CS6 has no separate numeric
  "save as version" combo for PSD *(inferred)*.

### Bit-depth support by format

- **16-bit** (requires Save As): Photoshop, PSB, Cineon, DICOM, IFF, JPEG, JPEG
  2000, Photoshop PDF, Photoshop Raw, PNG, Portable Bit Map, TIFF.
- **32-bit** (requires Save As): Photoshop, PSB, OpenEXR, Portable Bitmap,
  Radiance, TIFF.
- **Save For Web & Devices** automatically converts 16-bit to 8-bit (`WF-005`).
- JPEG supports only 8-bit; saving a 16-bit image to JPEG auto-lowers the depth.

### Per-format options

- **PSD / PSB**: Maximize Compatibility composite; 16- and 32-bpc supported. PSD is
  the only format besides PSB that supports all Photoshop features.
- **Photoshop 2.0 (macOS)**: flattens and discards layers.
- **TIFF**: **Image Compression** (None, RLE, LZW, ZIP, JPEG); **Pixel Order**
  (interleaved vs. planar); **Byte Order** (IBM PC / Macintosh); **Save Image
  Pyramid**; **Save Transparency**; **Layer Compression** (Discard Layers And Save
  A Copy; else keep layer data); **Bit depth (32-bit only)** (16/24/32);
  **Predictor** for 32-bit LZW/ZIP. JPEG compression is available only for opaque
  8-bpc RGB/Gray images ≤30,000 px. **Ask Before Saving Layered TIFF Files** is a
  File Handling preference.
- **JPEG**: **Matte**; **Image Options** quality (menu, slider, or 0–12);
  **Format Options** (Baseline "Standard", Baseline Optimized, Progressive).
- **PNG**: **Interlace** (None / Interlaced).
- **GIF (CompuServe GIF)**: RGB images first pass through the **Indexed Color**
  dialog, then a row order (**Normal** / **Interlaced**). GIF is 8-bpc only.
- **Photoshop EPS**: **Preview** (TIFF 1-bit/8-bit / None), **Encoding** (ASCII,
  ASCII85, Binary, JPEG; JPEG quality), **Include Halftone Screen**, **Include
  Transfer Function**, **Transparent Whites** (Bitmap only), **PostScript Color
  Management**, **Include Vector Data** (vector preserved for other apps, rasterized
  in Photoshop), **Image Interpolation** (smooth the preview). EPS supports Lab,
  CMYK, RGB, Indexed, Duotone, Grayscale, Bitmap; **no alpha channels**; supports
  clipping paths.
- **Photoshop DCS 1.0 / 2.0**: EPS-based CMYK/multichannel separations. DCS 1.0
  writes one file per color channel plus an optional composite (keep all files
  together); DCS 2.0 retains spot channels and can write a single file.
- **Photoshop Raw**: **Header** size (default 0), **interleaved / non-interleaved**
  channel order, and (macOS) File Type / File Creator. Not the same as camera raw.
- **BMP**: Windows or OS/2 format; bit depth (1–24, and 32-bit alpha per the format
  topic); RLE for 4/8-bit Windows; **Flip Row Order**; **Advanced Modes**.
- **Cineon**: 16-bpc RGB only (Kodak film workflow).
- **Targa (TGA)**: resolution (16/24/32-bit) and **Compress (RLE)**.

### Save in Photoshop PDF format

`File > Save As > Photoshop PDF` first exposes a **Color** option (embed a color
profile or use the Proof Setup profile) plus layers/notes/spot color/alpha
inclusion, then the **Save Adobe PDF** dialog:

- **Adobe PDF Preset** (easiest path) and per-category fine-tuning:
  **General**, **Compression**, **Output**, **Security**, **Summary**.
- General: **Description**, **Preserve Photoshop Editing Capabilities**,
  **Embed Page Thumbnails**, **Optimize For Fast Web View**, **View PDF After
  Saving**.
- Compression/downsampling: **Do Not Downsample** or **Average / Subsampling /
  Bicubic Downsampling** with a resolution and a "for images above" threshold;
  **Compression** ZIP (lossless), JPEG (lossy), JPEG2000; **Image Quality**;
  **Tile Size** (JPEG2000); **Convert 16 Bit/Channel Image To 8 Bit/Channel**
  (default on; ZIP is the only compression if off; forced to 8-bit below PDF 1.4).
- Output / PDF-X: **Color Conversion** (No Conversion / Convert To Destination),
  **Destination**, **Profile Inclusion Policy**, **Output Intent Profile Name**,
  **Output Condition**, **Output Condition Identifier**, **Registry Name**.
- Security: document-open password, permissions password, permissions, encryption
  level (tied to Compatibility); presets do **not** store passwords.
- Standards/presets: PDF/X-1a, PDF/X-3, PDF/X-4 (live transparency), PDF/A-1b,
  High Quality Print, Press Quality, Smallest File Size, Rich Content PDF,
  Standard, Illustrator Default, Oversized Pages. Save/load/edit/delete custom
  `.joboptions` presets via `Edit > Adobe PDF Presets` or **Save Preset**.

**Photoshop PDF** (Preserve Photoshop Editing Capabilities on) can contain only a
single image and preserves layers/alpha/spot color; it opens in Photoshop CS2+.
**Standard PDF** (option off) supports multiple pages/images and rasterizes
vector/text on open. Photoshop PDF supports all modes except Multichannel; Bitmap
uses CCITT Group 4 compression.

### Save a Copy and Save for Web

`Save a Copy` uses the same format/option matrix without changing the document.
Web-oriented output is a separate dialog: `File > Save for Web & Devices`
(`WF-005`), which also flattens 16-bit to 8-bit and produces GIF/JPEG/PNG-8/PNG-24.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `File > Save` | Command | `Ctrl/Cmd+S` *(inferred)* | Current format |
| `File > Save As` | Dialog | `Ctrl/Cmd+Shift+S` *(inferred)* | Format + options |
| `File > Save a Copy` | Dialog | — | As A Copy; keeps current file open |
| `File > Save for Web & Devices` | Dialog | `Ctrl/Cmd+Shift+Alt+S` *(inferred)* | `WF-005` |
| Save As dialog — File saving options | Checkbox group | — | Layers, alpha, notes, spot, ICC, previews |
| Save As dialog — Format pane | Dialog section | — | Rebuilt per format |
| TIFF Options | Dialog | — | Compression, pixel/byte order, pyramid, transparency, layer compression |
| JPEG Options | Dialog | — | Matte, quality 0–12, baseline/optimized/progressive |
| PNG Options | Dialog | — | Interlace |
| GIF Options | Dialog | — | Indexed-color conversion, row order |
| EPS Options | Dialog | — | Preview, encoding, halftone, vector data, interpolation |
| DCS Format | Dialog | — | DCS 1.0 / 2.0 |
| Photoshop Raw Options | Dialog | — | Header, channel order, file type/creator |
| BMP Options | Dialog | — | Format, depth, RLE, flip, advanced |
| Targa Options | Dialog | — | Resolution, RLE |
| Save Adobe PDF | Dialog | — | Preset, General, Compression, Output, Security, Summary |
| `Edit > Adobe PDF Presets` | Dialog | — | Save/load/edit/delete `.joboptions` |
| `Edit/PS > Preferences > File Handling` | Preference | `Ctrl/Cmd+K` | Compatibility, previews, extension, background save, recovery |

## Parameters & ranges

| Control | Format | Type | Default | Range / options | Notes |
|---|---|---|---|---|---|
| As A Copy | all | bool | Off | on / off | Keep current file open |
| Alpha Channels | all | bool | On *(inferred)* | on / off | Drop alpha on save |
| Layers | all | bool | On *(inferred)* | on / off | Flatten/merge when off |
| Notes | all | bool | On *(inferred)* | on / off | — |
| Spot Colors | all | bool | On *(inferred)* | on / off | — |
| Embed Color Profile / ICC | all | bool | On *(inferred)* | on / off | Uses proof setup on Windows |
| Image Compression | TIFF | enum | LZW *(inferred)* | None, RLE, LZW, ZIP, JPEG | JPEG only for opaque 8-bpc RGB/Gray ≤30k px |
| Predictor | TIFF (32-bit) | enum | None | None / Predictor | LZW & ZIP with 32-bit |
| Pixel Order | TIFF | enum | Interleaved | Interleaved, Planar | Planar can compress slightly better |
| Byte Order | TIFF | enum | IBM PC *(inferred)* | IBM PC / Macintosh | Both readable by modern apps |
| Save Image Pyramid | TIFF | bool | Off | on / off | Multiresolution |
| Save Transparency | TIFF | bool | On *(inferred)* | on / off | Alpha for other apps |
| Layer Compression | TIFF | enum | RLE *(inferred)* | RLE / ZIP / Discard Layers And Save A Copy | Discard flattens |
| Bit depth (32-bit only) | TIFF | enum | 32 | 16 / 24 / 32 | Label per Help |
| Quality | JPEG | int | 10 *(inferred)* | 0–12 | Higher = better/larger |
| Format Options | JPEG | enum | Baseline | Baseline, Baseline Optimized, Progressive | Progressive needs Optimized |
| Matte | JPEG | color | None | None / color (eyedropper, fg, bg, white, black, other) | Simulates transparency |
| Interlace | PNG | enum | None | None / Interlaced | Larger file when interlaced |
| Row order | GIF | enum | Normal | Normal / Interlaced | After Indexed Color conversion |
| Encoding | EPS | enum | Binary *(inferred)* | ASCII, ASCII85, Binary, JPEG | JPEG for lossy |
| Preview | EPS | enum | TIFF 8-bit *(inferred)* | None / TIFF (1-bit/8-bit) | Cross-platform preview |
| Include Vector Data | EPS | bool | On if vector present | on / off | Rasterized in Photoshop |
| Transparent Whites | EPS | bool | Off | on / off | Bitmap mode only |
| PostScript Color Management | EPS | bool | Off | on / off | Level 3 for CMYK |
| Image Interpolation | EPS | bool | On *(inferred)* | on / off | Smooths preview |
| DCS version | DCS | enum | 2.0 *(inferred)* | 1.0 / 2.0 | 2.0 keeps spot channels |
| Header | Photoshop Raw | int | 0 | ≥0 bytes | Placeholder zeros |
| Interleave | Photoshop Raw | enum | Interleaved *(inferred)* | Interleaved / Non-interleaved | — |
| BMP format / depth | BMP | enum / int | Windows / 24 | Windows, OS/2; 1/4/8/16/24/32 | 32-bit alpha |
| Flip Row Order | BMP | bool | Off | on / off | Bottom-up by default |
| TGA resolution | TGA | enum | 24 *(inferred)* | 16 / 24 / 32 | Pixels per entry |
| TGA Compress (RLE) | TGA | bool | Off | on / off | — |
| Adobe PDF preset | PDF | enum | *(user last)* | High Quality Print, Press Quality, Smallest File Size, PDF/X-1a, PDF/X-3, PDF/X-4, PDF/A-1b, Rich Content, Standard, etc. | Shared across CS apps |
| Preserve Photoshop Editing Capabilities | PDF | bool | On *(inferred)* | on / off | Required for layered Photoshop PDF |
| Compatibility | PDF | enum | Acrobat 6 (PDF 1.5) *(inferred)* | Acrobat 3 (1.3) … Acrobat X (1.7) | Gates features/encryption |
| Standard | PDF | enum | None | None / PDF/X-1a / PDF/X-3 / PDF/X-4 | Compliance target |
| Color Conversion | PDF | enum | No Conversion | No Conversion / Convert To Destination | Spot preserved |
| Downsampling | PDF | enum | Bicubic, 300 ppi / 1200 ppi mono | Do Not Downsample / Average / Subsampling / Bicubic + threshold | Per preset |
| Compression | PDF | enum | ZIP | None / ZIP / JPEG / JPEG2000 | JPEG2000 needs PDF 1.5+ |
| Convert 16 Bit To 8 Bit | PDF | bool | On | on / off | Off ⇒ ZIP only/forced 8-bit below 1.4 |
| Encryption level | PDF | enum | *(from Compatibility)* | 40-bit RC4 / 128-bit RC4 / AES | Presets do not store security |
| Maximize PSD/PSB Compatibility | PSD/PSB | enum | Always *(inferred)* | Always / Ask / Never | File Handling preference |
| Ask Before Saving Layered TIFF Files | TIFF | bool | Off *(inferred)* | on / off | File Handling preference |

## Algorithms & pipeline

### Save dispatch

1. Determine the target: current path/format for **Save**, chosen format for
   **Save As**, or copy target for **Save a Copy**.
2. Build an immutable `SaveRequest { document_revision, path, format, options }`
   from a snapshot (`WF-001` background-safe).
3. Choose the serializer:
   - **PSD/PSB** → `pictura_core::psd` (the only lossless full-document writer);
     append the merged composite when Maximize Compatibility is on.
   - **Raster formats** → `pictura_io::raster` / `pictura_io::tiff` etc., from a
     flattened raster derived from the document (`ARCH-011`).
   - **PDF** → `pictura_io::pdf` (`printpdf`/`lopdf` candidates); layered path
     writes PSD-like layer data when Preserve Photoshop Editing Capabilities is on,
     otherwise a flattened/standard PDF.
   - **EPS/DCS** → PostScript writer; include a TIFF preview and vector data as
     requested.
   - **Photoshop Raw** → byte stream with optional zero header, interleaved or
     planar.
4. Write to a temporary file in the destination directory, `fsync`, atomically
   rename on success (`WF-001`).
5. On success, advance `saved_revision` and clear the dirty flag if the live
   revision equals the snapshot.

### Flattening and feature reduction

- Formats that cannot hold layers flatten visible layers (or merge per format) and
  **discard hidden layers** (Help's flattening note). Alpha/spot/notes are kept
  only if the format supports them and the option is checked.
- The export works from a **snapshot**; it never mutates the in-memory document.
- Bit-depth reduction (16→8 for JPEG/GIF/Save for Web; 32→16/8 for formats that
  cannot hold float) is explicit in the option set.

### ICC embedding

- Native/RASTER: the **ICC Profile / Embed Color Profile** checkbox controls whether
  the working-space (or proof) profile is written. In PSD/PSB it is image resource
  1039; in TIFF/PNG/JPEG the format's ICC tag/chunk; in PDF the Output/Profile
  Inclusion Policy (`ARCH-007`, `ARCH-011`).
- "Use Proof Setup" on Windows embeds the proof setup profile rather than the
  working space.

### Layered TIFF

TIFF layer data uses Photoshop's private TIFF tags; other applications see only
the flattened image. **Layer Compression** chooses RLE/ZIP for layer pixels, and
**Discard Layers And Save A Copy** flattens. The **Ask Before Saving Layered TIFF
Files** preference prompts before writing layers.

### Layered PDF

**Preserve Photoshop Editing Capabilities** stores Photoshop data (layers, alpha,
spot) in the PDF so it reopens in Photoshop CS2+; otherwise a standard PDF is
written and layers are flattened. PDF compatibility level gates live transparency
and layer preservation (PDF 1.3 none; 1.4+ live transparency; 1.5+ layered PDF).

### Version-compatibility enforcement

Saving in an older-version format is a feature-filter operation: collect document
features, intersect with the target version's supported set, warn/omit the
remainder. The check must be data-driven from the format capability matrix in
`ARCH-011` rather than hard-coded per dialog.

## Rust module mapping

Proposals, extending `ARCH-011`:

- `pictura_io::save::SaveFormat` — enum covering PSD/PSB, TIFF, JPEG, PNG, GIF,
  EPS, DCS, Photoshop Raw, BMP, Cineon, Targa, PDF, plus the capabilities flags
  (layers, alpha, spot, notes, ICC, 8/16/32).
- `pictura_io::save::SaveOptions` — per-format option structs:
  `TiffOptions { image_compression, predictor, pixel_order, byte_order, pyramid,
  transparency, layer_compression, bit_depth }`, `JpegOptions { quality, format,
  matte }`, `PngOptions { interlace }`, `GifOptions { interlace }`,
  `EpsOptions { preview, encoding, jpeg_quality, halftone, transfer,
  transparent_whites, ps_color_mgmt, vector_data, interpolation }`,
  `RawOptions { header, interleaved }`, `BmpOptions { os2, depth, rle, flip }`,
  `TgaOptions { depth, rle }`, `PdfOptions { preset, standard, compatibility,
  preserve_photoshop, thumbnails, fast_web, downsampling, compression,
  color_conversion, destination, profile_policy, security }`,
  `PsdOptions { maximize_compatibility }`.
- `pictura_io::save::serialize(req) -> SaveTask` — snapshot serializer with progress
  and atomic write.
- `pictura_io::flatten::flatten_for(format, caps, doc) -> RasterImage` — the shared
  feature-reduction step.
- `pictura_io::tiff::{write_layered, write_flat}` — private-tag layer writer.
- `pictura_io::pdf::{write_layered, write_standard, apply_preset}` — preset table
  and PDF/X validation hints.
- `pictura_io::color::embed_profile(bytes, doc, use_proof) -> Option<Icc>` — ICC
  embedding policy (`ARCH-007`).
- `pictura_shell::save::SaveDialogController` — populates the format list and
  option pane, validates and produces `SaveOptions`.

Types crossing the Qt boundary: `SaveFormat` + capability flags, the per-format
option structs, `SaveRequest/SaveProgress`, and a capability/warning summary for
the dialog's "format cannot hold all features" warning.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `SaveAsDialog` | `QDialog` | Filename, Format combo, file-saving checkboxes, per-format option pane, warning label |
| `SaveOptionsPane` swarm | `QWidget` | `TiffOptionsWidget`, `JpegOptionsWidget`, `PngOptionsWidget`, `GifOptionsWidget`, `EpsOptionsWidget`, `DcsOptionsWidget`, `RawOptionsWidget`, `BmpOptionsWidget`, `TgaOptionsWidget`, `PdfOptionsWidget` |
| `PdfPresetCombo` | `QComboBox` + model | `.joboptions` presets, shared with CS-style preset management |
| `PdfOptionsDialog` | `QDialog` | General / Compression / Output / Security / Summary pages |
| `FormatCapabilityModel` | `QAbstractListModel` | Format list with capability flags and option-pane factory |
| `FeatureLossWarningLabel` | `QLabel`/`QMessageBox` | Warns when the format cannot hold all features |
| `SaveProgressIndicator` | `QProgressBar` | Reused from `WF-001` |

Widgets over QML for the dense option forms; the PDF dialog uses a
`QStackedWidget`/list-page layout. Capability filtering is model-driven so the
option panes stay thin.

## Data-model impact

- **Save settings** are per-document session state (`WF-001`), holding the last
  `SaveFormat` and `SaveOptions`.
- Export never mutates the document; it serializes from a snapshot. The only
  document fields that change are `path`, `format`, `saved_revision`, and `dirty`.
- PSD/PSB write the full layer tree, masks, styles, paths, channels, and image
  resources; each other format consumes a flattened/restricted projection
  (`ARCH-011`).
- ICC embedding writes image resource 1039 (PSD/PSB), the format's ICC slot
  (TIFF/PNG/JPEG), or the PDF output intent/profile policy.
- Layer data in TIFF uses Photoshop private tags; unknown metadata/blocks are
  preserved for native round-trips.
- Undo: Save/Save As/Save a Copy are **not** undoable document operations (they do
  not alter document content). Changing the active format is window state.

## Edge cases

- **Format cannot hold features**: warn at the dialog bottom; PSD/PSB recommended.
- **Save on an untitled document**: routes to Save As.
- **16-bit → JPEG/GIF/Save for Web**: auto-reduce to 8-bit; warn where CS6 does.
- **32-bit float**: only PSD/PSB/OpenEXR/Portable Bitmap/Radiance/TIFF; other
  formats must convert or reject, never silently truncate.
- **CMYK JPEG**: Adobe APP14 conventions; some readers cannot open CMYK JPEG.
- **TIFF JPEG compression**: only opaque 8-bpc RGB/Gray ≤30,000 px; otherwise
  unavailable.
- **Layered TIFF**: other apps see a flattened composite; `Ask Before Saving
  Layered TIFF Files` prompts.
- **PDF 16-bit**: below Acrobat 5 (PDF 1.4) it is forced to 8-bit; JPEG2000
  compression requires PDF 1.5+.
- **PDF security with presets**: passwords are not stored in `.joboptions`; warn.
- **PSD/PSB >2 GB / >30,000 px**: PSB required; TIFF caps at 4 GB; Photoshop Raw
  flattens.
- **Alpha/spot/notes**: silently dropped if the format/option does not carry them;
  surface what is being dropped.
- **Background save + Save As**: the target path/format must be committed only
  when the write succeeds; a failed background Save As leaves the document
  pointing at the old file.
- **Extension vs content on Save As**: the extension follows the chosen format;
  don't let a mismatched extension silently change the codec.
- **Read-only destination / disk full**: atomic temp+rename protects the original;
  report and keep dirty.
- **GPU unavailable**: no impact.

## Parity acceptance criteria

- Given a layered PSD and `File > Save` with the same format, the file is rewritten
  in place and the document stays clean; no format dialog is shown for PSD.
- Given `File > Save As > TIFF` with `Layers` on and Layer Compression = ZIP, the
  saved TIFF reopens in Photoshop with the same layer count and pixel-identical
  layers; another application sees the flattened composite.
- Given `File > Save As > JPEG` at quality Q, the re-encoded image matches a
  reference within the JPEG PSNR tolerance, is 8-bit, and carries EXIF/IPTC/XMP the
  format supports.
- Given a 16-bit RGB document saved as PNG with Interlace = None, the file is
  16-bit and lossless; saved as JPEG it is auto-reduced to 8-bit with a warning.
- Given `File > Save As > Photoshop PDF` with Preserve Photoshop Editing
  Capabilities on, reopening in Kooka Pictura restores the layer tree, alpha, and
  spot colors; with it off, a standard flattened PDF is produced.
- Given PDF Compatibility = Acrobat 3 (PDF 1.3), live transparency is flattened and
  layers are not preserved; at Acrobat 6 (PDF 1.5), layers are preserved.
- Given `File > Save a Copy` with As A Copy on, a copy is written and the open
  document's path and format are unchanged.
- Given a CMYK document saved as Photoshop EPS, the result supports a clipping path
  and no alpha channel, and reopens rasterized.
- Given Maximize Compatibility = Never, the PSD omits the merged composite and is
  smaller than the Always result for the same document.
- Given a document with features a chosen format cannot hold, the dialog shows the
  incompatibility warning before saving.
- Given an ICC-tagged working space and Embed Color Profile on, the saved
  TIFF/PNG/PDF carries an ICC profile matching the working space within ICC
  byte-equality (or a documented tag-level equivalence).

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — the
  CS6 Help corpus, downloaded and text-extracted for this pass. Established: the
  **Save a file** / **Save As** procedures and the format-incompatibility warning;
  the full **File saving options** list (As A Copy, Alpha Channels, Layers, Notes,
  Spot Colors, Use Proof Setup / ICC Profile / Embed Color Profile, Thumbnail,
  Use Lower Case Extension, Image Previews); **Set file saving preferences** and the
  macOS image-preview types; **Save large documents** (PSB / Photoshop Raw / TIFF
  limits); **Choosing a file format** and the 16-bit/32-bit support lists;
  **About file compression** (RLE, LZW, JPEG 0–12, CCITT, ZIP); **Maximize
  compatibility for PSD and PSB** (Always/Ask/Never) and the earlier-version
  feature-discard note; the per-format **File formats** topics (PSD, Photoshop 2.0,
  DCS 1.0/2.0, EPS, Photoshop Raw, DNG, BMP, Cineon, DICOM, GIF, IFF, JPEG, PSB,
  OpenEXR, PCX, PDF, PNG, PBM, Radiance, Scitex CT, Targa, TIFF, WBMP) including
  the EPS mode/alpha/clipping-path support and TIFF 4 GB/layer/transparency/pyramid
  notes; **Saving files in graphics formats** with the option dialogs for TIFF,
  JPEG, PNG, GIF, EPS (and its encoding options), DCS, Photoshop Raw, BMP, Cineon,
  Targa; **Saving PDF files** in full (preset list, PDF/X and PDF/A, compatibility
  table, General options, compression/downsampling, Output/PDF-X options, security,
  preset save/load) and the Photoshop-PDF vs. standard-PDF distinction; the CS6
  **Supported file formats** list.
- `https://html.duckduckgo.com/html/?q=Photoshop+CS6+New+Document+dialog+options+artboard+recent`
  — search-result page (snippet only); used only for the artboard cross-reference in
  `WF-002`, not for save behavior.

Internal cross-references (not external URLs): `docs/01-architecture/file-formats.md`
(per-format matrix and crate proposals), `docs/01-architecture/color-management.md`
(`ARCH-007`), `docs/01-architecture/document-model.md` (`ARCH-008`).

## Open questions

- **Default option values.** Many stated defaults here are *(inferred)* (for
  example TIFF compression LZW, JPEG quality 10 in Save As, PDF compatibility
  Acrobat 6). Resolve by opening each CS6 Save As dialog on a default install and
  recording the pre-filled values.
- **PSD "save as version".** Whether CS6 offers an explicit version target for PSD
  beyond Maximize Compatibility, and how it maps to the format list, is unverified.
- **ICC "Use Proof Setup" semantics on Windows.** The Help label differs from
  macOS ("Embed Color Profile"); whether the Windows path embeds the proof profile
  or the document profile needs confirmation.
- **Layered TIFF tag fidelity.** Which specific Photoshop private TIFF tags CS6
  writes for layers, and whether masks/adjustment layers survive, is deferred to
  TIFF/`ARCH-011` testing against CS6-authored files.
- **PDF layered data form.** Whether Preserve Photoshop Editing Capabilities embeds
  a full PSD-like structure or a restricted subset is not documented. Resolve by
  parsing CS6-authored layered PDFs.
- **DCS option surface.** The DCS Format dialog's exact extra controls (beyond EPS)
  are only summarized by Help; confirm against a CS6 build.
- **BMP/TGA/Cineon exact defaults and option sets** need a CS6 dialog pass.
- **16→8-bit conversion policy.** Whether CS6 always auto-converts or can refuse for
  certain formats (beyond the stated JPEG and Save for Web cases) is unverified.
- **PDF/X preflight behavior.** The exact validation message flow when a document
  fails PDF/X compliance is described only generally; parity requires the exact
  prompt copy.
- **Qt6 details.** The option-pane and PDF-dialog class sketches need verification
  against Qt6 current docs before implementation.
