# Test plan

One row per spec scenario. Coverage names the automated test, or
`manual: <reason>` when it cannot be automated. Checked by
`scripts/check-test-plan.py`. Tests that run Ghostscript self-skip when `gs`
is absent; CI installs it. Fixtures (PDFs, EPS files) are written by the tests
themselves, by hand or through `gs -sDEVICE=pdfwrite`, so no binary fixture is
committed.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `pdf_import_dialog_shown` | Open a generic one-page PDF through the window | Import PDF dialog runs; no document until OK | `tst_import_pdf::genericPdfShowsDialog` |
| `pdf_pictura_skips_dialog` | Save a layered document as Photoshop PDF with Preserve on; open it | Layers back; dialog driver never called | `tst_import_pdf::picturaPdfSkipsDialog` |
| `pdf_sniff_signature` | Copy a PDF to `art.ai`; open it | Import PDF dialog runs | `tst_import_pdf::pdfRecognizedBySignature`, `pictura-codec::sniff_recognizes_pdf_and_eps` |
| `pdf_cancel` | Open a PDF; driver rejects the dialog | Document count unchanged | `tst_import_pdf::cancelOpensNothing` |
| `pdf_dialog_defaults` | Open `sample.pdf`; read the dialog fields | Pages, 3D disabled, `1 of 1 page(s) selected`, Name, Bounding Box, Anti-aliased, Inches, 300 Pixels/Inch, RGB Color, 8 bit, Small, Suppress off | `tst_import_pdf::dialogDefaults` |
| `pdf_thumbnail_size` | Switch Thumbnail Size Small → Large → Fit Page | Icon size 64 → 128 → grid width | `tst_import_pdf::thumbnailSizeChangesGrid` |
| `pdf_page_boxes` | Parse page info for MediaBox/CropBox/TrimBox only; also through Ghostscript | Media 612×792, Crop/Bleed/Art 540×720, Trim 468×648 | `pictura-codec::page_boxes_apply_pdf_defaults`, `pictura-codec::page_info_reads_boxes_through_ghostscript` |
| `pdf_bounding_box` | Bounding box of a page painting (20,20)–(80,50); dialog size at 300 ppi | 60×30 pt; 0.833 × 0.417 in | `pictura-codec::bounding_box_hugs_marks`, `tst_import_pdf::boundingBoxSetsSize` |
| `pdf_page_rotate` | Page info and render of a 144×72 page with `/Rotate 90` | Box 72×144; render portrait | `pictura-codec::rotated_page_swaps_box`, `pictura-codec::rotated_page_renders_portrait` |
| `pdf_constrain_proportions` | Media Box of 144×72; set Width 4 in; untick Constrain; set Width 3 | Height 2, then still 2 | `tst_import_pdf::constrainKeepsAspect` |
| `pdf_resolution_keeps_size` | Resolution 300 → 150 | Inches unchanged; Image Size label quarters | `tst_import_pdf::resolutionKeepsPhysicalSize` |
| `pdf_multipage_max_size` | Select pages 144×72 and 288×144, Media Box; open at 300 ppi | Fields 4 × 2 in; first page 600×300 px | `tst_import_pdf::multiPageShowsLargestBox` |
| `pdf_render_page` | Render red-rectangle page, Media Box, 72 ppi; open through the window | 144×72 px, 72 ppi, `Layer 1`, red inside, transparent outside | `pictura-codec::render_page_crops_box`, `tst_import_pdf::opensPageOnTransparentLayer` |
| `pdf_antialias` | Render a triangle page with Anti-aliased on and off | Partial alpha present, then none | `pictura-codec::antialias_toggles_edge_coverage` |
| `pdf_mode_depth` | Open as CMYK 16-bit, then Grayscale 8-bit | Document modes/depths match | `tst_import_pdf::modeAndDepthApply` |
| `pdf_multi_select_documents` | Select pages 1 and 3 of `deck.pdf`; open | Two documents `deck-1.pdf`, `deck-3.pdf`; label `2 of 3 page(s) selected` | `tst_import_pdf::multiSelectOpensDocuments` |
| `pdf_images_mode_lists` | List images of a page drawing one image twice plus one inside a Form XObject; switch the dialog to Images | Two images; label `1 of 2 image(s) selected` | `pictura-codec::images_listed_once_through_forms`, `tst_import_pdf::imagesModeListsImages` |
| `pdf_images_mode_opens` | Render the 40×30 Flate RGB image; open it from Images mode | 40×30 px equal to the samples | `pictura-codec::image_renders_at_native_size`, `tst_import_pdf::imagesModeOpensImage` |
| `pdf_objects_broken_xref` | Corrupt `startxref`; read objects | Objects found by scan | `pictura-codec::objects_survive_broken_xref` |
| `pdf_password_prompt` | Encrypt a PDF with user password `kooka` (`gs` / `qpdf`); open; answer `kooka` | Dialog shows one page | `pictura-codec::password_required_is_reported`, `tst_import_pdf::passwordPromptOpens` |
| `pdf_wrong_password` | Answer `wrong`, then cancel | Prompt shown twice; nothing opens | `pictura-codec::wrong_password_is_rejected`, `tst_import_pdf::wrongPasswordAsksAgain` |
| `pdf_suppress_warnings` | Open a PDF with a damaged xref with Suppress off, then on | Warning box once; page opens both times | `tst_import_pdf::suppressWarningsHidesMessage` |
| `gs_missing_reports` | Point `PICTURA_GHOSTSCRIPT` at a missing file; render; open PDF and EPS in the window | Engine error; message names Ghostscript; no document | `pictura-codec::missing_ghostscript_is_an_error`, `tst_import_pdf::missingGhostscriptReports` |
| `eps_dialog_defaults` | Open an EPS with `%%BoundingBox: 0 0 144 72` | 2 × 1 in, 300 Pixels/Inch, RGB Color, Constrain and Anti-aliased on | `pictura-codec::eps_bounding_box_prefers_hires`, `tst_import_pdf::epsDialogDefaults` |
| `eps_render` | Open the blue EPS at 72 ppi | 144×72 opaque blue | `pictura-codec::eps_renders_bounding_box`, `tst_import_pdf::epsOpensAtSize` |
| `eps_dos_binary_header` | Wrap the EPS in a DOS binary header with a dummy TIFF section; read and render | Same box and render | `pictura-codec::dos_eps_header_is_unwrapped` |
| `eps_cancel` | Cancel the EPS dialog | No document | `tst_import_pdf::epsCancelOpensNothing` |
| `open_as_command` | Read the `File > Open As…` action | Enabled; `Ctrl+Alt+Shift+O` | `tst_import_pdf::openAsIsLive` |
| `open_as_forces_format` | Open As a PNG named `photo.dat` with the PNG format | Document with the PNG's pixels | `tst_import_pdf::openAsForcesFormat` |
| `open_as_mismatch` | Open As a PNG with Generic PDF | Status `Could not open …: format mismatch or damaged file.`; no document | `tst_import_pdf::openAsReportsMismatch` |
| `open_filters_pdf_eps` | Read `openFileFilters()` | Generic PDF and Generic EPS entries; All Formats includes them | `tst_import_pdf::openFiltersListPdfEps` |
| Rendering fidelity on real-world PDFs | Open a few PDFs with CMYK art, transparency groups, embedded fonts, JPX/JBIG2 images | Pages look like Acrobat's rendering | manual: no CS6 or Acrobat oracle in CI; Ghostscript itself is the renderer |
| Flatpak bundles Ghostscript | Build the Flatpak; open a PDF and an EPS inside it | Both import | manual: CI does not build the Flatpak on pull requests |
