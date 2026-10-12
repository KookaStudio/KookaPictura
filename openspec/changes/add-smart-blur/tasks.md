# Tasks: add-smart-blur

## 1. Engine

- [x] 1.1 Add `SmartBlurMode` and `Filter::SmartBlur` with the selective Gaussian and edge map in `pictura-filters/src/blur/smart.rs`.
- [x] 1.2 List Normal mode in `filter_preserves_opacity`; give the kind a `radius + 1` preview apron.

## 2. App

- [x] 2.1 Map the `smart-blur` kind (Radius, Threshold, Quality, Mode).
- [x] 2.2 Add the `Filter ▸ Blur ▸ Smart Blur` row.

## 3. Verification

- [x] 3.1 Unit tests: smoothing vs step, threshold gate, Edge Only, Overlay Edge, Quality, flat/alpha/refusals.
- [x] 3.2 Mapping row (no equivalent), kernel digests, profile entry.
- [x] 3.3 Rust mapping test; `tst_smart_blur`; the Smart Blur slot count in `tst_filter_menu`.
- [x] 3.4 Update `docs/06-filters/blur-filters.md` and `docs/dev/STATE.md` (`TASK-ALLOWS-DOCS`).
- [x] 3.5 Run `bash scripts/verify-full.sh` and `openspec validate --all --strict`.
