# Tasks: loading-tab-progress

## 1. Tests first

- [x] 1.1 Rust `open_psd_reports_phases` collecting the phase sequence
- [x] 1.2 `tst_loading_tab::progressCardShowsPhases` and `progressCardCancelCancels`; extend `genericPdfProbeHasNoTab` with the card assertions

## 2. Rust

- [x] 2.1 Add the `open_progress` qsignal to `PictureView`
- [x] 2.2 Thread `&dyn Fn(u8, &str)` through the decode/import helpers with a no-op for synchronous callers
- [x] 2.3 `open_jobs::start` and `open_pdf_start` build the queued progress callback at the phase boundaries

## 3. App shell

- [x] 3.1 `LoadingCard` (`loading_card.{h,cpp}`): file name, phase, bar, percentage, Cancel, test accessors
- [x] 3.2 The loading page becomes a stacked card page/canvas page; `finishLoadingDocument` switches and deletes the card
- [x] 3.3 `wireOpenSignals` / `onOpenProgress` update the card; the card's Cancel shares the deferred cancel path
- [x] 3.4 `theme.cpp` card rule; `CMakeLists.txt` registers the new files

## 4. Docs and verification

- [x] 4.1 `docs/10-workflow-io/document-lifecycle.md`: the phase card and that percentages are phase weights (TASK-ALLOWS-DOCS)
- [x] 4.2 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`
- [x] 4.3 `cargo nextest run --workspace`, `cargo test --workspace --doc`
- [x] 4.4 `ctest --test-dir build -R '^tst_'`
- [x] 4.5 `openspec validate --all --strict`; archive the change
