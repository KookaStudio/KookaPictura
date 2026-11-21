# Test plan — file-browse-and-statistics

Every scenario is automated except the two native file/folder pickers, which
have no offscreen driver, and the statistical Auto-Align visual check. Dialog list behaviour is tested through the dialog's
public add methods; the merge cases run real files through the app offscreen
with small PNG fixtures and read pixels with `PictureView::composite_argb`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `fb_menu` | Open the File menu in an Extended build with no document | `Browse…` sits between `Open` and `Open As`, shows `Ctrl+Alt+O`, and is enabled | `tst_file_menu::browseMenuRow` |
| `fb_no_bridge` | Dispatch `file.browse` on a machine without Adobe Bridge | The folder hand-off runs through the URL opener; no Bridge process or dependency is involved | `tst_file_menu::browseNeedsNoBridge` |
| `fb_document_folder` | Open a temp PNG, install a URL-opener spy, dispatch `file.browse` | The spy receives the PNG's containing folder | `tst_file_menu::browseOpensTheDocumentFolder` |
| `fb_home` | Dispatch `file.browse` with no document, then with an untitled document | Both dispatches hand the home folder to the opener | `tst_file_menu::browseWithoutAPathOpensHome` |
| `fb_unchanged` | Record a document's pixels, path, modified state, active view, and history; dispatch `file.browse` | All five are unchanged | `tst_file_menu::browseLeavesDocumentsUnchanged` |
| `iss_command` | Extended build with no document; inspect and trigger `file.scripts.statistics` | The row exists under Scripts, is labelled `Statistics…`, and is enabled | `tst_edition_gating::editionGating`, `tst_statistics::statisticsCommandEnabledWithoutDocument` |
| `iss_edition` | Standard build; look for `file.scripts.statistics` | No action exists | `tst_edition_gating::editionGating` |
| `iss_mean` | Accept two solid opaque PNGs (red, blue) with Mean and align off | One untitled, unsaved document with one Smart Object layer; `composite_argb` is the rounded per-channel mean (purple, ±1) | `tst_statistics::mergesTwoImagesWithMean` |
| `iss_mode` | Accept the same sources with Maximum | The composite pixel is per-channel maximum (magenta) | `tst_statistics::selectedStackModeIsApplied` |
| `iss_align` | Accept two sources with the same feature at translated positions, align on, then repeat with align off | Aligned: the feature occupies one position after reduction; unaligned: both positions remain | manual: Auto-Align needs a real-photo fixture on this engine; the dialog forwards the flag (`tst_statistics::twoFilesUnlockOkAndAlign`) and `tst_auto_align` covers the engine |
| `iss_partial_failure` | Select two readable PNGs and one corrupt image file; accept | A document is created from the two readable sources; the warning names the corrupt file | `tst_statistics::unreadableSourceIsSkipped` |
| `iss_all_fail` | Select one readable PNG and one corrupt image file; accept | No document is created; the warning names the corrupt file | `tst_statistics::fewerThanTwoDecodableCreatesNothing` |
| `iss_remember` | Accept the dialog with Median and align on; read `loadSession().statisticsOptions`; then dispatch the command again and inspect the new dialog | The session records Median's code and align true, and the next dialog preselects them | `tst_statistics::acceptedChoiceIsStored`, `tst_statistics::rememberedChoiceSeedsTheNextInvocation` |
| `iss_no_document` | With no document, dispatch the command, add two files, accept | The dialog opens and the merge creates a document | `tst_statistics::worksWithoutDocument` |
| `sd_controls` | Open the Statistics dialog | Title is `Image Statistics`; Choose Stack Mode, Source Files, Use, list, Browse…, Remove, Add Open Files, align, OK, and Cancel are present; no smart-object checkbox | `tst_statistics::dialogShowsControls` |
| `sd_defaults` | Construct the dialog with no remembered choice | Use is Files, mode is Mean, align is off, list is empty, OK is disabled | `tst_statistics::dialogDefaults` |
| `sd_two_files` | Add one file, then a second | OK and align stay disabled at one and become enabled at two | `tst_statistics::twoFilesUnlockOkAndAlign` |
| `sd_buttons` | Construct with no selection and no open document; then select a row and construct with a saved open document | Remove and Add Open are disabled first; selecting enables Remove; a saved open document enables Add Open | `tst_statistics::buttonsFollowSelectionAndOpenDocuments` |
| `sd_browse_files` | Add two image files through the dialog's file-add path | Both appear in order | manual: the native file picker cannot be driven offscreen; list order is `tst_statistics::addFilesKeepsOrderAndDedupes` |
| `sd_folder` | Use Folder and add a folder with two images and a text file | The two images are added, the text file is not | `tst_statistics::folderAddsSupportedImages` |
| `sd_remove` | Select one row and press Remove | Only that row leaves the list | `tst_statistics::removeSelected` |
| `sd_add_open` | Construct with one saved clean document and one unsaved document; press Add Open Files | The clean document is added, the unsaved one is not, and the save-first warning is shown | `tst_statistics::addOpenFilesAddsSavedOnly` |
| `sd_dedupe` | Add the same path twice | The list contains it once | `tst_statistics::addFilesKeepsOrderAndDedupes` |
| `sd_remembered` | Construct with remembered Skewness and align on | The mode menu shows Skewness and align is checked | `tst_statistics::rememberedChoiceSeedsDialog` |
| `sd_cancel` | Open the dialog over an open document and cancel | Pixels, path, modified state, and history are unchanged | `tst_statistics::cancelChangesNothing` |

## Additional non-scenario checks

- `tst_file_menu::scriptsBrowseIsDisabled` covers the disabled
  `File > Scripts > Browse…` row added by the proposal.
- `pictura-script::golden_catalogue_matches_the_captured_table`,
  `pictura-script::golden_const_names_match_the_captured_ids`, and
  `pictura-script::cr_id_drift` cover the catalogue and generated-header
  updates.
