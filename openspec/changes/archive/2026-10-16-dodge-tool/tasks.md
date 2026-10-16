# Tasks: dodge-tool

## 1. Engine

- [x] 1.1 `pictura_paint::tone` (`ToneRange`, `ToneOptions`, `ToneBrush`) and `Stroke::begin_tone`; unit tests.

## 2. Bridge

- [x] 2.1 `begin_dodge` in `cxxqt_object/paint_tools.rs`; one "Dodge" state.

## 3. Tool

- [x] 3.1 `tool_retouch.cpp` handler; bar in `options_bar_retouch.cpp`; catalog row enabled.

## 4. Verification

- [x] 4.1 `dodgeTool` in the Qt Test suite `tst_retouch_tools`; `shift_plain` (117) retargeted; `bash scripts/verify-fast.sh`.
