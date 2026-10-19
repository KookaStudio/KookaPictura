# Test plan

One row per spec scenario in
`specs/document/document-lifecycle/spec.md`. Coverage names the automated test,
or `manual: <reason>` when it cannot be automated. Checked by
`scripts/check-test-plan.py`.

The loading-tab tests use `PicturaMainWindow::setOpenDelayForTest`, which keeps
a real open in flight long enough to observe the tab, spinner, hover, and
cancel without blocking the GUI thread. PDF/EPS cases that need Ghostscript
self-skip when `gs` is absent, as the existing import suite does.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `lt_off_thread` | Start an open with the delay hook; process events while the worker is held | The tab exists, the spinner advances, events are processed, and no document is available yet | `tst_loading_tab::loadingTabShowsWhileWorkerRuns`, `pictura-app::open_psd_matches_sync` |
| `lt_loading_tab` | Open a small PSD and a PNG with the delay hook | A tab titled with the file name is present with an empty workspace and no canvas | `tst_loading_tab::openInsertsLoadingTab` |
| `lt_background` | Open A, then start B while A is active | A stays active and visible; B becomes active only after its open succeeds | `tst_loading_tab::backgroundOpenKeepsCurrentDocument` |
| `lt_empty_workspace` | With no document, start an open | The loading tab is current, `imageView()` is null, and the page has no canvas | `tst_loading_tab::firstOpenShowsEmptyWorkspace` |
| `lt_busy_cursor` | Start two delayed opens, finish one, then cancel the other | The override cursor is set while either runs and cleared after the last ends | `tst_loading_tab::busyCursorSpansConcurrentOpens` |
| `lt_generic_pdf` | Start a generic PDF; observe the probe and the dialog driver; accept a page | No tab during the probe or dialog; a tab when the page render starts | `tst_loading_tab::genericPdfProbeHasNoTab`, `tst_import_pdf::pdfEntryPointUsesLoadingTab` |
| `lt_entry_points` | Open a PSD, a PNG, a smart object, a PDF, and an EPS through the frame | Each entry point uses the loading-tab path and its completion rules | `tst_loading_tab::entryPointsShareThePath`, `tst_import_pdf::pdfEntryPointUsesLoadingTab`, `tst_import_pdf::epsEntryPointUsesLoadingTab` |
| `lt_spinner_animates` | Read the spinner angle, wait, read it again while the worker is held | The angle advances | `tst_loading_tab::spinnerAnimates` |
| `lt_hover_close` | Send enter/leave to the spinner | Hover state reports the close glyph, leaving restores the spinner | `tst_loading_tab::hoverSwapsToClose` |
| `lt_click_cancel` | Click the spinner while held; then wait past the worker delay | The tab is removed immediately, the previous tab returns, the late result is discarded, and no error is shown | `tst_loading_tab::clickingSpinnerCancels` |
| `lt_middle_click_cancel` | Middle-click the loading tab while held | The open is cancelled and the tab removed | `tst_loading_tab::middleClickCancels` |
| `lt_normal_tab_unchanged` | Hover and middle-click a normal document tab | No spinner is installed and its middle-click behaviour is unchanged (Qt's closable-tab close) | `tst_loading_tab::normalTabsAreUnchanged` |
| `lt_cancel_discards` | Cancel a delayed open; let the worker finish; process events | No document and no tab appear, and no error is shown | `tst_loading_tab::cancelledResultIsDiscarded` |
| `lt_success` | Open a PSD with no other document; wait for completion | The tab is filled, the default close button is back, and the tab is active | `tst_loading_tab::successFillsTabAndActivates` |
| `lt_success_keeps_selection` | Open A, start B delayed, select A, let B finish | A stays active and B is filled in the background | `tst_loading_tab::userSelectedTabIsNotStolen` |
| `lt_notice` | Open a flat CMYK PSD and wait | The status bar shows `Converted from CMYK; saved as CMYK` | `tst_loading_tab::successKeepsStatusNotice` |
| `lt_failure` | Open a missing or undecodable file | The loading tab is removed, no document is added, and the previous tab returns | `tst_loading_tab::failureRemovesTab` |
| `lt_cancel_distinct` | Cancel an open and inspect the completion outcome | The outcome is cancelled and no error is reported | `tst_loading_tab::cancelIsNotFailure` |
| `lt_banner_removed` | Read the built stylesheet and open a document | No `#documentOpenBanner` rule or widget exists | `tst_design_system::loadingBannerRuleIsGone`, `tst_loading_tab::bannerIsGone` |
| Decode and apply stay byte-identical | Decode a PSD, raster, smart object, and import through the worker helpers and apply them; compare with the synchronous open | Document size, mode, depth, active layer, history label, path, dirty flag, and composite match | `pictura-app::open_psd_matches_sync`, `pictura-app::open_image_matches_sync`, `pictura-app::open_smart_object_matches_sync`, `pictura-app::import_render_matches_sync` |
| Cancellation drops a finished result | Set the cancellation flag, then apply a decoded result | The view stays without a document and no signal-visible state changes | `pictura-app::cancelled_open_result_is_dropped` |
| PDF statuses are classified | Probe a generic PDF and a missing file through the decode helper; the Pictura and protected cases stay with the import suite | `none` is reported without a document; the existing Pictura/protected tests still open or prompt | `pictura-app::pdf_open_status_classifies`, `tst_import_pdf::picturaPdfSkipsDialog`, `tst_import_pdf::passwordPromptOpens` |
