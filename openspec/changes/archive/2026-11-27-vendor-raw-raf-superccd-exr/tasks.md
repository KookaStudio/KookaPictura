# Tasks

## 1. Tests
- [x] 1.1 Word sections, SuperCCD layouts and records in `tests/camera_raw/raw/raf_builder.rs`
- [x] 1.2 Every new or changed test named in `test-plan.md`

## 2. Codec
- [x] 2.1 `DngImage::fuji_width`, `None` for every other reader
- [x] 2.2 `dng/raf.rs`: primary frame, upright 2 × 2 layouts, word sections and their levels
- [x] 2.3 `dng/raf_superccd.rs`: active area and LibRaw's grid
- [x] 2.4 `dng/demosaic/superccd.rs`: `fuji_rotate` after the demosaic, budgeted

## 3. App
- [x] 3.1 Save Image writes a SuperCCD raw as linear

## 4. Colour
- [x] 4.1 `raw_jpeg_blocks` uses the upright camera RGB for SuperCCD
- [x] 4.2 Fitter reads word-section presets from `0x2100` and `0x2400`
- [x] 4.3 Fit the SuperCCD and EXR sample bodies; regenerate the table

## 5. Close
- [x] 5.1 Corpus run against LibRaw (`PICTURA_RAW_CORPUS`, rawpy)
- [x] 5.2 Follow-up issue for SuperCCD SR frame merging (#1648)
- [x] 5.3 ROADMAP, PARITY, PROVENANCE and STATE.md
- [x] 5.4 Archive the change
