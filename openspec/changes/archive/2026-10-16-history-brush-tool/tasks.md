# Tasks: history-brush-tool

## 1. History

- [x] 1.1 `History` brush source (oldest default, state / snapshot, index follow, pinning); unit test.

## 2. Bridge

- [x] 2.1 `begin_history_brush` and the source accessors in `cxxqt_object/paint_tools.rs`; one "History Brush" state.

## 3. Tool

- [x] 3.1 `tool_stamps.cpp` History Brush handler; catalog row enabled; options-bar row.
- [x] 3.2 History panel source marker and left-column choice.

## 4. Verification

- [x] 4.1 `history_brush_tool` self-test (544); `shift_plain` (117) asserts the Y group; `bash scripts/verify-fast.sh`.
