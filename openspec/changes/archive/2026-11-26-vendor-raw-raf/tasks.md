# Tasks

## 1. Tests
- [x] 1.1 Synthetic RAF writer and encoders (`tests/camera_raw/raw/raf_builder.rs`)
- [x] 1.2 Every test named in `test-plan.md`

## 2. Codec
- [x] 2.1 `dng/raf.rs`: header, directory, raw directory, codings, levels; `vendor.rs` sniff and dispatch
- [x] 2.2 Refuse compressed (#1620), SuperCCD and EXR (#1621) by name
- [x] 2.3 `dng/demosaic/xtrans.rs`; `Ahd` routes X-Trans to it

## 3. Colour
- [x] 3.1 `fit-camera-jpeg-matrices.py` reads RAF; fit the samples
- [x] 3.2 Regenerate `camera_jpeg_matrices.rs`

## 4. App
- [x] 4.1 Open dialog filters list `*.RAF`

## 5. Close
- [x] 5.1 Real-file corpus run (`PICTURA_RAW_CORPUS`)
- [x] 5.2 ROADMAP, PARITY, PROVENANCE and STATE.md
- [x] 5.3 Archive the change
