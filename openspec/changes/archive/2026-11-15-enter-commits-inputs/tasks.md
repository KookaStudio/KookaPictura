# Tasks

## 1. Tests

- [x] 1.1 Add `tst_value_input_enter` with the Enter/Shift+Enter, invalid, dialog default-button, and multi-line cases.
- [x] 1.2 Add `tst_type_session_panels::enterInSizeFieldCommitsAndLeaves` and `tst_type_panels::glyphsFamilyEnterCommitsAndLeaves`.
- [x] 1.3 Add `tst_crop_options::enterInFieldKeepsCropActive`, `tst_perspective_crop::enterInFieldKeepsQuadStaged`, `tst_transform_options::enterInFieldKeepsTransformActive`, and `tst_pen_tools::enterInFieldKeepsPathActive`.
- [x] 1.4 Register `tst_value_input_enter` in the Qt test CMake list.

## 2. App

- [x] 2.1 Add the shared application Enter filter (`value_input_enter.{h,cpp}`) and register it in the shell sources.
- [x] 2.2 Install the filter at startup and from `PicturaMainWindow`.
- [x] 2.3 Handle Enter/Shift+Enter in `NumericField` (commit, deactivate or highlight).
- [x] 2.4 Guard the Crop, Perspective Crop, and path Enter paths in `frame_keys.cpp` with the focused-editor check.
- [x] 2.5 Guard the Free Transform Enter path in `image_view.cpp` with the focused-editor check.
- [x] 2.6 Keep the multi-line and search/rename Enter paths out of the policy.

## 3. Gates

- [x] 3.1 Build and run `tst_value_input_enter`, `tst_crop_options`, `tst_perspective_crop`, `tst_transform_options`, `tst_pen_tools`, `tst_type_session_panels`, and `tst_type_panels`.
- [x] 3.2 `python3 scripts/check-test-plan.py`, `openspec validate --all --strict`.
- [x] 3.3 Archive with `openspec archive enter-commits-inputs`.
