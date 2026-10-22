# Tasks

## 1. Command routing and deferred leaves

- [ ] 1.1 Add failing `tst_automate_menu` cases `fiveCommandsOpenDialogs` and `deferredCommandsUnavailable`; register the suite in `crates/pictura-app/cpp/tests/CMakeLists.txt`.
- [ ] 1.2 Add a status-tip field to `CommandSpec`/`CommandRegistry` and apply it to the `QAction`; add the command ids and the missing Web Photo Gallery leaf; implement the five handlers in a new `frame_menus_automate.cpp`; set the Conditional Mode Change/Photomerge status tips; make the cases pass.
- [ ] 1.3 Add `frame_menus_automate.cpp` and its header to `CMakeLists.txt`; verify `ctest --test-dir build -R '^tst_automate_menu$' --output-on-failure`.

## 2. Image Processor

- [ ] 2.1 Add failing `tst_image_processor` cases `folderWritesJpegPsdTiff`, `resizeFitsBox`, `unreadableFileIsSkipped`, `copyrightAndProfileWritten`, and `atLeastOneFormatRequired`; register the suite.
- [ ] 2.2 Add the box-fit scale helper and the 0–12 → 0–100 quality mapping, and a metadata-aware save path for IPTC copyright and the sRGB profile.
- [ ] 2.3 Add the Image Processor dialog and the per-file processing loop with the three output folders, resize, sRGB, copyright, and the unreadable-file report; make the cases pass.
- [ ] 2.4 Add the dialog source/header to `CMakeLists.txt`; verify `ctest --test-dir build -R '^tst_image_processor$' --output-on-failure`.

## 3. Contact Sheet II and Picture Package

- [ ] 3.1 Add failing `tst_contact_sheet` cases `gridPlacesLayers`, `placeDownChangesOrder`, `filenameCaptions`, and `flattenCollapsesLayers`; add `tst_picture_package` cases `layoutPlacesCopies`, `pageGeometryFollowsOptions`, and `flattenCollapsesPackage`; register both suites.
- [ ] 3.2 Add a raster placement helper that sets an explicit layer rect and maps colour to the document mode; add the resolution-aware new-document path (`create_document` + `set_document_resolution` + `addDocument`).
- [ ] 3.3 Add the Contact Sheet dialog with Place across/down, auto-spacing, rotate, captions, and flatten; make the Contact Sheet cases pass.
- [ ] 3.4 Add the built-in Picture Package layouts, source File/Folder selection, and flatten; make the Picture Package cases pass.
- [ ] 3.5 Add both dialogs' sources/headers to `CMakeLists.txt`; verify `ctest --test-dir build -R '^tst_(contact_sheet|picture_package)$' --output-on-failure`.

## 4. PDF Presentation

- [ ] 4.1 Add a failing `pictura-codec` test `pdf_writer_emits_pages_in_order` for a multi-page page list and a `tst_pdf_presentation::orderAndDuplicates` / `tst_pdf_presentation::pageSizesFollowSourceDpi` case; register the suite.
- [ ] 4.2 Add the page-list writer to `pictura-codec::pdf` and keep `write_pdf` as a one-page wrapper; make the codec test pass and the existing PDF tests unchanged.
- [ ] 4.3 Extend the decode bridge with the source DPI and add the PDF Presentation dialog with add/reorder/duplicate and per-page sizes; make the Qt case pass.
- [ ] 4.4 Add the dialog source/header to `CMakeLists.txt`; verify `cargo nextest run -p pictura-codec` and `ctest --test-dir build -R '^tst_(pdf_presentation|save_pdf|import_pdf)$' --output-on-failure`.

## 5. Web Photo Gallery

- [ ] 5.1 Add failing `tst_web_photo_gallery` cases `outputTreeAndLinks` and `linksResolve`; register the suite.
- [ ] 5.2 Add the built-in gallery writer and dialog with the recorded option ceiling; make the cases pass.
- [ ] 5.3 Add the dialog source/header to `CMakeLists.txt`; verify `ctest --test-dir build -R '^tst_web_photo_gallery$' --output-on-failure`.

## 6. Integration and records

- [ ] 6.1 Update `docs/dev/doc-code-parity-gaps.md` (AUTO-4 to PARTIAL, UI-8), `docs/02-ui-ux/menus.md` (File tree / Web Photo Gallery), `ROADMAP.md` (split the five shipped from the three deferred), and `docs/dev/STATE.md`; record the optional-plugin, Camera Raw, Run Action, slide-show, and gallery-option ceilings with follow-up issues. Carry `TASK-ALLOWS-DOCS`.
- [ ] 6.2 Run `openspec validate --all --strict`, `python3 scripts/check-test-plan.py`, and `bash scripts/verify-full.sh`; fix every failure.
- [ ] 6.3 Archive the change with `openspec archive automate-command-subset` and verify `scripts/check-openspec-archived.sh` passes.
