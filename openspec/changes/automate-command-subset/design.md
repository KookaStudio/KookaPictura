# Design

## Context

See `proposal.md` — Why. The File > Automate/Scripts leaves are registered with path-derived ids and have no handlers. The app already has single-document open/save, `decode_image_rgba`, `encode_image_rgba`, `place_image`, `add_raster_layer_from_rgba` (origin-only and always RGB), and a one-page `pictura_codec::pdf::write_pdf`. `frame_menus.cpp` is 1106 LOC and near the 1200-line cap. The resolution-aware new-document path already exists as `image_adjust::create_document`/`set_document_resolution`, and a new tab is added through `PicturaMainWindow::addDocument`, not `document_api::newDocument`.

## Goals / Non-Goals

**Goals:**

- Five working commands with deterministic, testable outputs.
- One new C++ translation unit for the menu wiring and one dialog per command.
- Multi-page PDF output without changing the one-page Save As behavior.
- Explicit unavailability for the three dependent commands, with a visible reason.

**Non-Goals:**

- Actions, droplets, Batch, File Naming, Photomerge, Auto-Blend, or scripting.
- Camera Raw input or Run Action in Image Processor.
- Adobe's gallery templates, multiple gallery styles, custom Picture Package layouts, or labels.
- PDF slide-show output and the full Output/Presentation option set.
- Background-thread batch execution; the five commands run in a modal dialog and report progress between files.
- Exact Photoshop option parity beyond the documented subset; the ceilings are recorded.

## Decisions

### D1: Orchestrate in the app, not a new crate

The commands need the app's document/save/import paths and dialogs. A new Rust automation crate would duplicate the open/save orchestration or add a dependency edge for little benefit. The implementation uses:
- `PictureView::open`/`save` for PSD/PSD-like inputs,
- Qt's image reader/writer for JPEG/TIFF/PNG inputs and outputs,
- `pictura_render` only for placing raster layers into a new document.

### D2: A separate `frame_menus_automate.cpp`

The menu wiring, enablement, and dialog launch live in a new translation unit. `command_tree.cpp` gains the missing Web Photo Gallery leaf and stable command ids. `frame_menus.cpp` is not grown. Every new `.cpp`/`.h` is added to the explicit `CMakeLists.txt`.

### D3: Registry status tips for the deferred leaves

`CommandSpec`/`CommandRegistry` gain a status-tip field applied to the `QAction`; the existing registry only sets text, enabled, and checked. The Conditional Mode Change and Photomerge actions stay disabled with a status tip naming the missing prerequisite (`#616`, panorama work). File Naming is not a menu leaf; the Batch destination (#618) is the missing surface and the spec says so.

### D4: Layout commands build a document, then place layers

Contact Sheet II and Picture Package create a new document through `image_adjust::create_document` + `set_document_resolution` and add it with `PicturaMainWindow::addDocument`, then scale each source to its cell and add it as a raster layer at a given `PsdRect`. A small engine helper adds a layer with an explicit rect and maps colour to the document mode (the current origin-only helper always creates RGB channels). Captions use the type-layer path when available; if the type bridge cannot create a layer without an interactive session, a raster caption layer is used and the ceiling is marked.

### D5: Multi-page PDF as a page list

`pictura_codec::pdf` gains a `PdfPage` list entry point. The existing `write_pdf` becomes a one-element wrapper, so `Save As` output is byte-identical. The page list shares the catalog, encryption, and font resources; each page holds one raster image. PDF Presentation maps its ordered list to pages, including duplicates. Each page's size uses the source image's captured DPI; the decode bridge returns DPI because the current `decode_image_rgba` returns only pixels.

### D6: Image Processor runs synchronously with per-file reporting

The dialog processes inputs one at a time with a progress dialog and a cancel check between files. It writes to `JPEG/`, `PSD/`, `TIFF/` under the destination. The encoder gains a box-fit scale helper and a documented 0–12 → 0–100 JPEG-quality mapping. A metadata-aware save path writes the IPTC copyright and embeds the sRGB profile; the current `encode_image_rgba` accepts only pixels. Unreadable files are accumulated into a report. Camera Raw and Run Action are not offered.

### D7: Web Photo Gallery writes a fixed built-in style

The gallery writer emits an index page, one page per image, the scaled large images under `images/`, and thumbnails under `thumbnails/`, using relative links and a small embedded CSS style. No Adobe template files are read or copied. Output names are sanitized and collision-suffixed deterministically. The full CS6 option set and multiple styles are not offered.

### D8: Picture Package and Web Photo Gallery are built-in deviations

CS6 ships both as optional 32-bit plug-ins and hides their menu items when not installed. Kooka provides both built-in; the spec and parity register record the availability difference rather than pretending the optional-install model exists.

## Risks / Trade-offs

- **Large folders block the UI** → The progress dialog updates between files; the command is explicitly not a background Batch replacement.
- **Output name collisions** → Append a deterministic numeric suffix and report the rename rather than overwriting.
- **PDF page geometry varies by source size** → Capture DPI at decode; use each image's pixel size at its own resolution; record the per-page size in the test.
- **JPEG/TIFF encode options differ from Photoshop** → Expose only the options the existing writers honor; record the rest as ceilings.
- **Contact-sheet memory** → Build one cell-sized RGBA plane per placed image; do not keep all sources in memory at once.
- **Flatten equivalence** → Compare the flattened composite with the unflattened composite in the test, not just layer counts.
- **Menu growth** → `command_tree.cpp` is at 1032 LOC; the new entries are small, but a further split of the File section is the fallback if it crosses the cap.
- **New test suites are invisible without CMake registration** → Every new `tst_*` is added to `crates/pictura-app/cpp/tests/CMakeLists.txt` in the same task that writes it.
- **Optional-plugin deviation** → Recorded in the spec and parity register; not silently presented as CS6 built-in behavior.
