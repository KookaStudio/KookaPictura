# Tasks

## 1. Tests
- [x] 1.1 Synthetic ARW writer (`tests/camera_raw/raw/arw_builder.rs`): uncompressed, ARW2 encoder, lossless tiles, SR2 block
- [x] 1.2 Every test named in `test-plan.md` (`tests/camera_raw/raw_arw.rs`, `tests/camera_raw/raw_camera_matrices.rs`, `arw_decode` unit tests, app sniff and filter tests)

## 2. Codec
- [x] 2.1 `dng/tiff.rs`: `ascii_in`
- [x] 2.2 `dng/arw_decode.rs`: ARW2 blocks, tone curve, SR2 decipher
- [x] 2.3 `dng/arw.rs`: the ARW reader; `vendor.rs` sniff and dispatch
- [x] 2.4 `scripts/gen-camera-matrices.py`: Sony bodies; regenerate `camera_matrices.rs`

## 3. App
- [x] 3.1 Open dialog filters list `*.ARW`

## 4. Close
- [x] 4.1 Real-file corpus run (`PICTURA_RAW_CORPUS`)
- [x] 4.2 ROADMAP, PARITY, PROVENANCE and STATE.md
- [x] 4.3 Archive the change
