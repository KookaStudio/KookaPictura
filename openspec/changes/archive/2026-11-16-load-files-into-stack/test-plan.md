# Test Plan

One row per scenario in the change's spec delta. Dialog and integration
outcomes are covered offscreen by `tst_load_files_into_stack`; the pure stack
construction, alignment, and smart-object combination are covered by Rust unit
tests in `pictura-app` and `pictura-render`. The native file chooser cannot be
driven offscreen, so the Browse row is a reviewer check.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `lfs_dialog` | Open `File ▸ Scripts ▸ Load Files into Stack…`. | The dialog shows Source Files, Use, the list, Browse…, Remove, Add Open Files, both checkboxes, OK, and Cancel. | `tst_load_files_into_stack::dialogShowsControls` |
| `lfs_defaults` | Open the dialog. | Use is Files, both checkboxes are off, and the list is empty. | `tst_load_files_into_stack::dialogDefaults` |
| `lfs_use_menu` | Open the Use menu. | It offers Files and Folder and nothing else. | `tst_load_files_into_stack::dialogUseMenu` |
| `lfs_empty_ok` | Open the dialog, then add one path. | OK is disabled while empty and enabled after a path is added. | `tst_load_files_into_stack::dialogOkNeedsFiles` |
| `lfs_cancel` | Open the dialog on an open document and cancel. | Pixels, path, modified state, and history are unchanged. | `tst_load_files_into_stack::dialogCancelChangesNothing` |
| `lfs_browse` | Click Browse… and choose two image files. | Both files appear in the list in the chosen order. | manual: the native file chooser cannot be driven on the offscreen platform; reviewer selects two files |
| `lfs_remove` | Select one row and click Remove. | Only that row leaves the list. | `tst_load_files_into_stack::removeSelected` |
| `lfs_add_open` | Open a saved `photo.png`, then click Add Open Files. | `photo.png` is appended to the list. | `tst_load_files_into_stack::addOpenFiles` |
| `lfs_dedupe` | Add the same path twice. | The list contains it once. | `tst_load_files_into_stack::duplicatesIgnored` |
| `lfs_folder` | Choose Folder and pick a folder with two images and a text file. | The list holds the two images and not the text file. | `tst_load_files_into_stack::folderListsImages` |
| `lfs_new_document` | Load two readable images. | A new untitled document with two layers is created. | `tst_load_files_into_stack::loadsOneLayerPerFile` |
| `lfs_canvas` | Load a 640 × 480 and a 320 × 200 image. | The new document is 640 × 480. | `pictura-app::canvas_is_largest_source`, `tst_load_files_into_stack::canvasIsLargestSource` |
| `lfs_order` | List `first.png` before `second.png` and load. | The top layer is `first.png`; the layer below is `second.png`. | `pictura-app::first_file_is_topmost`, `tst_load_files_into_stack::firstFileIsTopmost` |
| `lfs_failures` | Load one readable image and one missing path. | The document has one layer and the missing path is reported. | `tst_load_files_into_stack::unreadableFileIsSkipped` |
| `lfs_all_failed` | Load only missing or unreadable paths. | No document is created. | `tst_load_files_into_stack::allFailedCreatesNothing` |
| `lfs_no_document` | Run the command with no document open. | A new stacked document is created. | `tst_load_files_into_stack::worksWithoutDocument` |
| `lfs_align` | Load two offset copies of one texture with the option on. | At least one layer is moved off the canvas origin. | `pictura-app::aligns_offset_copies` |
| `lfs_no_align` | Load the same sources with the option off. | Every layer keeps its `(0, 0)` rect. | `pictura-app::no_align_keeps_positions` |
| `lfs_smart_object` | Load two files with the option on. | The document holds one smart-object layer whose embedded source contains two layers. | `pictura-app::smart_object_embeds_stack`, `tst_load_files_into_stack::smartObjectCombinesLayers` |
| `lfs_no_smart_object` | Load two files with the option off. | The document holds two separate raster layers. | `pictura-app::no_smart_object_keeps_layers`, `tst_load_files_into_stack::noSmartObjectKeepsLayers` |
