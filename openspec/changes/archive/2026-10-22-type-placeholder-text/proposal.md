# Proposal: Placeholder text and the Warp Text icon

## Why

Owner review of #684 asked for two follow-ups on the same branch:

- The type options bar's Warp Text button reused Free Transform's warped-grid
  icon. Photoshop marks the button with a "T" standing on an arc, and the
  button should read as Warp Text.
- A new type object should start filled with placeholder text, "Lorem Ipsum
  Pictura", already selected, so typing replaces it. This is not CS6 behavior
  (Photoshop added it in CC); it is an owner decision and labelled non-parity.

## What changes

- A custom `options.warpText` icon (a T over an arc on the 24 × 24 Lucide
  grid) for the four type tools' Warp Text buttons.
- A new type session from the Horizontal or Vertical Type tool, by a click or
  a dragged box, starts with "Lorem Ipsum Pictura" fully selected. Typing
  replaces it; committing it untouched creates a layer with that text. Type
  Mask sessions, and sessions that reopen an existing type layer, start as
  before.
- `Edit > Preferences > Type` becomes a real page with **Fill New Type Layers
  With Placeholder Text**, on by default and saved with the session.

## Capabilities

- Added: `tools/type-placeholder-text`.
- Modified (added requirement): `tools/warp-text`.

## Impact

- C++ only: `tool_type.cpp` (session start), `tools.{h,cpp}` (the shared
  flag), `preferences_dialog.{h,cpp}`, `frame_preferences.cpp`,
  `frame_session.cpp`, `frame.cpp`, `session.{h,cpp}`, `command_tree.cpp`,
  `commands.h`, `search_index.cpp`, `options_bar_type.cpp`, and the icon
  asset plus its `lucide-map.json` and `pictura.qrc` entries.
- Tests: `tst_warp_text` and a new `tst_type_placeholder` suite.

## Out of scope

- `Type > Paste Lorem Ipsum` (a separate stub).
- Other Type preferences (Middle Eastern, East Asian, font names).
