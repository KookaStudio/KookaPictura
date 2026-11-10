# Tasks

## 1. Tests

- [x] 1.1 Add `stampToolsOpenBrushPicker` to `tst_canvas_context_menus`: for
  each of Clone Stamp, Pattern Stamp, History Brush, and Art History Brush, a
  routed canvas right-click requests the brush picker.
- [x] 1.2 Add `stampPresetApplies` to `tst_canvas_context_menus`: choosing a
  preset in a picker opened by a stamp tool changes the tip its options expose.

## 2. Implementation

- [x] 2.1 Override `hasCanvasContextMenu()` and `onContextMenu()` on
  `StampToolHandler` to request the shared Brush Preset picker.

## 3. Verification

- [x] 3.1 `openspec validate clone-stamp-canvas-brush-picker --strict`.
- [x] 3.2 `python3 scripts/check-test-plan.py`.
- [x] 3.3 Build and run `ctest --test-dir build -R '^tst_canvas_context_menus$'`.
- [x] 3.4 `bash scripts/verify-fast.sh`. It reaches one pre-existing,
  unrelated failure in `pictura-render::oracle::imagemagick_fixtures_reproduce`
  (`ramp_Divide.rgba` bytes differ); the Qt and Rust suites, including the new
  cases, pass.
