# Tasks

## 1. Tests
- [x] 1.1 `tests/raw_rw2_colour.rs`: Panasonic lookup, white, decode, refusal and corpus tests
- [x] 1.2 `raw_format_2_saturates_at_3967` and the updated normalization helper in `tests/raw_rw2.rs`
- [x] 1.3 `raw_sniff_detects_by_signature` with Panasonic and Photoshop Raw `.RAW` files; `tst_raw_open::openFiltersListCameraRaw` with the RW2 extensions

## 2. Tools and table
- [x] 2.1 `examples/raw_jpeg_blocks.rs` and `scripts/fit-camera-jpeg-matrices.py`
- [x] 2.2 `CameraMatrices` names each matrix's light; `gen-camera-matrices.py` adds the GX9 and the accepted camera-JPEG fits
- [x] 2.3 Fit the samples and regenerate `camera_matrices.rs`

## 3. Codec and app
- [x] 3.1 `rw2.rs`: RawFormat 2 white at most 3967
- [x] 3.2 Open dialog filters list `*.RW2 *.RWL *.RAW`

## 4. Close
- [x] 4.1 Render each fitted body beside its camera JPEG
- [x] 4.2 Follow-up issue for the bodies still without colour
- [x] 4.3 ROADMAP, PARITY, PROVENANCE and STATE.md
- [x] 4.4 Archive the change
