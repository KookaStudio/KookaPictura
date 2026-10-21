# File Formats

- **Spec ID:** `ARCH-010` (provisional; see `INDEX.md`)
- **Status:** `Draft`
- **Parity tier:** `Core` (PSD/PSB/TIFF/PNG/JPEG/GIF/BMP); `Extended-only` (EXR/HDR/JPEG 2000/RAW/DNG and the print formats, per edition)
- **New in CS6:** `Changed` — CS6 documentation lists enhanced OpenEXR support (multi-part/multi-view, high bit depth) and new audio/video container imports; the core still-image set is unchanged from CS5.
- **Depends on:** `ARCH-008` document-model, `ARCH-007` color-management, `10-workflow-io/open-and-new.md`, `10-workflow-io/save-and-save-as.md`, `10-workflow-io/export-formats.md`, `10-workflow-io/web-export-and-slices.md`, `10-workflow-io/camera-raw-workflow.md`, `10-workflow-io/file-info-and-metadata.md`

> All crate and module names below are **design proposals**. The repository
> contains a working implementation; this file stays the behavioural contract
> for the feature, and `docs/dev/doc-code-parity-gaps.md` records what shipped
> and what did not. The CS6 read/write capabilities in the matrix are
> *(inferred)* from each format's nature and the CS6 support list; they are not
> all confirmed by a parsed CS6 source. See `## Open questions`.

## CS6 behavior

Photoshop CS6 opens and saves a fixed set of still-image formats. The native
formats (PSD/PSB) are the only ones that round-trip the full document model;
every other format is an import/export boundary that flattens or restricts the
document. Camera raw files open through the Adobe Camera Raw plug-in rather than
the document loader.

The read/write matrix below is the contract. "R/W" means Photoshop documents
both directions for the format; "R" means open/import only. Capabilities such as
layer preservation, alpha, and metadata are per-format.

