# Tasks: sharpen-tool

## 1. Engine

- [x] 1.1 `Focus::Sharpen` and Protect Detail in `pictura_paint::focus`; `Stroke::begin_focus`; unit tests.

## 2. Bridge

- [x] 2.1 `begin_focus` in `cxxqt_object/paint_tools.rs`; one "Sharpen" state.

## 3. Tool

- [x] 3.1 `tool_retouch.cpp` handler; bar in `options_bar_retouch.cpp`; catalog row enabled; `CMakeLists.txt` entries.

## 4. Verification

- [x] 4.1 `sharpenTool` in the Qt Test suite `tst_retouch_tools`; the guard (98) retargeted; `bash scripts/verify-fast.sh`.
