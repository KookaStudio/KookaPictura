# Tasks: hue-saturation-replace-color-shift

## 1. Engine

- [x] 1.1 Hue/Saturation lightness as a per-channel blend (`color.rs`, native depth through the shared `hue_saturation_rgb`).
- [x] 1.2 GPU `adj_hs` to match.
- [x] 1.3 Replace Color `shift`, with `replace_color_result` and `replace_color_shift_for`.

## 2. App

- [x] 2.1 Bridge `image_replace_color_result` and `image_replace_color_shift_for`.
- [x] 2.2 Replace Color dialog: the Result swatch and picker call the bridge.

## 3. Verification

- [x] 3.1 Rust tests: lightness blend, dark noise stays neutral, the shift-for round trip, a gray sample takes no colour.
- [x] 3.2 `tst_replace_color` updated for the gray-sample case.
- [x] 3.3 `bash scripts/verify-full.sh`; `openspec validate --all --strict`.
