# Tasks: background-eraser-tool

## 1. Engine

- [x] 1.1 Share `match_strength` / `reachable` from `replace.rs`; `BackgroundEraser` and `StrokeKind::BackgroundErase`; `ensure_alpha`; unit tests.

## 2. Bridge

- [x] 2.1 `begin_background_eraser` (Background → layer); one "Background Eraser" state.

## 3. Tool

- [x] 3.1 Handler in `tool_eraser.cpp`; bar in `options_bar_erase.cpp`; catalog row enabled; `CMakeLists.txt` entries.

## 4. Verification

- [x] 4.1 `background_eraser_tool` self-test (550); guard (98) and `keys_shown` (116) retargeted; `bash scripts/verify-fast.sh`.