| Format | Ext | Direction (CS6) | Bit depths / modes | Layers / channels / alpha | Metadata | Proposed Rust crate(s) |
|---|---|---|---|---|---|---|
| PSD | `.psd` | R/W | 1/8/16/32; Bitmap, Gray, Indexed, RGB, CMYK, Multichannel, Duotone, Lab | Full layer tree, masks, styles, paths; up to 56 channels incl. alpha/spot | ICC (res 1039), EXIF (1058/1059), XMP (1060), IPTC (1028), paths, slices | read `psd` 0.3.5 (parse-only); write: custom `pictura_core::psd` |
| PSB | `.psb` | R/W | same as PSD; dimensions to 300,000 px | same as PSD; 8-byte length fields | same as PSD | custom `pictura_core::psd` (PSB mode) |
| TIFF | `.tif`, `.tiff` | R/W | 8/16 (32 inferred); RGB, Gray, CMYK, Lab, Indexed | Photoshop layers, alpha/transparency, clipping path | ICC, EXIF, IPTC, XMP, Photoshop tags | `tiff` 0.11 / `image` codec |
| PNG | `.png` | R/W | 8/16; Gray, RGB, Indexed, alpha | single image; alpha | ICC (iCCP); limited text/XMP (write support limited) | `png` 0.18 / `image` codec |
| JPEG | `.jpg`, `.jpeg`, `.jpe` | R/W | 8 only; Gray, RGB, CMYK | no layers; no alpha | ICC, EXIF, IPTC, XMP | decode `zune-jpeg` 0.5 / `jpeg-decoder` 0.3; encode `jpeg-encoder` 0.7.1 (or `mozjpeg`) |
| JPEG 2000 | `.jp2`, `.j2k`, `.jpf`, `.jpx`, `.jpm` | R/W | 8/16 (JPX/JP2); Gray, RGB, Indexed, alpha | single image; alpha | ICC | `jpeg2k` 0.10.1 (OpenJPEG bindings); alternatives `jp2k`, `hayro_jpeg2000` |
| GIF | `.gif` | R/W | 8 indexed (1-bit alpha) | single image or animation frames; 1-bit alpha | none (no ICC) | `gif` 0.14 / `image`; palette via `imagequant` 4.4.1 |
| BMP | `.bmp`, `.dib` | R/W | 1/4/8/24/32 (16 inferred); RGB, Gray, Indexed | single image; 32-bit alpha | minimal; ICC via resource | `image` codec |
| PDF | `.pdf` | R/W (import rasterizes pages; export writes multi-page) | 8/16 container; RGB, Gray, CMYK, Lab, Indexed | open recreates layers from PDF; save can preserve Photoshop layers and PDF/X | ICC, XMP, output intents | `printpdf` 0.12.8 / `lopdf`; read+rasterize via Ghostscript/Poppler (candidate) |
| EPS | `.eps`, `.epsf` | R/W | 8; Gray, RGB, CMYK, Lab, Indexed (as PostScript color) | transparent raster + clipping path (res 2999); no layers | ICC/EXIF via TIFF preview; IPTC/XMP | write `postscript` (candidate); rasterize via Ghostscript |
| RAW (ACR) | per-camera (`.cr2`, `.nef`, …) | R (via Camera Raw plug-in) | camera raw, opens as 8/16-bit RGB in a working space | no layers until opened | EXIF, IPTC, XMP sidecar | `rawler` 0.7.2 / `rawloader`; `libraw-rs` 0.0.4 (`rsraw`, `rawlib`) |
| DNG | `.dng` | R/W (convert) | camera raw / 8/16 RGB after demosaic; linear or non-linear | no layers; single image | EXIF, IPTC, XMP | `rawler` / `libraw-rs` (DNG is TIFF/EP-based) |
| PBM/PGM/PPM/PAM | `.pbm`, `.pgm`, `.ppm`, `.pam` | R (write inferred off) | 1 (PBM), 8/16 (PGM/PPM) Gray/RGB | single image; no alpha (PAM supports) | none | `image` `pnm` codec |
| IFF (ILBM) | `.iff`, `.lbm`, `.ilbm` | R (write inferred off) | 1–8 bitplanes, indexed; Amiga screen modes | single image; masked/alpha | none | `iff` (candidate); `oxideav-iff`; `iff_rs` |
| TGA | `.tga`, `.vda`, `.icb`, `.vst` | R/W | 8/16/24/32; Gray, Indexed, RGB, alpha | single image; 1-bit/8-bit alpha | none/simple footer | `image` `tga` codec |
| OpenEXR | `.exr` | R/W | 16 half / 32 float; RGB, Gray, alpha | multi-part/multi-view (CS6); alpha | EXIF-ish attributes; ICC not native | `exr` (used by `image`) |
| Radiance HDR | `.hdr`, `.rgbe` | R/W (write inferred) | 32-bit float RGB (RGBE) | single image | none | `image` `hdr` codec |

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| File > Open / Open As | Dialog | `Ctrl/Cmd+O`, `Ctrl/Cmd+Alt+Shift+O` | Format filter; routes RAW to Camera Raw; `Open As` overrides the extension |
| File > Open Recent | Submenu | n/a | Recently opened documents |
| File > Camera Raw | Dialog | `Ctrl/Cmd+Shift+Alt+O` (inferred) | ACR develop settings; opens to an RGB document |
| File > Save | Command | `Ctrl/Cmd+S` | Native PSD/PSB, or last save format |
| File > Save As | Dialog | `Ctrl/Cmd+Shift+S` | Format combo + per-format options |
| File > Save a Copy | Dialog | n/a | Flattened copy; does not change the document's format |
| File > Save for Web | Dialog | `Ctrl/Cmd+Shift+Alt+S` | GIF/JPEG/PNG-8/PNG-24 with palette and quality options |
| File > Import | Submenu | n/a | PDF image, annotations, WIA/scan (OS-dependent) |
| File > Export > Zoomify | Submenu | n/a | Tile export |
| File > Place | Command | n/a | Places PDF/EPS/AI/SVG as smart objects (rasterizes as needed) |
| File > File Info | Dialog | `Ctrl/Cmd+Alt+Shift+I` | EXIF/IPTC/XMP carriage per format |
| Format Options pane | Dialog section | n/a | Rebuilt per selected save format |

