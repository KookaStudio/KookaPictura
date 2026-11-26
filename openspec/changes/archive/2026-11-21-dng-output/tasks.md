# Tasks

## 1. Tests
- [x] 1.1 `pictura-codec` unit tests in `dng/write/tests.rs`
- [x] 1.2 `pictura-codec` linear-reader and decode tests in `tests/camera_raw/dng_linear.rs`
- [x] 1.3 `pictura-codec` reference-tool test `tests/camera_raw/dng_writer_oracle.rs`
- [x] 1.4 `pictura-app` Rust tests in `open_jobs/raw_open/save_dng_tests.rs`
- [x] 1.5 Qt cases in `tst_raw_open`

## 2. Codec
- [x] 2.1 `dng/write/`: IFD builder, lossless-JPEG encoder, original raw, previews, `write_dng`
- [x] 2.2 `dng/linear.rs`: `read_linear_dng`; `decode_plane` samples per pixel
- [x] 2.3 `decode_raw_source` linear path; exports

## 3. App
- [x] 3.1 `raw_open/save_dng.rs`: worker, default path, job on `PendingRaw`
- [x] 3.2 Bridge functions in `open_jobs.rs`
- [x] 3.3 `RawSaveOptionsDialog`, `runRawSaveImage`, Save Image button and status in the Pictura Raw dialog; nested-dialog runner from the coordinator

## 4. Close
- [x] 4.1 ROADMAP, parity-gap register, STATE.md
- [x] 4.2 Archive the change
