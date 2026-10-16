# Tasks: art-history-brush-tool

## 1. Engine

- [x] 1.1 `pictura_paint::art_history` (styles, scatter, tolerance gate, seed) and `Stroke::begin_art_history`; unit tests.

## 2. Bridge

- [x] 2.1 `begin_art_history_brush` and the style names in `cxxqt_object/paint_tools.rs`; one "Art History Brush" state.

## 3. Tool

- [x] 3.1 `tool_stamps.cpp` Art History Brush handler; catalog row enabled; options-bar row.

## 4. Verification

- [x] 4.1 `art_history_brush_tool` self-test (548); `shift_plain` (117) asserts the Y group; `bash scripts/verify-fast.sh`.
