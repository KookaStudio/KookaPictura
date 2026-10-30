# Tasks

## 1. Test plan

- [x] 1.1 Write `test-plan.md` with one row per spec scenario

## 2. Cursor asset

- [x] 2.1 Add `assets/cursors/cursor.colorPickerField.svg` and register it in
  `assets/pictura.qrc`

## 3. Field pointer

- [x] 3.1 `color_picker_dialog.cpp`: `ColorPlane` uses the circular cursor,
  falling back to the crosshair

## 4. Test

- [x] 4.1 `tst_color_picker::fieldCursorIsACircle`

## 5. Verify

- [x] 5.1 Build, then run `tst_color_picker`, `openspec validate --all
  --strict`, and `bash scripts/verify-fast.sh`
