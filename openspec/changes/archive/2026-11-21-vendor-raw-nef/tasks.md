# Tasks

## 1. Tests
- [x] 1.1 Synthetic NEF writer (`tests/camera_raw/raw/nef_builder.rs`) with packers and a Nikon Huffman encoder
- [x] 1.2 Every test named in `test-plan.md` (`tests/camera_raw/raw_nef.rs`, `tests/camera_raw/raw_camera_matrices.rs`, `tst_raw_open`)

## 2. Codec
- [x] 2.1 `dng/ljpeg.rs`: a bit reader without byte stuffing
- [x] 2.2 `dng/tiff.rs`: one-entry ASCII read for prefix sniffing
- [x] 2.3 `dng/nef.rs` + `dng/nef_decode.rs`: the NEF reader; `vendor.rs` dispatch
- [x] 2.4 Nikon bodies in `scripts/gen-camera-matrices.py`, regenerated table, PROVENANCE

## 3. App
- [x] 3.1 Open dialog filters list `*.NEF`

## 4. Close
- [x] 4.1 ROADMAP raw lines and STATE.md
- [x] 4.2 Archive the change
