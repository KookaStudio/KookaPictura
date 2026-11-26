# Tasks

## 1. Tests
- [x] 1.1 Synthetic DNG writer and lossless JPEG encoder (`tests/camera_raw/dng/`)
- [x] 1.2 Every test named in `test-plan.md` (`tests/camera_raw/dng_reader.rs`)

## 2. Codec
- [x] 2.1 `dng/tiff.rs`: classic and BigTIFF IFD parsing with bounds checks
- [x] 2.2 `dng/ljpeg.rs`: lossless JPEG decoder
- [x] 2.3 `dng/levels.rs`: linearization, black/white normalization, active-area crop
- [x] 2.4 `dng/mod.rs`: IFD selection, strip/tile decode, budget, `read_dng` and `DngImage`

## 3. Close
- [x] 3.1 ROADMAP raw-open line and STATE.md
- [x] 3.2 Archive the change
