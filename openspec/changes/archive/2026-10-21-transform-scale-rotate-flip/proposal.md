# Wire Edit > Transform Scale/Rotate/Again and the exact rotations and flips

## Why

`command_tree.cpp` registers the full `Edit > Transform` submenu, but `Scale`,
`Rotate`, `Again`, `Rotate 180°`, the two quarter turns, and both flips are
inert leaves. Only `Free Transform` and the document-level `Image > Image
Rotation` commands are wired, so the menu promises commands that do nothing.

## What Changes

- Make the `Edit > Transform` submenu a live command group in the CS6 order and
  with the screenshot's labels: Again, Scale, Rotate, Skew, Distort, Perspective,
  Warp, Rotate 180°, Rotate 90° Clockwise, Rotate 90° Counter Clockwise, Flip
  Horizontal, Flip Vertical, separated into four groups.
- Add constrained `Scale` and `Rotate` sessions on top of the existing
  Free Transform session: Scale drives the eight scale handles and has no rotate
  band; Rotate drives every handle as a rotation and leaves the scale untouched.
- Apply `Rotate 180°`, `Rotate 90° Clockwise`, `Rotate 90° Counter Clockwise`,
  `Flip Horizontal`, and `Flip Vertical` immediately to the active layer, or to
  the selected pixels when a selection is active. With a session already open
  they update the live box instead of committing.
- Record one history state per immediate command, named for the command.
- Add `Transform Again` (`Shift+Ctrl+T`): replay the last committed repeatable
  similarity transform, including its relative reference point and translation,
  and record one `Transform Again` state.

## Capabilities

- Added: `tools/transform-commands`

## Impact

- Code: `pictura-app` `cxxqt_object/state.rs`,
  `cxxqt_object/impl_transform/{session,dispatch,bridge}.rs`; C++
  `commands.h`, `command_tree.cpp`, `frame_menus.cpp`, `frame.cpp`,
  `tool_transform.cpp`, `tools.h`, `options_bar_transform.cpp`; Qt Test
  `tests/tst_transform_commands.cpp` and the test CMake list.
