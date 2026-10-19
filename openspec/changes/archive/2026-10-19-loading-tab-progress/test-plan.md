# Test plan

One row per spec scenario in
`specs/document/document-lifecycle/spec.md`. Coverage names the automated test,
or `manual: <reason>` when it cannot be automated. Checked by
`scripts/check-test-plan.py`.

The card tests use `PicturaMainWindow::setOpenDelayForTest`, which holds the
completion after the decode, so the final phase (`Preparing layers…`, 90%) is
observable while the tab is still loading.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `lt_phase_progress` | Start a PSD with the delay hook; wait for the card; read its phase, bar, and percentage | The card is visible with the file name, `Preparing layers…`, a value of 90, and `90%`; the worker's phase sequence is read, decode, composite, prepare | `tst_loading_tab::progressCardShowsPhases`, `pictura-app::open_psd_reports_phases` |
| `lt_phase_cancel` | Open A, start B delayed, click the card's Cancel | B's tab and card disappear; A is active; no error; the late result is discarded | `tst_loading_tab::progressCardCancelCancels` |
| `lt_phase_no_probe` | Start a generic PDF; inspect the card in the dialog driver and during the page render | No card during the probe/dialog; a card exists while the chosen page renders | `tst_loading_tab::genericPdfProbeHasNoTab`, `tst_import_pdf::pdfEntryPointUsesLoadingTab` |
