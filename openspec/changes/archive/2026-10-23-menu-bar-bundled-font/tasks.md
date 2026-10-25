# Tasks

## 1. Test first

- [x] 1.1 Add `tst_ui_scale::menuBarUsesBundledFont` covering the menu bar and
  Window menu family and pixel size at startup and at 150 percent.

## 2. Fix

- [x] 2.1 Set the main window font from the application font after
  `applyBundledUiFont` in the constructor, before the menu bar exists.
- [x] 2.2 Repeat the window-font update in `setUiScalePercent` so the existing
  menu bar follows live scale changes.

## 3. Verification

- [x] 3.1 Run the app build and `tst_ui_scale`.
- [x] 3.2 Run `openspec validate --all --strict` and archive the change in the
  shipping PR.
