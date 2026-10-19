# Tasks: loading-tab-spinner

## 1. Tests first

- [x] 1.1 Rust unit tests named in `test-plan.md`: decode/apply parity for PSD, raster, smart object, and import; the cancellation drop; PDF status classification
- [x] 1.2 `tst_loading_tab` Qt Test suite named in `test-plan.md`, registered in `cpp/tests/CMakeLists.txt`
- [x] 1.3 Remove the banner case from `tst_design_system` and delete `tst_open_loading_banner` from the test list
- [x] 1.4 Add the loading-tab assertions to `tst_import_pdf` for the PDF and EPS entry points (self-skip without `gs`)

## 2. Rust worker

- [x] 2.1 Add `impl cxx_qt::Threading for PictureView` and the `open_finished` / `open_detected` signals
- [x] 2.2 Add the `open_cancelled` flag, the test delay, and `open_cancel`
- [x] 2.3 Split `open`, `open_image`, `open_as_smart_object`, and `pdf_open` into decode + `apply_opened`
- [x] 2.4 Split `pdf_import_open_page`, `pdf_import_open_image`, and `eps_import_open` into render + import/apply
- [x] 2.5 New `open_jobs.rs`: start functions, worker spawn, queued apply, status strings, PDF detection signal
- [x] 2.6 Convert every captured Qt value to a Rust-owned `Send` value before spawning

## 3. App shell

- [x] 3.1 `DocEntry` loading/token/page fields; `addDocument` split into canvas creation/wiring helpers; `removeDocument` null-safe
- [x] 3.2 `TabSpinner` widget (arc animation, hover close glyph, click cancel, test accessors)
- [x] 3.3 `frame_open.cpp`: async start functions for PSD, raster, smart object, router, and Open As; loading tab begin/finish/cancel; busy-cursor counter; sync wrappers and `documentOpenFinished`
- [x] 3.4 `frame_pdf.cpp` / `frame_import.cpp`: async `pdf_open` probe with `open_detected`, generic-PDF import flow, page/image renders, EPS render, password retries, warnings and recent
- [x] 3.5 Middle-click event filter; tab-activation tracking; `closeDocument` and `closeEvent`/`aboutToQuit` cancellation guards
- [x] 3.6 Control server: map a cancelled open to a `cancelled` error; keep the synchronous wrapper
- [x] 3.7 Remove the banner member, helpers, signal, QSS rule, and `frame_columns.cpp` repositioning
- [x] 3.8 Add `tab_spinner.{h,cpp}` (and any new `frame_*` file) to `CMakeLists.txt`

## 4. Docs and verification

- [x] 4.1 `docs/01-architecture/threading-and-concurrency.md`: resolve the async-runtime open question and describe the open worker; `docs/10-workflow-io/document-lifecycle.md`: record the loading tab (TASK-ALLOWS-DOCS)
- [x] 4.2 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`
- [x] 4.3 `cargo nextest run --workspace`, `cargo test --workspace --doc`
- [ ] 4.4 `bash scripts/verify-fast.sh`
- [x] 4.5 `openspec validate --all --strict`; archive the change
