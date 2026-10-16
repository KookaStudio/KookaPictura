# Tasks: brush-preset-picker

## 1. Engine

- [x] 1.1 Dab scatter / count / shape jitter in `StrokeConfig` and `Stroke::sample`; unit tests. Move the stroke tests to `stroke/tests.rs`.

## 2. Bridge

- [x] 2.1 Dynamics in `PaintTip`; `begin_brush`; `brush_dab_preview`.

## 3. UI

- [x] 3.1 `BrushPresetPicker` with the two-segment Size slider and the default set; `CMakeLists.txt` entry.
- [x] 3.2 The tip button on every brush bar; controller dynamics.

## 4. Verification

- [x] 4.1 `brush_preset_picker` self-test (549); checks 307 and 325 retargeted; `bash scripts/verify-fast.sh`.
