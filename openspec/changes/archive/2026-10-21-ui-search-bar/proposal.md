# Shift+Space search bar for every UI entry point

## Why

Pictura reproduces CS6's deep menu tree, and a command such as
`Edit > Transform > Warp` is hard to find for a user who does not already know
where it lives (#788). CS6 has no command search. This is a post-CS6 feature
added by owner decision and is labelled non-parity.

## What Changes

- Add `Help > Search…` (`help.search`, default `Shift+Space`), a registry
  command, so it shows in the menu, `list_commands`, and Edit > Keyboard
  Shortcuts, where it can be rebound.
- Add a search overlay over the canvas: a query field and a result list. Each
  row shows the entry's location as a breadcrumb (`Edit > Transform > Warp`)
  and its shortcut.
- Index every registry command, every tool, every panel, every Preferences
  page, and a hand-kept table of key panel and options-bar controls.
- Enter or a click runs the selected entry. Alt+Enter reveals where it lives:
  the menu chain opens on the item, or the panel, control, or toolbox slot is
  shown and outlined. Stub and disabled entries are listed greyed and do not
  run; they can still be revealed.
- Shift+Space does nothing while a text field or a type edit has focus, bare
  Space still pans, and Shift+Space never starts a Space pan.
- Show a `Search  Shift+Space` hint in the status bar that follows the live
  binding.

## Capabilities

- Added: `ui/search-bar`

## Impact

- Code: C++ `commands.{h,cpp}`, `command_tree.cpp`, `frame.h`,
  `frame_menus.cpp`, `frame_build.cpp`, `frame_keys.cpp`, `toolbox.h`,
  `theme.cpp`, `options_bar.cpp`, `panels/layers_panel.cpp`; new
  `search_index.{h,cpp}`, `search_controls.cpp`, `search_bar.{h,cpp}`,
  `frame_search.cpp`; Qt Test `tests/tst_search_bar.cpp`; `CMakeLists.txt`
  and the test CMake list.
- Docs: `docs/02-ui-ux/menus.md`, `ROADMAP.md`, `docs/dev/STATE.md`.
