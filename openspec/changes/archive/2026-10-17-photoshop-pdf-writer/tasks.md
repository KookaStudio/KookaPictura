# Tasks: photoshop-pdf-writer

## 0. Decisions (owner, on #626)

- [x] 0.1 Scope: the whole spec section except JPEG2000 encoding (no encoder).
- [x] 0.2 Preserve Editing embeds Pictura's own PSD, marked as an approximation.
- [x] 0.3 A PDF save makes `.pdf` the document path.
- [x] 0.4 CI installs `poppler-utils`, `ghostscript`, `qpdf`.
- [x] 0.5 Security uses the RustCrypto crates `md-5`, `aes`, `sha2` (RC4 and CBC
  chaining are written over them; `rc4`/`cbc` not needed).
- [x] 0.6 CMYK destination: PhotoCraft's synthetic CC0 coated CMYK profile.

## 1. Colour

- [x] 1.1 `assets/icc/photocraft-coated-cmyk.icc` + `assets/PROVENANCE.md` row.
- [x] 1.2 `pictura-color`: `Profile::coated_cmyk`, `Profile::gray_gamma22`,
  `Profile::channels`, `convert_space` (1/3/4 ↔ 1/3/4 channels, 8/16-bit).

## 2. Engine (`pictura_codec::pdf`)

- [x] 2.1 `object.rs`: object model, serializer, string/name escaping, xref.
- [x] 2.2 `options.rs`: `PdfOptions`, enums, defaults, `normalize`, `locked`.
- [x] 2.3 `joboptions.rs`: parser (Acrobat-tolerant), canonical writer,
  `get`/`set` by key, preset text without security/save options.
- [x] 2.4 `presets.rs`: twelve built-ins, summary lines and warnings.
- [x] 2.5 `image.rs`: mode → colour space and samples, ICC tagging, conversion,
  alpha soft mask / flatten, 16-bit.
- [x] 2.6 `resample.rs`: Average / Subsampling / Bicubic.
- [x] 2.7 `ccitt.rs`: Group 4 encoder.
- [x] 2.8 `crypt.rs`: R2/R3/R4/R6 standard security handler.
- [x] 2.9 `xmp.rs`; standards: output intents, info keys, TrimBox.
- [x] 2.10 `linearize.rs`: Fast Web View layout + hint stream.
- [x] 2.11 `mod.rs`: `write_pdf`, page, content, notes, thumbnail, PieceInfo.
- [x] 2.12 `read.rs`: extract Pictura's embedded PSD.
- [x] 2.14 `text.rs`: live text — `Type0` font embedding, `ToUnicode`, `TJ`
  runs, fill in the image's colour space, constant alpha; `type_plain.rs`;
  `pictura_render::pdf_live_type` / `pdf_type_fonts`; app background composite
  and font registration before save; tests (D13).
- [x] 2.13 Rust tests (unit + `pdftoppm`/`gs`/`qpdf` oracles, self-skipping).

## 3. App

- [x] 3.1 `encode_image.*`: `encode_jpeg` bytes edge.
- [x] 3.2 `cxxqt_object/pdf_export.rs`: `pdf_save`, `pdf_open`, option
  get/set/normalize/locked/summary, presets, joboptions bridge; `save()` and
  `build.rs` routing.
- [x] 3.3 `pdf_save_dialog.{h,cpp}`: Save options, preset, standard,
  compatibility, five pages, Save Preset.
- [x] 3.4 `pdf_presets_dialog.{h,cpp}`: `Edit > Adobe PDF Presets…`.
- [x] 3.5 Save As filter, Save/Save As routing, warning, View After Saving,
  open of a `.pdf`, session last options; `CMakeLists.txt`.
- [x] 3.6 `tst_save_pdf` Qt suite.

## 4. CI, docs, close-out

- [x] 4.1 `.github/workflows/ci.yml`: poppler-utils, ghostscript, qpdf.
- [x] 4.2 `docs/dev/doc-code-parity-gaps.md` WF-2, `docs/dev/STATE.md`
  (`TASK-ALLOWS-DOCS`, separate commit).
- [ ] 4.3 Follow-up issues: JPEG2000 encoding; spot-colour channels; Proof
  Setup colour option.
- [x] 4.4 Build, `verify-fast`, archive the change.
