# Tasks

## 1. State and persistence

- [x] 1.1 `CropSettings` on `CropOptions`; leaving Classic checks Show Cropped Area and Auto Center Preview; persisted under `toolOptions.crop`. Qt Test `tst_crop_settings::settingsPersist`.

## 2. Canvas

- [x] 2.1 `ImageView::CropShield` and `cropShieldAlpha()` replace the hardcoded tint; the crop handler pushes the style. Qt Tests for off, opacity, colour, hidden outside and Auto Adjust.
- [x] 2.2 Auto Center Preview pans to the box when a gesture settles. Qt Test `autoCenterPansToTheBox`.

## 3. Menu and keys

- [x] 3.1 Compact Settings menu with the shield panel and colour picker; `P` / `H` through `ToolController::handleCropSettingsKey`. Qt Tests `menuListsItsItems`, `classicModeGatesTheModernItems`, `keysToggleClassicAndShowArea`.

## 4. Ship

- [x] 4.1 Build, run `tst_crop_*`, `verify-fast.sh`, archive the change, tick `ROADMAP.md`.
