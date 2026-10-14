# Proposal

## Why

The design system lives in the global stylesheet plus Fusion and the app's
custom-painted widgets. A generated `design-system.html` mirrors that by hand,
so it drifts and cannot show the real rendering or live interaction. The
application is the only faithful representation of its own design system, so it
should be browsable from inside the app.

## What Changes

- **In-app design-system browser.** Add `Help ▸ Developer ▸ Design System…`: a
  modeless tool window that presents the design system as a storybook. It is the
  real UI by construction — the pages are built from the application's own
  widgets and styled by the global stylesheet — so it is 1:1 with the running
  interface, live and interactive.
- **Structure.** A filterable story list grouped into Foundations, Shell,
  Controls, and Patterns; a per-story title, description, and QSS selector; a
  token readout for the current brightness level; and a brightness control that
  previews a level live and restores the user's level when the window closes.
- **Coverage.** Every primitive the global stylesheet defines appears on some
  story, and a Qt Test builds every page and asserts the primitives are present.
- **No external artifact.** The browser replaces the standalone HTML guide as
  the way to review the design system; no screenshotting, no build step to view.

## Capabilities

### New Capabilities

<!-- None: this change extends the existing capability. -->

### Modified Capabilities

<!-- None: this change only adds a requirement. -->

## Impact

- New `crates/pictura-app/cpp/design_system_dialog.{h,cpp}` and
  `design_system_dialog_pages.cpp` (the story registry), registered in
  `CMakeLists.txt`.
- `commands.h`, `command_tree.cpp`, `frame_menus.cpp`, `frame.h`: the
  `Help ▸ Developer ▸ Design System…` command and its modeless window.
- `theme.{h,cpp}`: a `Theme::currentLevel()` accessor so a preview can restore
  the level it found.
- New `crates/pictura-app/cpp/tests/tst_design_system_dialog.cpp`.
- No engine-crate, bridge, or dependency change.
