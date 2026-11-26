# Tasks

## 1. Tests
- [x] 1.1 CRX encoder (`tests/camera_raw/raw/crx_encode.rs`) and CR3 builder (`tests/camera_raw/raw/cr3_builder.rs`)
- [x] 1.2 Every test named in `test-plan.md`

## 2. Codec
- [x] 2.1 `dng/crx.rs`, `dng/crx_lines.rs`, `dng/crx_wavelet.rs`: the LibRaw port, with attribution
- [x] 2.2 `dng/cr3.rs`: container, track choice, metadata, levels
- [x] 2.3 `sniff_raw`, `sniff_raw_prefix` and `read_raw` name and read CR3
- [x] 2.4 CR3 bodies in `scripts/gen-camera-matrices.py`, table regenerated

## 3. App
- [x] 3.1 `*.CR3` in the Open dialog's filters

## 4. Close
- [x] 4.1 Real-file corpus run and the LibRaw comparison
- [x] 4.2 Patent status search
- [x] 4.3 ROADMAP, PARITY, PROVENANCE and STATE.md
- [x] 4.4 Archive the change
