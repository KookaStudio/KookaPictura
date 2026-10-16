# Tasks: color-replacement-tool

## 1. Engine

- [x] 1.1 `pictura_paint::replace` (modes, sampling, limits, tolerance, anti-alias); unit tests.
- [x] 1.2 `StrokeKind` / `Stroke::begin_kind`: per-dab strokes on the live-stroke path; unit tests.
- [x] 1.3 Move the non-separable blend helpers to `pictura_core::nonseparable`.

## 2. Bridge

- [x] 2.1 `begin_color_replacement` in `cxxqt_object/paint_tools.rs`; one "Color Replacement Tool" state.

## 3. Tool

- [x] 3.1 `tool_colorreplacement.cpp`; catalog row enabled; options-bar row; size ring and brackets.
- [x] 3.2 The B-group cycle reaches Color Replacement after Pencil.

## 4. Verification

- [x] 4.1 `color_replacement_tool` self-test (540); `shift_plain` (117) and `keys_shown` (116) updated; `bash scripts/verify-fast.sh`.
