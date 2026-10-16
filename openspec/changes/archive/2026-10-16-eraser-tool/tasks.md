# Tasks: eraser-tool

## 1. Engine

- [x] 1.1 `pictura_paint::eraser` (`EraserMode`, `begin_erase`, background vs transparency, Erase To History) and the square tip; unit tests.

## 2. Bridge

- [x] 2.1 `begin_eraser` in `cxxqt_object/paint_tools.rs`; one "Eraser" state.

## 3. Tool

- [x] 3.1 `tool_eraser.cpp` handler (Alt erases to history); catalog row enabled; options-bar row; `CMakeLists.txt` entry.

## 4. Verification

- [x] 4.1 `eraser_tool` self-test (547); `shift_plain` (117), `keys_shown` (116), and the guard (98) retargeted; `bash scripts/verify-fast.sh`.