## Parameters & ranges

Format-specific save/open controls. Ranges marked *(inferred)* are unconfirmed
for CS6.

| Control | Format | Type | Default | Range / options | Notes |
|---|---|---|---|---|---|
| Maximize Compatibility | PSD/PSB | bool | On | on / off | Writes merged composite so other apps can read |
| Compression | TIFF | enum | LZW | None, RLE, LZW, ZIP, JPEG | JPEG compression loses data |
| Save Image Pyramid | TIFF | bool | Off | on / off | Multi-resolution pyramid |
| Save Transparency | TIFF | bool | On | on / off | Alpha channel |
| Interleave | TIFF | enum | n/a | IBM PC / Macintosh | Byte order |
| Layers | TIFF | bool | Off | on / off | Preserve layers (inferred for CS6) |
| Bit depth | TIFF/PNG/TGA/EXR | enum | document | format-dependent | 8/16, 24/32, half/float |
| Compress (PNG) | PNG | enum | None | None, TIFF/PNG-8/PNG-24, interlaced | Save for Web palette options |
| Quality | JPEG | int | 10 (Save As), 60 (Save for Web) | 0–12 / 0–100 | Baseline vs progressive |
| Format | JPEG | enum | Baseline | Baseline, Baseline Optimized, Progressive | — |
| Subsampling | JPEG | enum | 4:2:0 | 4:4:4, 4:2:2, 4:2:0 | Save for Web |
| Quality/Compression | JPEG 2000 | float | 0.5 (inferred) | file size vs quality sliders | — |
| Colors | GIF/BMP | int | 256 | 2–256 | Indexed palettes |
| Dither | GIF | enum | Diffusion | None, Diffusion, Pattern, Noise | — |
| Transparency | GIF | bool | On | on / off | Matte color for edges |
| BMP encoding | BMP | enum | Windows | Windows / OS/2; RLE variants | 32-bit alpha (Windows) |
| TGA resolution | TGA | enum | 32 | 16/24/32 bits per pixel | RLE toggle |
| TGA origin | TGA | enum | n/a | bottom-left / top-left | — |
| Preserve Photoshop Editing Capabilities | PDF | bool | On (inferred) | on / off | Keep layers for round-trip |
| PDF preset / standard | PDF | enum | — | PDF/X-1a, PDF/X-3, PDF/X-4, High Quality Print, etc. | Version/preset |
| Color handling | PDF/EPS | enum | n/a | CMYK/RGB/Gray as-is or convert | — |
| Anti-alias / preview | EPS | enum | n/a | None, TIFF 1/8 bit, JPEG | Preview in other apps |
| Camera Raw workflow | RAW/DNG | enum | — | Open as 8/16-bit, working space, resize | ACR settings |

## Algorithms & pipeline

### PSD/PSB parser and serializer

The native format is specified in `ARCH-008` and the Adobe File Formats
Specification. Key loading concerns for this spec:

- **Section order**: header → color mode data → image resources → layer and mask
  information → image data. Length fields are authoritative; unknown bytes are
  skipped or preserved.
- **Compression codes** for channel data: `0` raw, `1` RLE/PackBits (scanline
  byte counts; 2-byte in PSD, 4-byte in PSB), `2` ZIP without prediction, `3`
  ZIP with prediction. Write RLE by default (Photoshop-compatible) and support
  ZIP on read.
- **Big-endian** byte order on every platform.
- **Opaque preservation**: image-resource blocks (IDs listed in `ARCH-008`),
  unknown additional-layer-information keys, and the duotone spec must be
  retained verbatim so a re-save is lossless even where the engine has no
  semantics.
