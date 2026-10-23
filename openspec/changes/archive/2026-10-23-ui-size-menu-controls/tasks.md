# Tasks

## 1. Tests first

- [x] 1.1 Extend `tst_ui_scale` with the single menu row, button stepping,
  shortcut bindings, and high-scale resize checks.

## 2. Menu and shortcuts

- [x] 2.1 Replace the visible Enlarge/Decrease/Reset rows with one UI Size
  widget row while keeping the commands as hidden actions.
- [x] 2.2 Bind `Ctrl+Alt++`, `Ctrl+Alt+-`, and `Ctrl+Alt+0` and keep the
  readout and button enablement live.

## 3. Window resizing

- [x] 3.1 Wrap the tools content in a vertical scroll area so its height no
  longer pins the frame minimum.
- [x] 3.2 Verify the frame can be resized and maximized at 200 percent.

## 4. Verification

- [x] 4.1 Run `cargo fmt --all`, the app build, and the Qt tests.
- [x] 4.2 Run `openspec validate --all --strict` and archive the change in the
  shipping PR.

## 5. Menu layout and live resize

- [x] 5.1 Stack the UI Size row as caption / step row / reset and rename the
  caption to `UI size`.
- [x] 5.2 Resize the open Window menu when the scale changes so its rows are not
  clipped.
- [x] 5.3 Move the UI Size row below the Window menu's Options item.
