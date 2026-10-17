# Proposal: photoshop-pdf-writer

## Why

Issue #626 (parity register row WF-2, from the #486 audit): no PDF writer
exists. `docs/10-workflow-io/save-and-save-as.md` *Save in Photoshop PDF format*
specifies `File > Save As > Photoshop PDF`, its Save options, the **Save Adobe
PDF** dialog (Preset, General, Compression, Output, Security, Summary), the
PDF/X and PDF/A standards, and `.joboptions` presets managed from
`Edit > Adobe PDF Presets`. None of it is built: Save As offers PSD, PSB, and
five raster formats, and `Edit > PDF Presets…` is an inert menu leaf.

## What Changes

- **Engine writer** `pictura_codec::pdf` (Qt-free): a single-page PDF holding
  the document composite as one image, sized from the document resolution, in
  the colour space its mode maps to (Gray, RGB, CMYK, Lab, Indexed, 1-bit
  Bitmap with CCITT Group 4), with ICC tagging, soft-mask transparency,
  Flate/JPEG compression, Average/Subsampling/Bicubic downsampling, 16-bit
  output, page thumbnails, linearization (Fast Web View), notes as PDF text
  annotations, XMP metadata, PDF/X-1a/X-3/X-4 and PDF/A-1b conformance, and the
  standard security handler (RC4-40, RC4-128, AES-128, AES-256).
- **Type layers as live text**: a type layer nothing covers is set over the
  image in its embedded font (`Type0`, `FontFile2`/`FontFile3`, `ToUnicode`),
  so PDF readers select and search it and editors retype it; other type stays
  in the image.
- **Preserve Photoshop Editing Capabilities** embeds the document's own PSD
  (`write_psd`) in a private page stream. Adobe's layered-PDF layout is
  unpublished, so this is an approximation: Pictura reopens its own Photoshop
  PDFs with their layers; Photoshop is not claimed to.
- **Options** are one value type (`PdfOptions`) serialized as Distiller-style
  `.joboptions` text, which is also the preset file format. Compatibility and
  standard gating (what a level forces or forbids) live in the engine and drive
  the dialog's enabled state.
- **Colour**: a bundled synthetic CMYK profile (*Photocraft Coated CMYK*, CC0,
  imported from the PhotoCraft project) is the CMYK destination and the
  PDF/X output intent; `pictura-color` gains a cross-space conversion.
- **App**: Save As lists *Photoshop PDF*; choosing it opens the Save Adobe PDF
  dialog; the document's path becomes the `.pdf`; Save on a `.pdf` re-opens the
  dialog with that document's last settings. `Edit > Adobe PDF Presets…`
  opens a preset manager (New / Edit / Delete / Load / Save As). Opening a
  `.pdf` that carries Pictura's embedded PSD restores the layered document
  (needed for Revert and reopen now that the path becomes `.pdf`); arbitrary PDF
  import stays with #627.
- **CI** installs `poppler-utils`, `ghostscript`, and `qpdf` so the PDF oracles
  (render compare, structure, linearization, decryption) run instead of
  skipping.
- **New dependencies** (RustCrypto, MIT/Apache-2.0): `md-5` 0.10, `aes` 0.8,
  `sha2` 0.10. The standard security handler needs MD5, AES, and
  SHA-256/384/512; audited primitives are preferred to hand-rolled ones (owner
  decision on #626). RC4 and the CBC chaining are a few lines over those
  primitives, so the `rc4` and `cbc` crates are not added.

## Capabilities

### New Capabilities

- `interop/pdf-export`: Photoshop PDF as a save format — the writer's content,
  colour, compression, downsampling, depth, transparency, metadata, standards,
  security, presets, the Save Adobe PDF dialog, the preset manager, and the
  reopen of Pictura's own Photoshop PDFs.

## Impact

- `crates/pictura-codec/src/pdf/` (new module tree) and `lib.rs`; `Cargo.toml`
  gains the five RustCrypto crates.
- `crates/pictura-color/src/lib.rs`: `convert_space`, `Profile::coated_cmyk`,
  `Profile::gray_gamma22`; `assets/icc/photocraft-coated-cmyk.icc` with its
  `assets/PROVENANCE.md` row.
- `crates/pictura-app/src/cxxqt_object/pdf_export.rs` (new bridge), one routing
  line each in `impl_core.rs` `save` and `cxxqt_object.rs`; `cpp/encode_image.*`
  gains a JPEG-to-bytes edge.
- `crates/pictura-app/cpp/pdf_save_dialog.*`, `pdf_presets_dialog.*` (new),
  `dialogs.cpp` (format filter), `frame_menus*.cpp` (Save As routing, Save,
  Edit menu), `frame_open.cpp` (reopen), `session.*` (last options),
  `CMakeLists.txt`.
- Tests: Rust unit and oracle tests under `pictura-codec`; `tst_save_pdf` Qt
  suite.
- `.github/workflows/ci.yml` (apt packages); `docs/dev/doc-code-parity-gaps.md`
  WF-2 and `docs/dev/STATE.md` (`TASK-ALLOWS-DOCS`).