- **Merged composite** in the image-data section; absent when Maximize
  Compatibility is off.

### TIFF

Photoshop's TIFF is TIFF 6.0 plus private tags. Must handle: multiple
photometric interpretations (RGB, CMYK, Lab, Gray, Indexed, transparency mask),
multiple strips/tiles, LZW/ZIP/JPEG/RLE/PackBits compression, planar vs
interleaved, 8/16-bit samples (32-bit float marked inferred), associated vs
unassociated alpha, ICC/EXIF/IPTC/XMP tags, and Photoshop-specific tags for
layers and resolution. The PSD spec's "Other Document File Formats" chapter
documents Photoshop-specific TIFF tags.

### ZIP-with-prediction

For PSD compression code `3`, the predictor is applied per row before deflate
and inverted after inflate. The exact predictor convention must match
Photoshop's; verify against reference files because this is a common
interoperability failure.

### Metadata extraction

- **EXIF** is parsed with `kamadak-exif` 0.6.1, which reads Exif from TIFF and
  TIFF-based RAW, JPEG, HEIF/HEIC/AVIF, PNG, and WebP.
- **XMP** is an RDF/XML packet; parse/write with an XML crate (for example
  `quick-xml`, candidate) and retain unknown properties.
- **IPTC (IPTC-NAA / IIM)** is stored in PSD image resource 1028 and in
  JPEG/TIFF APP13/IRB; parse and preserve as opaque records when not editing.
- **Color**: ICC bytes per `ARCH-007`.

### Camera RAW and DNG

RAW is not a pixel format: a raw decoder must extract CFA data and metadata, then
a pipeline (demosaic, white balance, color matrix, tone) converts to RGB. CS6
delegates this to Camera Raw. For Kooka Pictura, `rawler`/`libraw-rs` are
candidates for CFA extraction (`rawler::RawImage` exposes image data plus CFA
config and black level); the demosaic/develop pipeline is a separate spec
(`10-workflow-io/camera-raw-workflow.md`). DNG is a TIFF/EP-based container, so
the TIFF layer plus a DNG tag reader is a viable route.

### Vector formats (PDF/EPS)

PDF open rasterizes pages via a renderer; PDF save writes the flattened (or
layer-preserved) image plus text/vector where supported. EPS is PostScript;
useful EPS features are the embedded TIFF/JPEG preview, the clipping path
(PSD image resource 2999 naming, `ARCH-008`), and Photoshop's EPS options.
Rasterizing PostScript reliably on Linux implies an external engine
(Ghostscript) unless a pure-Rust interpreter is adopted; this is a design
decision, not a sourced CS6 fact.

### HDR formats (OpenEXR, Radiance)

Both are float RGB. OpenEXR should use the `exr` crate. Radiance HDR uses RGBE
encoding (`image::codecs::hdr`). Open in 32-bit mode; document color management
(`ARCH-007`) applies.

## Rust module mapping

Proposals. A dispatcher selects a codec by signature (not extension) and returns
a common `DecodedImage`/`EncodeRequest`.

- `pictura_io::detect` — signature sniffing (`image::guess_format`, plus custom
  PSD/PSB `8BPS` and PDF `%PDF` checks).
- `pictura_io::psd` — the only format that maps to/from the full document
  model; thin wrapper over `pictura_core::psd`.
- `pictura_io::raster` — a uniform interface over `image` codecs (PNG, JPEG,
  GIF, BMP, TIFF, PNM, TGA, HDR, EXR) and specialized encoders where quality
  matters (`jpeg-encoder`, `imagequant`).
- `pictura_io::tiff` — TIFF options (compression, layers, depth) over `tiff`.
- `pictura_io::jpeg2000` — `jpeg2k` wrapper.
- `pictura_io::pdf` — write via `printpdf`/`lopdf`; import via an external
  renderer adapter (Ghostscript/Poppler).
- `pictura_io::eps` — write PostScript directly or via a `postscript` crate
  (candidate); import via the same external renderer adapter.
