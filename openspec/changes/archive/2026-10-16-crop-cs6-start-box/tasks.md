# Tasks

## 1. Options bar

- [x] 1.1 Remove the `Content-Aware` checkbox; Qt Test asserts it is absent.

## 2. Start box

- [x] 2.1 `reset()` starts both modes in the preview state.
- [x] 2.2 The preview draws its handles (no guides); a handle press resizes and activates it; the hover cursor shows resize over a handle.
- [x] 2.3 Qt Tests: both modes start in preview, the preview corner bracket is drawn, a corner/edge handle drag resizes; existing draw-a-box setups start inside the start box.

## 3. History name

- [x] 3.1 `crop_to` / `straighten_crop` take the History label; the tool passes `Crop: <overlay>` from `CropOptions::overlayNames()`, shared with the grid menu.
- [x] 3.2 Qt Test: commits under Rule of Thirds, Grid, and Golden Spiral name their state; the `crop_tool` self-test expects the overlay name.

## 4. Rotate feedback and overlay icons

- [x] 4.1 `ImageView::setCropRotating` draws the rotate grid; the rotate gesture shows `∠: <angle>°` through the drag readout and clears both on release, cancel, and deactivate.
- [x] 4.2 Six `crop.overlay.*` custom icons (map, qrc, provenance table); menu entries carry them and the button follows the selection; `crop.grid` removed.
- [x] 4.3 Qt Tests: rotate grid and readout appear and clear; every overlay has a distinct icon and the button follows it.
