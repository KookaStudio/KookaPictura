# Tasks: red-eye-tool

## 1. Engine

- [x] 1.1 `pictura_paint::healing::red_eye_layer` (Pupil Size, Darken Amount); unit tests.

## 2. Bridge

- [x] 2.1 `red_eye` in `cxxqt_object/healing.rs`; one "Red Eye Tool" state.

## 3. Tool

- [x] 3.1 `tool_redeye.cpp` (drag a box, click a 24 px box); catalog row enabled; options-bar row.
- [x] 3.2 The J-group cycle reaches Red Eye after Content-Aware Move.

## 4. Verification

- [x] 4.1 `red_eye_tool` self-test (539); `shift_plain` (117) asserts the J cycle; guard (98) probes Clone Stamp; `bash scripts/verify-fast.sh`.
