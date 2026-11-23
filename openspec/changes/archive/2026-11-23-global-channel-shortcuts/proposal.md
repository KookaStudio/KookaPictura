# Proposal

Tracking issue: #1553.

## Why

The Channels panel labels its composite and working colour rows `Ctrl+2` through
`Ctrl+5`, but nothing in the window handles those keys. The labels are purely
decorative, so the shortcuts do nothing whether the canvas, a panel, or the
Channels list has focus.

## What Changes

- The main window handles `Ctrl+2` through `Ctrl+5` as document-wide channel
  shortcuts, including while the canvas has keyboard focus.
- `Ctrl+2` shows all colour channels; `Ctrl+3`, `Ctrl+4`, and `Ctrl+5` toggle
  the Red, Green, and Blue rows with the same rules as the Channels panel's eye
  column (the last visible channel stays visible).
- The panel resolves the key against the row shortcut it already publishes, so
  the labels and the behaviour cannot drift.

## Capabilities

### Added Requirements

- `ui/channels-panel`: the composite and working colour channel shortcuts act
  globally, not only while the Channels panel has focus.

## Impact

- `crates/pictura-app/cpp/panels/channels_panel.h` and `.cpp`: a shortcut-to-row
  toggle entry point.
- `crates/pictura-app/cpp/frame_keys.cpp`: window key dispatch for `Ctrl+2` to
  `Ctrl+5`.
- `crates/pictura-app/cpp/tests/tst_channels_panel.cpp`: canvas-focus coverage.
