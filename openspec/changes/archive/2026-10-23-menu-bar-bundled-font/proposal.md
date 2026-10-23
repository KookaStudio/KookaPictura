# Change: Use the bundled UI font in the menu bar

## Why

The main window is created before the bundled UI font is installed, so it keeps
the platform's default Sans Serif face. `QMenuBar` inherits the window's font
rather than the application font, so the menu bar renders in a different face
(and, on HiDPI displays, a different pixel size) from the menu items and the
rest of the chrome.

## What Changes

- Set the main window's font to the bundled application font after
  `applyBundledUiFont` and on every live UI-scale change, before the menu bar is
  created and while it exists.
- The menu bar and its menus then resolve to Noto Sans at the current scale,
  matching the menu items and the chrome.

## Impact

- Affected specs: `ui/bundled-fonts`
- Affected code: `crates/pictura-app/cpp/frame.cpp`,
  `crates/pictura-app/cpp/frame_ui_scale.cpp`,
  `crates/pictura-app/cpp/tests/tst_ui_scale.cpp`
