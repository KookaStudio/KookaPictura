# Proposal

## Why

`File ▸ Save As` presents the CS6 options dialogs for JPEG, PNG, BMP and TIFF,
but every control Qt's `QImageWriter` cannot express is shown disabled: the BMP
format, depth, RLE and flip controls, PNG interlace, and TIFF RLE/ZIP, pixel
order, byte order, pyramid and layer compression. Six CS6 Save As formats — GIF,
EPS, DCS 1.0/2.0, Photoshop Raw, TGA and Cineon — have no filter, dialog or
writer at all, and a typed unknown suffix falls through to the PSD writer. The
parity register records this as `WF-1`/`WF-4` (`docs/dev/doc-code-parity-gaps.md`)
and ROADMAP tracks it as #625 and the GIF half of #628. The previous
saved-options change (`2026-10-26-save-format-options`) deliberately shipped the
controls disabled and deferred the encoder work to this issue; this change
completes that surface.

## What Changes

- **Engine writers for Save As.** Add `pictura-codec` writers for BMP, PNG,
  TIFF, GIF, TGA, Photoshop Raw, Cineon, EPS and DCS 1.0/2.0. JPEG stays on
  Qt; WebP stays on the existing engine encoder. The engine writers receive
  JPEG bytes through an app-supplied callback for TIFF and EPS compression;
  that callback is a payload encoder, not the Export As encode path. Writes stay
  atomic (temp file plus rename); DCS 1.0 writes its plate set as a unit.
- **Honor every disabled control.** BMP file format (Windows/OS/2), depth
  1/4/8/16/24/32, RLE4/RLE8, Flip Row Order and Advanced Modes 16-bit layouts;
  PNG Adam7 interlace; TIFF None/RLE/LZW/ZIP/JPEG compression, interleaved or
  planar pixel order, IBM PC/Macintosh byte order, Save Image Pyramid, Save
  Transparency, and Layer Compression for layered TIFF (Photoshop tag 37724
  reusing the PSD layer section).
- **Six new Save As formats.** Add `CompuServe GIF`, `Photoshop EPS`,
  `Photoshop DCS`, `Photoshop Raw`, `Targa` and `Cineon` filters, dialogs and
  session-remembered settings. GIF first runs the existing Indexed Color
  conversion and then writes the retained index plane with the existing GIF
  encoder; DCS shares the EPS PostScript writer and writes CMYK separations.
- **Dispatch and refusals.** Save dispatch resolves the format from the suffix
  plus the Save As filter hint (EPS and DCS share `.eps`), records the resolved
  kind on the document for plain Save, and refuses an unknown suffix instead of
  writing PSD bytes. `PictureView::save`, `saveRunsOnWorker`,
  `save_start_rust`, and `document_api::saveStart` share one format set, so a
  new or unknown suffix never reaches the PSD worker.
- **Mode-aware save request.** The app hands the writers a SaveRequest carrying
  the document (or its mode planes) plus the retained palette and index plane,
  so Raw, EPS, DCS and Indexed TIFF receive meaningful channels rather than
  sRGB RGBA.
- **Spec scope correction.** `interop/raster-export`'s "MUST NOT add a raster
  encoder to the engine crates" rule is scoped to the Export As/Quick Export
  path; Save As encoders are governed by the new `codec/raster-save-encoders`
  capability, and the Save As JPEG payload callback is not the Export As path.
- **Ceilings kept honest.** DCS 2.0 writes process CMYK only: the document model
  has no spot-channel ink names, so extra channels are omitted with the existing
  feature-loss warning rather than mislabelled. Cineon gets a fixed 10-bit RGB
  dialog (its CS6 surface is unverified); Raw and Cineon export 8-bit samples
  because the app has no native-depth save path. Every remaining disabled
  control carries a tooltip naming the ceiling.
- **Out of scope.** Save a Copy (the remaining half of #628), named DCS 2.0 spot
  inks, 16/32-bit native save, macOS-only Raw file type/creator, and any change
  to Export As or Quick Export behavior.

## Capabilities

### New Capabilities

- `codec/raster-save-encoders`: byte-level Save As writers for BMP, PNG, TIFF,
  GIF, TGA, Photoshop Raw, Cineon, EPS and DCS, their honored options, and their
  deterministic output guarantees.

### Modified Capabilities

- `interop/save-format-options`: the dialog/format enumeration grows to ten
  formats, the honored-options requirement replaces the Qt ceiling, and GIF's
  Indexed-first flow, DCS's file set, and the unknown-suffix refusal become
  requirements.
- `interop/raster-export`: the Qt-boundary encoding requirement is scoped to
  Export As and Quick Export so engine Save As writers do not contradict it.

## Impact

- **Engine (`pictura-codec`):** new `save/` writer modules, a reusable PSD
  layer-section builder for layered TIFF, and writer tests/oracles. No new
  runtime dependency: `flate2` already covers PNG/ZIP, the GIF encoder and
  quantizer already exist, and the TIFF IFD/metadata helpers are extended.
- **App (Rust):** the save bridge carries a typed `SaveOptions` parsed from the
  dialog JSON (`serde_json`, already in `Cargo.lock`) and a `SaveRequest` that
  carries the document/mode planes plus the retained palette and index plane;
  `PictureView::save` dispatches the new writers, and WebP keeps the existing
  engine encoder.
- **App (C++):** six new dialogs, a BMP Advanced Modes sub-dialog, enabled
  existing controls, the Save As filter list/index map (new filters inserted
  before All Formats, which stays last), DCS file-set handling, and Indexed
  Color sequencing for GIF.
- **Specs/docs:** this change; `docs/dev/doc-code-parity-gaps.md` `WF-1`/`WF-4`,
  `ROADMAP.md` #625/#628 item, and `docs/dev/STATE.md`.
- **Tests:** codec unit/oracle tests (`magick`, `ghostscript`), Qt dialog and
  save-path tests, and a test-plan row for every scenario. Photoshop layered-TIFF
  reopen, DCS page-layout interop, Cineon film-tool interop, and the CS6 visual
  comparisons remain manual rows.
- **Related issues:** closes the remaining scope of #625 and the GIF Save As
  part of #628; #628's Save a Copy half stays open.