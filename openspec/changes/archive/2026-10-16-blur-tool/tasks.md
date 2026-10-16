# Tasks: blur-tool

## 1. Engine

- [x] 1.1 `pictura_paint::focus` (`BlurOptions`, `BlurMode`, `BlurBrush`) and `Stroke::begin_blur`; unit tests.

## 2. Bridge

- [x] 2.1 `begin_blur` in `cxxqt_object/paint_tools.rs`; one "Blur" state.

## 3. Tool

- [x] 3.1 `tool_blur.cpp` handler (since `tool_retouch.cpp`); bar in `options_bar_paint.cpp` (since `options_bar_retouch.cpp`); catalog row enabled; `CMakeLists.txt` entries.

## 4. Verification

- [x] 4.1 `blurTool` in the Qt Test suite `tst_retouch_tools` (first a self-test, code 554, now retired); the guard (98) retargeted; `bash scripts/verify-fast.sh`.
