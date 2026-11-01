# Test Plan

One row per scenario in the spec delta. Coverage names an automated test, or
`manual:` with the reason it cannot be automated.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| Open a file adds a document | Open a valid PSD, then open a PNG through `openDocumentAtPath` | A document is added with the file's path recorded | `tst_loading_tab::successFillsTabAndActivates`, `tst_open_save::rasterOpenKeepsFilePath` |
| Open failure does not add a document | Select a corrupt PSD among readable PNGs and confirm | The corrupt file adds no document and the application stays usable | `tst_multi_file_open::failedFileDoesNotStopTheRest` |
| Recent files | Open a document, read the Open Recent list | The opened path is at the front of the bounded list | manual: unchanged by this change; the existing self-test `recent_files` check (exit code 533) covers it |
| `dlg_open_multiple` | Drive the real File > Open dialog: assert `ExistingFiles`, select three PNGs, accept; repeat with `Open Documents As Tabs` off and two PNGs | Three documents open, one per selected path, each with its own path; with the preference off each opens in a floating window | `tst_multi_file_open::multiSelectOpensOneDocumentPerFile`, `tst_multi_file_open::multipleSelectionFollowsTheTabsPreference` |
| `dlg_open_multiple_partial` | Select two PNGs and a corrupt PSD in the dialog and accept | The two PNGs open, the corrupt PSD adds no document | `tst_multi_file_open::failedFileDoesNotStopTheRest` |
| `dlg_open_single` | Select one PNG in the dialog and accept | One document opens with that path | `tst_multi_file_open::singleSelectionOpensOneDocument` |