- `pictura_io::raw` — `rawler`/`libraw-rs` adapter producing CFA + metadata.
- `pictura_io::iff` — `iff`/`oxideav-iff` (candidate) ILBM reader.
- `pictura_io::metadata` — EXIF (`kamadak-exif`), XMP (XML), IPTC (opaque).
- `pictura_io::color` — ICC embed/extract via `pictura_color`.

Types crossing boundaries: `DecodedImage { width, height, depth: BitDepth,
mode: ColorMode, planes: PlaneData, metadata: Metadata, profile: Option<Vec<u8>> }`
and `EncodeRequest` with per-format option enums. Raw returns
`RawImage { cfa, black_level, white_level, camera_wb, metadata }` instead.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `OpenDialog` | `QFileDialog` | Format filter from the matrix; "Camera Raw" routing |
| `SaveAsDialog` | `QDialog` | Format combo, per-format option pane, "Embed Color Profile", "Use Proof Setup / ICC" |
| `TiffOptionsWidget` | `QWidget` | Compression, byte order, layers, transparency |
| `JpegOptionsWidget` | `QWidget` | Quality, format, subsampling, preview size |
| `GifOptionsWidget` | `QWidget` | Palette, dither, transparency, matte |
| `PdfOptionsWidget` | `QWidget` | Preset/standard, layers, compression, output intent |
| `CameraRawDialog` | `QDialog` | ACR-style develop settings before opening |
| `MetadataDialog` | `QDialog` | EXIF/IPTC/XMP pages (`ARCH-008`) |
| `FileFormatModel` | `QAbstractListModel` | Extensions, direction flags, and option factories |

Directory placement and dialogs are shared with `02-ui-ux` and
`10-workflow-io/save-and-save-as.md`.

## Data-model impact

- Importing a non-native format creates a document in a compatible color mode
  and bit depth and, where the format carries them, populates metadata, ICC
  profile, alpha channels (as alpha or transparency), paths (TIFF clipping
  path), and animation frames (GIF).
- Exporting a non-native format flattens or restricts per the matrix. The export
  must not mutate the in-memory document; it works from a snapshot.
- The file-format choice and per-format options are persisted per document in a
  non-serialized "save settings" record (not written to the file).
- Unknown metadata and unknown PSD blocks are preserved for lossless native
  round-trip.
- PSD/PSB save uses the Document serializer in `ARCH-008`; bit-depth and mode are
  document fields, not format options.

## Edge cases

- **Extension vs content** — detect by signature; a `.jpg` that is a PSD must
  open correctly, and a mislabeled file must not be trusted.
- **PSB > 30,000 px** — 8-byte length fields; TIFF/PNG/JPEG size limits differ.
- **CMYK JPEG** — 4-component JPEG uses Adobe APP14 conventions; handle
  inverted-CMYK quirk on read.
- **16-bit PNG** — supported; 16-bit GIF/JPEG are not. Convert or reject per
  format rather than silently truncating.
- **PNG palette + alpha** — indexed with transparency; quantization on save via
  `imagequant`.
- **GIF animation** — multiple frames and delays; document model support is out
  of core scope (timeline spec).
- **TIFF tile/strip variants** — both must be read; write strips for
  compatibility.
- **TIFF CMYK + ICC** — honor the embedded profile; do not assume a default.
- **PostScript without a renderer** — EPS/PDF import fails gracefully; do not
  crash on missing Ghostscript.
- **RAW without a decoder match** — camera unsupported: surface the camera model
  and offer to open as generic TIFF if applicable.
- **DNG variants** — linear DNG has demosaiced channels; non-linear carries CFA.
- **OpenEXR deep/multi-part data** — CS6 added multi-part/multi-view; the `exr`
  crate may not expose every feature; degrade gracefully.
- **Huge files** — stream/tile; avoid loading a multi-GB TIFF into one
  allocation.
- **Metadata round-trip** — preserve unknown EXIF/IPTC/XMP fields byte-for-byte
  where possible; note that not all formats can carry all metadata.
