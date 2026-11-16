# Design

## Context

See `proposal.md` for motivation. The current `File ▸ Save As` path is split
between C++ dialog/dispatch code and a Rust save bridge:

- `crates/pictura-app/cpp/import_save_save.cpp` opens `JpegOptionsDialog`,
  `PngOptionsDialog`, `BmpOptionsDialog` or `TiffOptionsDialog` by suffix, stores
  the accepted `SaveFormatOptions` as JSON in the session, and calls
  `PictureView::save(path)`.
- `crates/pictura-app/src/cxxqt_object/impl_core/save.rs` sends PDF to the PDF
  writer, PSD/PSB to the PSD save job, and PNG/JPG/TIF/BMP to the C++
  `QImageWriter` edge, with WEBP on the existing engine encoder. Any other
  suffix currently falls through to the PSD writer.
- The Qt edge (`crates/pictura-app/cpp/encode_image.cpp`) can set quality,
  compression, optimized and progressive flags, and a matte, but cannot express
  BMP headers/depths, PNG interlace, TIFF planar/endian/pyramid/private tags, or
  the new containers.
- Reusable engine pieces already exist: the PSD layer-and-mask writer
  (`write.rs`/`write_layers.rs`), the TIFF IFD reader/writer used for metadata
  injection (`raster_metadata/tiff.rs`), the GIF encoder and quantizer (`web/`),
  the Indexed conversion that retains a palette and index plane
  (`document_ops/mode/flat.rs`), and the hand-written PDF/PostScript-adjacent
  code in `pdf/`.
- `interop/raster-export` currently forbids raster encoders in the engine. That
  rule was written for Export As; this change scopes it to the export path so
  Save As can own its writers.

## Goals / Non-Goals

**Goals:**

- One Save As dispatch that knows all ten formats, including the shared `.eps`
  suffix.
- Option-faithful files for every control the dialogs present, with writer
  output verified by parsing the container and by an external decoder.
- Keep the engine Qt-free and deterministic; keep JPEG on the existing Qt path
  and WebP on the existing engine path.
- Keep the session format additive, so an older session still loads.
- One atomic write contract for single-file formats and DCS's multi-file set.

**Non-Goals:**

- Export As, Quick Export, Save a Copy, or any other export entry point.
- Named DCS 2.0 spot inks, 16/32-bit native Save As, animated GIF, or
  Photoshop's layered TIFF masks/adjustment layers beyond what the existing PSD
  layer-section writer already produces.
- A worker-thread refactor for raster saves; the new writers run synchronously
  like the current raster path.
- macOS-only Raw file type/creator metadata.

## Decisions

### D1 — Engine writers for Save As, Qt for JPEG, engine for WebP

New Save As writers live in `pictura-codec`; JPEG remains on the C++ Qt edge
and WebP remains on the existing engine encoder (`encode_webp`). TIFF and EPS
need JPEG-compressed payloads, so the codec writer accepts a JPEG-encode
callback that the app implements with the existing Qt edge. That callback is a
payload encoder, not the Export As encode path. This keeps one JPEG
implementation without adding a Rust JPEG encoder.

*Alternatives:* add the `image`/`tiff`/`png` crates (new dependencies, and the
`tiff` crate does not cover planar/endian/SubIFD pyramids or Photoshop private
tags); post-process Qt bytes (collapses into writing an encoder for PNG
interlace and TIFF planar/pyramid). Own writers match the repo's hand-written
PDF, GIF and WebP precedent and keep the engine Qt-free.

### D2 — One dispatch by suffix plus a Save As filter hint

`PictureView::save` resolves a Save As kind from the suffix. Because EPS and DCS
share `.eps`, the C++ Save As flow passes the selected filter's kind as a hint;
the resolved kind is stored on the document (`source_format`-style) and reused
by plain Save. Formats with no writer are refused instead of falling through to
PSD. `PictureView::save`, `saveRunsOnWorker`, `save_start_rust`, and
`document_api::saveStart` share one format set, so a new or unknown kind never
reaches the PSD worker. A missing suffix is refused in the Save As dispatch;
`format_for_path`'s `"psd"` default remains only for `source_format`. Export
paths keep their existing suffix dispatch.

