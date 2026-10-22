# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| The five implemented commands open their dialogs | Trigger Image Processor, Contact Sheet II, PDF Presentation, Picture Package, and Web Photo Gallery | Each dialog opens and no document is mutated before acceptance | `tst_automate_menu::fiveCommandsOpenDialogs` |
| The deferred commands stay unavailable | Inspect or trigger Conditional Mode Change and Photomerge, and look for a Batch File Naming option | Each is unavailable with a prerequisite reason and no document changes | `tst_automate_menu::deferredCommandsUnavailable` |
| A folder produces the three formats | Process a folder with a readable PSD and JPEG with all three formats selected | `JPEG/`, `PSD/`, and `TIFF/` each contain one valid output per input | `tst_image_processor::folderWritesJpegPsdTiff` |
| Resize to fit | Process a large image with a smaller resize box | The output fits inside the box with the aspect ratio preserved | `tst_image_processor::resizeFitsBox` |
| An unreadable input is skipped | Include one corrupt file in a folder run | The other files are processed and the corrupt file is reported | `tst_image_processor::unreadableFileIsSkipped` |
| Copyright and profile are written | Enable copyright metadata and sRGB conversion, then process a file | The output carries the copyright field and the sRGB profile | `tst_image_processor::copyrightAndProfileWritten` |
| At least one output format is required | Clear all three format checkboxes and accept the dialog | The dialog refuses with a message and writes nothing | `tst_image_processor::atLeastOneFormatRequired` |
| A 2×2 grid places four images | Build a 2-column by 2-row contact sheet from four images with Place across | Four image layers appear in the four cells in row-major order | `tst_contact_sheet::gridPlacesLayers` |
| Place down changes the order | Build the same sheet with Place down | The images fill the column first and the layer order follows it | `tst_contact_sheet::placeDownChangesOrder` |
| Filename captions | Enable Use Filename As Caption | Each placed image has a caption containing its source filename | `tst_contact_sheet::filenameCaptions` |
| Flatten collapses the sheet | Build an unflattened sheet, then flatten it | The flattened composite matches the unflattened composite and one layer remains | `tst_contact_sheet::flattenCollapsesLayers` |
| Order and duplicates are preserved | Reorder three entries and duplicate one, then write the PDF | The PDF has four pages in the displayed order, including the duplicate | `tst_pdf_presentation::orderAndDuplicates` |
| The page count matches the list | Write a presentation with N images carrying distinct DPI | The PDF reports exactly N pages and each page size follows its source DPI | `pictura-codec::pdf_writer_emits_pages_in_order`, `tst_pdf_presentation::pageSizesFollowSourceDpi` |
| Save As stays one page | Save a single document through the normal Save As PDF path | The PDF has one page and the existing PDF tests are unchanged | `pictura-codec::pdf_writer_emits_pages_in_order` |
| A built-in layout places the copies | Place one source with a four-copy built-in layout | Four distinct placed copies appear inside the page bounds | `tst_picture_package::layoutPlacesCopies` |
| Page geometry follows the options | Choose a page size, resolution, and mode | The new document matches those values | `tst_picture_package::pageGeometryFollowsOptions` |
| Flatten collapses the package | Flatten the package | The flattened composite matches the unflattened package and one layer remains | `tst_picture_package::flattenCollapsesPackage` |
| The output tree is complete | Generate a gallery for two images | The index file and the three directories exist with the generated files | `tst_web_photo_gallery::outputTreeAndLinks` |
| Links resolve | Inspect the generated index and page files | Every local image and page link resolves to a file in the output tree | `tst_web_photo_gallery::linksResolve` |
