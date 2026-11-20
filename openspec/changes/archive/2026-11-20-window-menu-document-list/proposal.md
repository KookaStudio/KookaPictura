# Proposal

## Why

Photoshop CS6 lists every open document window at the bottom of the Window menu,
with the active window checked, so a window can be found and activated without
hunting through tabs or overlapping floats. Kooka Pictura's document-tabs and
windows bundle (#317) built the window registry but left this list out as a
non-goal (#923). `docs/02-ui-ux/menus.md` already describes the list, so the
long-form contract and the app disagree; `ROADMAP.md` tracks the missing item.

## What Changes

- The Window menu gains a final section listing every open document window,
  below the UI Size row and separated from it by a separator.
- Entries follow the `Ctrl+Tab` cycle order: tab windows in strip order first,
  then floating windows and tile panes in the order they entered that host.
- Each entry shows the window's full label, including the `:N` ordinal suffix of
  a document's later windows.
- The active window's entry is checked.
- Choosing an entry activates that window, whichever host holds it.
- The list is rebuilt whenever the Window menu opens, so open, close, rename,
  and host moves are reflected; with no documents open the menu shows neither
  entries nor a list separator.
- **BREAKING (spec contract)**: `ui/ui-scaling` no longer states that the UI Size
  row is the Window menu's last row. The open-document list follows it, so the
  requirement and its placement scenario are updated.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `document/document-windows`: adds the requirement that the Window menu lists
  every open document window as its last section, with cycle order, full labels,
  the active check, activation across hosts, and refresh on each menu open.
- `ui/ui-scaling`: modifies `Window menu scale commands` so the UI Size row sits
  below Options with the open-document list, when non-empty, below the row as
  the menu's last section.

## Impact

- `crates/pictura-app/cpp/menu_registrar.cpp`,
  `crates/pictura-app/cpp/menu_registrar.h`, and
  `crates/pictura-app/cpp/menu_window_arrange.cpp`: dynamic list built from
  `DocumentRegistry::windows()`, labels from `fullLabel()`, activation through
  `activateDocWindow()`.
- `crates/pictura-app/cpp/tests/tst_document_windows.cpp`: new Qt Test cases.
  `tst_ui_scale.cpp` keeps its existing placement case, which still holds with
  no documents open.
- `ROADMAP.md`: the `#923` item is ticked.
- No command-catalogue, shortcut, session, or documentation changes: the window
  rows are transient menu content, not commands, and `docs/02-ui-ux/menus.md`
  already specifies the behavior.