*Alternative:* separate `.dcs` extension (not Photoshop-compatible); singleton
DCS filter merged into EPS (loses the CS6 dialog split).

### D3 — One JSON options payload across the bridge

The C++ dialog result is serialized once (`SaveFormatOptions` already has a JSON
shape) and passed to a single Rust setter as `kind + JSON`. Rust parses it into
a typed per-format `SaveOptions` with defaults for plain Save. The Rust
`SaveRequest` also carries the `Document` (or its mode planes) and the retained
palette/index plane, so Raw/EPS/DCS and Indexed TIFF are written from their mode
data rather than sRGB RGBA. This avoids a 30-argument cxx function and keeps the
existing session store authoritative.

*Alternative:* a shared cxx struct (generated cross-language type for no gain);
one flat setter (unreadable and error-prone).

### D4 — Session keys are additive

The session's `saveFormatOptions` object gains `gif`, `eps`, `dcs`, `raw`,
`tga` and `cineon` entries, plus the new BMP/TIFF fields. Absent keys fall back
to CS6 defaults, so no schema bump or migration is needed.

### D5 — BMP writer with a real Advanced Modes control

The writer owns the Windows/OS/2 headers, depth conversion, RLE4/RLE8, row
order and 16-bit layouts. Sub-24-bit output uses the existing deterministic
quantizer for an RGB document and the retained palette for an Indexed one.
Advanced Modes exposes X1R5G5B5, A1R5G5B5 and R5G6B5; any CS6 option whose exact
effect remains unverified is disabled with a tooltip rather than faked. The OS/2
form is limited to 1, 4, 8 and 24 bits: 16/32-bit OS/2 output, Flip Row Order
and Advanced Modes with OS/2 are refused because the OS/2 core header has an
unsigned height and no bit-fields.

*Alternative:* keep Qt's 24/32-bit BMP and disable depth/RLE (does not satisfy
the issue).

### D6 — PNG writer with Adam7

A small PNG encoder over `flate2` writes the four 8-bit color types, adaptive
row filters, the chosen compression level and Adam7 passes. PNG metadata stays
dropped, matching today's behavior.

*Alternative:* libpng through Qt (not a public Qt dependency); the `png` crate
(new dependency for a small encoder).

### D7 — TIFF writer, pyramid and layered TIFF

The TIFF writer emits classic TIFF, chunky or planar strips. Flat output
honors either byte order; layered output is always big-endian because the PSD
layer payload in tag 37724 is authored big-endian and the container must agree.
`Discard Layers And Save A Copy` restores the chosen order. The writer supports
None/PackBits/LZW/Deflate/JPEG compression, an internal predictor for
LZW/Deflate where it applies, alpha as an extra sample, and SubIFDs for the
pyramid. An
Indexed TIFF also writes the ColorMap tag (320) and the retained palette. JPEG
compression is gated to the documented case: opaque 8-bpc RGB/gray at most
30,000 px; other modes are refused. Layer data is the existing PSD
layer-and-mask section embedded in tag 37724, with the document's layer
compression selecting PackBits or Deflate. `Discard Layers And Save A Copy`
writes a flat file. Metadata injection can run after encoding because the
existing TIFF metadata pass appends a new IFD0 and leaves strip and SubIFD
offsets valid.

*Alternatives:* Qt for flat TIFF and a private rewriter for the rest (two TIFF
readers/writers); the `tiff` crate (not enough control over planar, endian,
SubIFDs and tag 37724).

### D8 — GIF is an Indexed conversion, not a second quantizer

Save As GIF reuses `IndexedColorDialog`, `preview_indexed` and
`convert_to_indexed`. The conversion previews without mutating; the commit
happens only after GIF Options is accepted, so cancelling either dialog leaves
the document untouched. The writer takes the retained palette and index plane
from the converted document and calls the existing `encode_gif`, so the pixels
and palette do not drift. The conversion flattens layers, as CS6's Indexed mode
does; the existing `saveAsNeedsFeatureWarning` predicate is extended to every new
Save As format, so a layered GIF/TGA/EPS/DCS/Raw/Cineon save warns before
flattening.

