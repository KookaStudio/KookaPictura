# Tasks: gradient-tool

## 1. Engine

- [x] 1.1 `pictura_paint::gradient` (styles, ramp, presets, `draw`) and the shared `fill` pass; unit tests.

## 2. Bridge

- [x] 2.1 `cxxqt_object/paint_tools/fills.rs`: preset queries and `draw_gradient`; one "Gradient" state.

## 3. Tool

- [x] 3.1 `tool_fills.cpp` drag handler; bar in `options_bar_fill.cpp`; catalog row enabled; `CMakeLists.txt` entries.

## 4. Verification

- [x] 4.1 `gradientTool` in the Qt Test suite `tst_fill_tools` (first a self-test, code 552, now retired); `shift_plain` (117), `keys_shown` (116), and the guard (98) retargeted; `bash scripts/verify-fast.sh`.