- **Endianness** — TIFF and PSD byte order and PSB 8-byte fields.

## Parity acceptance criteria

- Given a PSD saved by CS6 with layers, masks, styles, paths, ICC, EXIF, XMP, and
  IPTC, opening and re-saving in Kooka Pictura preserves all fields and unknown
  blocks; the composite is visually identical within tolerance.
- Given a PSB with dimensions above 30,000 px, open/save round-trips without
  truncation.
- Given RLE, ZIP, and ZIP-with-prediction PSD layer channels, decode matches the
  raw channel data bit-exactly.
- Given a TIFF with LZW, ZIP, JPEG, and RLE compression, 8- and 16-bit samples,
  and CMYK/RGB/Lab/Gray photometric modes, each decodes correctly and the ICC
  profile is honored.
- Given a PNG at 8- and 16-bit with and without alpha, decode/encode round-trips
  losslessly and preserves the iCCP profile when present.
- Given a baseline, progressive, and CMYK JPEG, decode succeeds and metadata is
  extracted; re-encode at quality Q matches a reference within the defined PSNR
  tolerance.
- Given a GIF with 256 colors and transparency, decode yields the correct
  palette and frame; save with `imagequant` meets the palette tolerance.
- Given a 32-bit EXR and a Radiance HDR file, each opens in 32-bit mode with
  correct float values within 1 ULP after round-trip.
- Given a JPEG 2000 codestream (`.j2k`) and JP2 container, decode succeeds and an
  encode/decode round-trip meets the quality tolerance.
- Given a Camera Raw file supported by ACR 7, the raw adapter extracts CFA data
  and the core metadata (camera, ISO, exposure) without error.
- Given a DNG, both linear and non-linear variants are detected and handled.
- Given a PBM/PGM/PPM, IFF/ILBM, and TGA file, open succeeds; TGA re-saves.
- Given metadata in any supported format, File Info displays EXIF/IPTC/XMP and a
  save preserves fields the format can carry.
- Given a mislabeled file (extension != content), the correct decoder is chosen
  by signature.
- Given Ghostscript is absent, attempting to open a PDF/EPS reports a clear error
  and does not crash.

## Sources

Fetched for this document:

- `https://web.archive.org/web/2023id_/https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/`
  — PSD/PSB structure, header modes/depths, image resource IDs, compression
  codes, layer records, EPS and TIFF chapters, path/caption/clipping resources.
  Primary source for PSD/PSB/PSD-TIFF-EPS behavior.
- `https://docs.rs/image/latest/image/codecs/index.html` — `image` 0.25.10
  supported formats and codecs: BMP, DDS (decode), OpenEXR, Farbfeld, GIF,
  Radiance HDR, ICO, JPEG, PNG, PNM (pbm/pgm/ppm/pam), QOI, TGA, TIFF, WebP;
  names the underlying crates (`png`, `tiff`, `gif`, `exr`, `zune-jpeg`,
  `qoi`, `image-webp`, `moxcms`). Establishes the Rust raster-codec baseline.
- `https://docs.rs/image/latest/image/` — `image` 0.25.10 API and dependency
  versions (for example `tiff ^0.11.2`, `png ^0.18.0`, `gif ^0.14.0`,
  `zune-jpeg ^0.5.5`, `exr ^1.74.0`, `qoi ^0.4`).
- `https://crates.io/api/v1/crates/psd` — `psd` 0.3.5, "A Rust API for parsing
  and working with PSD files" (read-only), repository
  `github.com/chinedufn/psd`.
- `https://docs.rs/crate/jpeg-encoder/latest` — `jpeg-encoder` 0.7.1: baseline
  and progressive, chroma subsampling, custom quantization tables, 1/3/4
  component colorspaces.
- `https://docs.rs/crate/kamadak-exif/latest` — `kamadak-exif` 0.6.1 (crate
  name `exif`): EXIF parsing from TIFF/RAW, JPEG, HEIF/HEIC/AVIF, PNG, WebP.
