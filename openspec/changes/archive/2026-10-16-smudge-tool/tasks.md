# Tasks: smudge-tool

## 1. Engine

- [x] 1.1 `pictura_paint::smudge` (`SmudgeOptions`, `SmudgeBrush`) and `Stroke::begin_smudge`; unit tests.

## 2. Bridge

- [x] 2.1 `begin_smudge` in `cxxqt_object/paint_tools.rs`; one "Smudge" state.

## 3. Tool

- [x] 3.1 `tool_retouch.cpp` handler; bar in `options_bar_retouch.cpp`; catalog row enabled.

## 4. Verification

- [x] 4.1 `smudgeTool` in the Qt Test suite `tst_retouch_tools`; `bash scripts/verify-fast.sh`.
