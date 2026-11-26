# Tasks

## 1. Data
- [x] 1.1 `scripts/gen-camera-matrices.py` fits A and D65 matrices from rawtoaces-data (pinned commit)
- [x] 1.2 Generated `crates/pictura-codec/src/dng/camera_matrices.rs` (14 Canon CR2 bodies; 600D deferred)
- [x] 1.3 `assets/PROVENANCE.md` entry

## 2. Tests
- [x] 2.1 `tests/camera_raw/raw/cr2_builder.rs`: maker-note colour data
- [x] 2.2 Every test named in `test-plan.md`

## 3. Codec
- [x] 3.1 `dng/vendor.rs`: `model_key`, `camera_color`
- [x] 3.2 `dng/cr2.rs`: as-shot neutral from tag `0x4001`, colour from the table

## 4. Close
- [x] 4.1 ROADMAP raw lines and STATE.md
- [x] 4.2 Archive the change