- `https://docs.rs/crate/imagequant/latest` — `imagequant` 4.4.1: RGBA→palette
  quantization for PNG/GIF.
- `https://docs.rs/crate/printpdf/latest` — `printpdf` 0.12.8: read/write PDF,
  images, layers, SVG, fonts; built on `lopdf`.
- `https://docs.rs/jpeg2k` (search result) and `https://docs.rs/openjpeg-sys` —
  `jpeg2k` is a safe wrapper over `openjpeg-sys` for loading/saving JPEG 2000;
  alternatives `jp2k`, `openjp2`, `hayro_jpeg2000`.
- `https://docs.rs/rawler` — `rawler` 0.7.2: extract raw image data and metadata
  from camera raw formats (`RawImage`, CFA config, black level).
- `https://docs.rs/libraw-rs` — `libraw-rs` 0.0.4: Rust bindings to the LibRaw
  C/C++ API; alternatives `rsraw`, `rawlib`.
- `https://docs.rs/iff` — `iff` crate for IFF containers; alternatives
  `iff_rs`, `oxideav-iff`.
- `https://bpb-us-w2.wpmucdn.com/wonecks.net/dist/3/535/files/2020/03/Supported-file-formats-in-Photoshop.pdf`
  — a mirror of Adobe's "Supported file formats in Photoshop" page (the version
  with an asterisk marking formats introduced in CS6). The fetch returned raw
  PDF bytes that could not be parsed in this pass; only its identity as the CS6
  support list is used.

Not parsed in this pass: `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf`
(official CS6 Help; fetch exceeded the 5 MB limit). The current Adobe
"supported file formats" help page was seen only as a search result and is not
cited as a source.

## Open questions

- **CS6 read/write matrix.** The direction and capability cells are largely
  *(inferred)*. Resolve by parsing the CS6 support-list PDF above or the CS6 Help
  PDF, and by testing CS6 Save As/open dialogs.
- **TIFF capabilities.** Whether CS6 saves layers, 32-bit float, and which
  compression schemes by default is unconfirmed. Resolve from the CS6 Help PDF
  and CS6-made TIFFs.
- **PDF import/export fidelity.** Which PDF features survive Photoshop's open
  (layers, vector, text) and which export presets exist is not sourced. Resolve
  from the CS6 Help PDF (`Save As PDF`).
- **EPS generation and rasterization.** The exact EPS options and the
  PostScript-interpreter strategy on Linux (Ghostscript vs a pure-Rust engine)
  are open. Resolve with a spike and licensing review.
- **JPEG 2000 options and depth.** CS6's J2K quality controls and 16-bit support
  are unconfirmed. Resolve from the CS6 Help PDF.
- **OpenEXR write support.** Whether CS6 saves EXR (versus read-only) and which
  channels/compressions it emits is unconfirmed. Resolve from the CS6 Help PDF.
- **Radiance HDR write support.** Same question as OpenEXR. Resolve from the CS6
  Help PDF.
- **PBM/PGM/PPM and IFF write support.** Whether Photoshop offers a Save As for
  these is unconfirmed; the matrix assumes read-only. Resolve from the CS6 Help
  PDF.
- **BMP and TGA option sets.** Exact depth/alpha/RLE options per CS6 need
  confirmation.
- **Metadata carriage per format.** Which of EXIF/IPTC/XMP each format can carry
  in CS6, and what is stripped, needs confirmation.
- **Camera Raw coverage.** The last ACR for CS6 is reported as 9.1.1 in
  community sources; the exact camera list and DNG conversion options are not
  sourced. Resolve in `10-workflow-io/camera-raw-workflow.md`.
- **Crate gaps.** There is no mature Rust PSD *writer* and no mature pure-Rust
  PostScript interpreter; both are build-vs-bind decisions. Resolve in
  `00-overview/feasibility-and-non-goals.md` and `licensing-and-provenance.md`.
