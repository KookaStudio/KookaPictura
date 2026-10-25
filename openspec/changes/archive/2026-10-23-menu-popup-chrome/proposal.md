# Change: Round menu popups and give icons breathing room

## Why

Menu popups currently have square outlines and icon-bearing rows draw their
icons almost against the left edge. The icon column reads as cramped, and the
square popup does not match the rounded controls and panels around it.

## What Changes

- Give every `QMenu` popup a 4 px outline radius (scaled with the UI size) and a
  2 px inset so item highlights do not spill into the rounded corners.
- Make menu popups translucent outside the rounded outline so the corners are
  genuinely cut, not just painted over the square window background.
- Move menu icons and check marks 10 px in from the item edge, leaving clear
  space to the outline.

## Impact

- Affected specs: `ui/design-system`
- Affected code: `crates/pictura-app/cpp/theme.cpp`,
  `crates/pictura-app/cpp/tests/tst_design_system.cpp`
