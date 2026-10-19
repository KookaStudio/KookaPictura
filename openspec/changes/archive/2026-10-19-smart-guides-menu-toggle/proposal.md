# Proposal: smart-guides-menu-toggle

## Why

Issue #737 shipped smart alignment guides always on, deferring the
`View > Show > Smart Guides` toggle. Users need a way to stop the magenta guides
and magnetic snapping, as in Photoshop, without turning the feature off in code.

## What Changes

- Add a checkable `View > Show > Smart Guides` command between `Count` and
  `Slices`, on by default.
- The flag is global view state: every open canvas follows it, and it persists
  in the session across restarts.
- While off, Move and Type drags neither snap nor draw smart guides, and any
  active guides clear immediately.
- A `tst_smart_guides::smartGuidesToggle` case covers the menu, a drag while
  off, and the persisted setting.

## Capabilities

### New Capabilities

<!-- None. -->

### Modified Capabilities

- `tools/smart-guides`: adds the visibility-toggle requirement.

## Impact

- `crates/pictura-app/cpp/commands.h`, `command_tree.cpp` — the command.
- `crates/pictura-app/cpp/frame.h`, `frame_guides.cpp`, `frame_session.cpp` —
  the global flag and its application to each canvas.
- `crates/pictura-app/cpp/session.{h,cpp}` — persistence.
- `crates/pictura-app/cpp/tests/tst_smart_guides.cpp` — the test.
- No new dependencies.
