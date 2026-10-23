# Change: UI size menu controls and high-scale window resizing

## Why

The Window menu exposes Enlarge, Decrease, and Reset UI Size as separate items.
Stepping more than one step forces the user to reopen the menu for every click.
At larger UI scales the tools column's unscrollable content also raises the
frame's minimum height above the screen, pushing the window decorations off
screen and preventing the frame from being resized or maximized.

## What Changes

- Replace the separate Enlarge and Decrease menu items with one UI Size row
  stacked as a caption (`UI size`), a step row with the percentage readout and
  minus/plus buttons, and a reset button below. Clicking a button keeps the menu
  open so several steps can be taken without reopening it, and the open menu
  resizes with the scale so its rows are never clipped.
- Keep the Enlarge, Decrease, and Reset commands as hidden shortcut actions so
  keyboard shortcuts, search, and the shortcut editor still address them.
- Bind the three commands: `Ctrl+Alt++` enlarges, `Ctrl+Alt+-` decreases, and
  `Ctrl+Alt+0` resets to 100 percent.
- Give the tools column a vertical scroll area so its content no longer sets the
  frame's minimum height. The frame stays resizable and maximizable at every UI
  scale.

The Interface Preferences UI Size control and the persisted percentage are
unchanged.

## Impact

- Affected specs: `ui/ui-scaling`
- Affected code: `crates/pictura-app/cpp/frame_ui_scale.cpp`,
  `frame_menus.cpp`, `command_tree.cpp`, `panels/panel_column.cpp`,
  `panels/panel_column.h`, `tests/tst_ui_scale.cpp`
