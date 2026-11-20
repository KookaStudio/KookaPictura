# Tasks

## 1. Tests
- [x] 1.1 `pictura-codec` tests in `tests/raw_develop.rs`
- [x] 1.2 `pictura-adjust` 16-bit render tests
- [x] 1.3 `pictura-app` Rust tests in `cxxqt_object/raw_open/tests.rs`
- [x] 1.4 Qt suite `tst_raw_open` (registered in `cpp/tests/CMakeLists.txt`)

## 2. Codec
- [x] 2.1 `dng/develop.rs`: `decode_raw_source`, `develop_raw`, `downscale_camera`, `raw_xmp`
- [x] 2.2 `crs_settings_from_xmp`

## 3. Pictura Raw
- [x] 3.1 `render_pictura_raw_rgb16`

## 4. App
- [x] 4.1 `cxxqt_object/raw_open.rs`: pending raw, seed, preview, decode and develop jobs
- [x] 4.2 Open coordinator: signature routing, `"raw"` status, dialog, failure message
- [x] 4.3 Pictura Raw dialog raw mode; reciprocal `AcrSlider`
- [x] 4.4 Open dialog Camera Raw filter

## 5. Close
- [x] 5.1 ROADMAP raw line, parity-gap note, STATE.md
- [x] 5.2 Follow-up issues: #1466 (smart object), #1465 (vendor matrices)
- [x] 5.3 Archive the change
