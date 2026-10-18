# Tasks: pdf-eps-import

## 1. Tests first

- [x] 1.1 `pictura-codec` tests named in `test-plan.md` (page info, bounding box, rotation, render, anti-aliasing, images, broken xref, passwords, missing Ghostscript, EPS, sniffing)
- [x] 1.2 `tst_import_pdf` Qt Test suite named in `test-plan.md`, registered in `cpp/tests/CMakeLists.txt`

## 2. Engine (`pictura-codec`)

- [x] 2.1 `ghostscript`: locate (`PICTURA_GHOSTSCRIPT`, else `gs`), run with `-dSAFER`, collect stderr warnings, password errors
- [x] 2.2 `png_read`: minimal decoder for `pngalpha` output
- [x] 2.3 `pdf_import`: page info program and parser with box defaults and rotation, Bounding Box, render-and-crop, thumbnails
- [x] 2.4 `pdf_import::objects`: object reader (xref tables, xref streams, object streams, predictors, scan fallback) and image listing / wrapping
- [x] 2.5 `eps`: DOS binary header, bounding box comments, render
- [x] 2.6 `sniff_vector`: `%PDF-` / `%!PS-Adobe` / DOS EPS signatures

## 3. App

- [x] 3.1 Bridge (`cxxqt_object/pdf_export/import.rs`): page info, thumbnails, images, open page / image / EPS into a view with mode, depth, resolution
- [x] 3.2 `ImportPdfDialog` (Select, thumbnails, Page Options, Image Size, Suppress Warnings)
- [x] 3.3 `RasterizeEpsDialog`
- [x] 3.4 Window routing: sniffing in `openDocumentAtPath`, password prompt, warnings box, dialog driver seam, multi-document naming
- [x] 3.5 `File > Open As…` command and format list; Open filters list Generic PDF / EPS
- [x] 3.6 New sources in `CMakeLists.txt`

## 4. Packaging and docs

- [x] 4.1 Flatpak manifest builds Ghostscript
- [x] 4.2 `docs/dev/STATE.md`, `docs/dev/doc-code-parity-gaps.md` (WF-3) — TASK-ALLOWS-DOCS
- [x] 4.3 `openspec validate --all --strict`; archive the change
