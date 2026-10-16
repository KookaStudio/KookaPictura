# Tasks: mixer-brush-tool

## 1. Engine

- [x] 1.1 `pictura_paint::mixer` (Wet, Load, Mix, Flow, reservoir pickup, transparency lock); unit tests.
- [x] 1.2 `StrokeKind::Mixer` and `Stroke::mixer_reservoir`; unit test.

## 2. Bridge

- [x] 2.1 `begin_mixer_brush` / `mixer_reservoir` in `cxxqt_object/paint_tools.rs`; one "Mixer Brush Tool" state.

## 3. Tool

- [x] 3.1 `tool_mixerbrush.cpp` (reservoir carry, Load / Clean after stroke, Alt-click load); catalog row enabled.
- [x] 3.2 Options-bar row: load swatch, toggles, presets, Wet / Load / Mix / Flow, disabled Sample All Layers.
- [x] 3.3 The B-group cycle reaches the Mixer Brush after Color Replacement.

## 4. Verification

- [x] 4.1 `mixer_brush_tool` self-test (541); `shift_plain` (117) asserts the B cycle; `bash scripts/verify-fast.sh`.
