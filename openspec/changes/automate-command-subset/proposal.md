# Proposal

## Why

The File menu's Automate and Scripts entries are registration-only stubs: Image Processor, Conditional Mode Change, Contact Sheet II, PDF Presentation, Photomerge, Picture Package, and Web Photo Gallery do nothing, and Web Photo Gallery is absent. The parity audit records the remaining Automate commands as missing (`AUTO-4`, issue #619).

Three of those are blocked by engines outside this bundle: Conditional Mode Change records an action step (#616), File Naming is a Batch option (#618), and Photomerge needs panorama alignment/blending work that overlaps Auto-Align/Auto-Blend (#582/#583; CS6's Photomerge performs its own alignment/blending rather than calling the separate commands). This change ships the five self-contained commands and leaves the three dependent ones explicitly unavailable rather than inventing divergent behavior.

## What Changes

- Implement `File > Scripts > Image Processor…`: process open files or a folder into JPEG, PSD, and/or TIFF, with resize-to-fit, sRGB conversion, and copyright metadata. Camera Raw input, Run Action, and the full Photoshop option set are recorded ceilings (Camera Raw has no reader; Actions is #616).
- Implement `File > Automate > Contact Sheet II…`: a grid of scaled images in a new document, with document size/resolution/mode, columns/rows, Place across/down, auto-spacing, rotate-for-best-fit, filename captions, and a flatten option.
- Implement `File > Automate > PDF Presentation…`: an ordered file list with add/reorder/duplicate and a multi-page PDF output. Slide-show output and the full Output/Presentation option set are recorded ceilings. The existing one-page Save As PDF path stays unchanged.
- Implement `File > Automate > Picture Package…`: built-in page layouts placing one or more copies, with page size, resolution, mode, and flatten. Labels and custom layouts are ceilings. CS6 ships Picture Package as an optional 32-bit plug-in; Kooka provides it built-in, which is a recorded deviation.
- Implement `File > Automate > Web Photo Gallery…`: a built-in style that writes `index.htm(l)`, `images/`, `pages/`, and `thumbnails/` with working links. CS6 ships Web Photo Gallery as an optional plug-in and exposes more styles/options; Kooka provides a built-in style, which is a recorded deviation.
- Keep `Conditional Mode Change`, File Naming (a Batch option, not a command), and `Photomerge` unavailable with a clear reason; #619 remains open/partial until #616, #618, and the panorama work land.

## Capabilities

### New Capabilities

- `interop/automate-commands`: the five self-contained Automate/Scripts file-processing commands and their output contracts.

### Modified Capabilities

None.

## Impact

- `crates/pictura-app` C++: five new dialogs, a new `frame_menus_automate.cpp` (the existing `frame_menus.cpp` is near its size cap), command ids, handlers, a registry status-tip field for the deferred leaves, and a resolution-aware new-document path (`image_adjust::create_document` + `set_document_resolution` + `PicturaMainWindow::addDocument`).
- `crates/pictura-codec/src/pdf`: a multi-page writer used by PDF Presentation; the existing one-page `write_pdf` remains a wrapper.
- Tests: `tst_automate_menu`, `tst_image_processor`, `tst_contact_sheet`, `tst_pdf_presentation`, `tst_picture_package`, `tst_web_photo_gallery`, and a PDF page-order unit test. Every new suite and every new `.cpp`/`.h` is added to the explicit CMake lists.
- No new dependencies. The file I/O reuses the existing open/save, decode/encode, and PDF paths; the metadata/resize/DPI helpers are added at that boundary.
- `docs/dev/doc-code-parity-gaps.md` AUTO-4 becomes `PARTIAL` and UI-8 is updated; `ROADMAP.md` separates the five shipped commands from the three deferred ones; `docs/dev/STATE.md` is updated.
