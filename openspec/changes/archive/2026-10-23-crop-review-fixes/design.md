# Design

- **Keys.** The tool letters are window `QShortcut`s (`rebuildToolShortcuts`),
  and a real key press reaches them before `keyPressEvent`. Their handler first
  offers the key to `ToolController::handleCropKey(key, shift)`, and
  `keyPressEvent` keeps the same call for a letter that has no tool shortcut.
  The grid menu shows `O` and `Shift+O` as text after a tab rather than as
  action shortcuts, so they cannot make the tool letter's shortcut ambiguous.
- **Show mode and orientation.** These are `CropSettings::overlayShow` and
  `CropSettings::overlayOrientation`, so they persist through the existing
  `toolOptions.crop` JSON. `CropOptions::cycleOverlay` and
  `cycleOverlayOrientation` hold the cycle rules, which keeps `tools.cpp` under
  its size cap. The tool hands the canvas overlay `-1` when the guides are
  hidden. The canvas mirrors the guides about the box centre (bit 0
  left-right, bit 1 top-bottom).
- **Runtime patch.** `applyToolOptionsJson` calls `setSettings`, as the other
  groups' `update` does. `applySettings` is removed. At startup the notify is
  harmless, because the session save is connected afterwards.