*Alternative:* quantize the RGB composite at save time (palette would not match
the document's Indexed mode).

### D9 — Mode-aware sampling for Raw, EPS, DCS and Indexed TIFF

PNG/BMP/TGA/GIF consume sRGB RGBA like today. Raw consumes the composite in the
document's mode (gray, RGB, CMYK, Lab) so its channel planes are meaningful.
A Multichannel document is refused because the normalized working document keeps
no Multichannel plate store to write. DCS consumes CMYK; a non-CMYK document is
converted to CMYK for the separations and warned, and extra channels are omitted with a warning because
the model has no ink names. EPS narrows to RGB, grayscale or CMYK; Lab,
Multichannel, Indexed, Duotone and Bitmap sources are converted to one of those
spaces. Indexed TIFF consumes the retained palette and index plane. The
SaveRequest in D3 carries these data.

### D10 — EPS/DCS PostScript with a TIFF preview

The EPS writer emits a DSC header, bounding box, color-space image invocation,
the selected encoding (ASCII/ASCII85/Binary/JPEG), optional halftone/transfer
PostScript, interpolation and vector data, and an optional preview. Previews use
the DOS EPS binary wrapper with a TIFF preview produced by the new TIFF writer,
which the app's own EPS reader already understands. DCS shares this writer:
1.0 writes a master plus four plate files through a set-level temp/rename, 2.0
writes one file with four process-color images; both honor the same
None/1-bit/8-bit TIFF Preview as EPS.

### D11 — Cineon is a fixed 10-bit RGB container

The writer emits the Kodak header and packs three 10-bit channels per 32-bit
word from the document's 8-bit RGB samples with a documented deterministic
mapping. The dialog is deliberately minimal (fixed 10-bit RGB readout) because
the CS6 option surface is not in the docs corpus; the approximation is called
out in the capability and the manual test rows.

*Alternative:* block Cineon until a CS6 reference exists (the issue lists it).

### D12 — Test strategy mirrors the existing layers

Codec tests parse the written container (headers, tags, bitstreams) and use
`magick`/`ghostscript` as an independent decoder when available, skipping when
absent as the existing oracle tests do. Qt Test covers dialog dispatch, defaults,
remembered settings, cancel semantics, and the app-level save glue. Layered
TIFF's Photoshop reopen, DCS in a page-layout application, Cineon in a film tool,
and the CS6 visual comparisons are explicit manual rows in the test plan.

## Risks / Trade-offs

- **TIFF JPEG strip compatibility** → write one complete JFIF stream per strip
  and verify with `magick`, Qt and (manually) Photoshop. If the full-stream form
  is rejected, fall back to abbreviated streams with a JPEGTables tag, or keep
  JPEG-compressed TIFF on the Qt path while the other TIFF options use the new
  writer.
- **Layered TIFF cannot be reopened by the app** → verify tag 37724 structurally
  and parse its layer section with the PSD reader; Photoshop reopen stays manual.
- **TIFF layer-section framing** → mirror the PSD section builder exactly and
  add a round-trip structural test before wiring the dialog.
- **DCS 2.0 cannot carry named spots** → write process CMYK, warn, and keep the
  ink-name model out of this change.
- **EPS option fidelity** (halftone/transfer/PS color management/vector data) →
  standard PostScript emission plus structural and Ghostscript checks; visual
  parity rows stay manual.
- **Cineon and BMP Advanced Modes CS6 defaults are unverified** → documented
  approximations, explicitly disabled where an option cannot be honored.
- **8-bit save ceiling** → Raw/Cineon write 8-bit document samples (Cineon packs
  to 10 bits); native-depth Save As is a separate change.
- **Synchronous encode on large documents** → the new writers run on the same
  path as today's raster save; a worker migration remains follow-up work.
- **File-size limits in one change** → split writers into per-format modules and
  test files so no new file approaches the 800/1400 LOC caps.

## Migration Plan

No user-data migration. The session JSON is additive and older sessions load
with defaults. Unknown suffixes now fail instead of writing PSD bytes; the six
new filters are inserted before the All Formats entry, which stays last, so
existing filter indices stay stable. Rollback is a plain revert: no existing
file format changes on disk, JPEG keeps its Qt path, and WebP keeps its engine
path.

## Open Questions

None blocking. The exact CS6 Advanced Modes and Cineon surfaces are resolved in
favor of the documented approximations above; a CS6 reference can refine them
without changing the specs or task breakdown.
