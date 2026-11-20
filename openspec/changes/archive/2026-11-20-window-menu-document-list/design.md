# Design

## Context

See `proposal.md` — Why.

The Window menu is built once by `CommandRegistry::buildMenuBar()` from the
static Rust command catalogue, then decorated in `MenuRegistrar`:

- `buildUiSizeMenu()` inserts the UI Size widget row before the hidden
  `window.uiSize` action, so the row is currently the menu's last visible item.
- `menu_recent.cpp` shows the established pattern for a dynamic list: keep the
  `QMenu*`, connect `aboutToShow`, and own the transient `QAction`s in a
  `QList<QAction*>`.
- `DocumentRegistry::windows_` already holds every window in `Ctrl+Tab` cycle
  order: tabs first, then floats and tile panes in host-entry order. That order is
  documented as, and exercised for tabs and floats in, the cycle contract; the new
  order case adds tile-pane coverage.
- `fullLabel(window)` returns the display name, `:N` ordinal, zoom, and
  mode/bit-depth suffixes; `DocumentRegistry::activateDocWindow(window)`
  activates a window in any host.

## Goals / Non-Goals

**Goals:**

- Render the list from the registry on demand, with no duplicate window model.
- Make the list the menu's last visible section, after the UI Size row.
- Keep the implementation inside the menu layer; no registry, catalogue, or
  session changes.

**Non-Goals:**

- Command-registry entries, keyboard shortcuts, or shortcut-editor rows for the
  window rows.
- Help ▸ Search indexing of the window rows.
- Persisting the list, limiting its length, or showing file paths.
- Custom menu sets (Edit ▸ Menus) honoring the list.

## Decisions

### D1. Transient `QAction`s owned by `MenuRegistrar`, not command entries

Each open window becomes a `QAction` parented to the Window menu and tracked in
`MenuRegistrar::windowDocumentActions_`. The command catalogue is static and
keyed by stable ids; window rows have neither a stable id nor a shortcut, and
mixing them into `CommandRegistry` would leak them into `describe()`, the
keyboard-shortcuts editor, and replay. Alternative considered: one placeholder
command plus a dynamic submenu — an indirection with no consumer.

### D2. Rebuild on `QMenu::aboutToShow`

The menu's `aboutToShow` handler deletes the previous rows and their separator
— both tracked in `windowDocumentActions_` — and appends a separator plus one
row per `documents_->windows()`, only while at least one window exists. This
mirrors `refreshRecentMenu()` and covers every update the issue names — open,
close, rename, host move — plus zoom and mode changes that feed `fullLabel()`
but emit no signal. Alternative considered: connect
`windowsChanged`, `documentsChanged`, `activeDocumentChanged`, and
`documentStateChanged`; it still misses zoom changes and needs each signal's
semantics kept in step with the label.

### D3. The list is the last section, below the UI Size row

Rows are appended with `menu->addActions()`, after the hidden `window.uiSize`
readout action that follows the UI Size widget row; hidden actions do not create
a visual gap. A leading separator is part of the appended block, so the UI Size
row stays visually separated from the list. This deliberately modifies
`ui/ui-scaling`'s former "UI Size row is the last row" rule; the alternative
(list above the footer) was rejected in favor of the CS6 bottom placement.

### D4. Order, label, and activation come from the registry

The order is exactly `DocumentRegistry::windows()`, which already matches the
`document-tabs` cycle-order contract. The row text is `fullLabel(window)` with
`&` doubled so a file name cannot become a mnemonic, matching the existing
`New Window For` label provider. Checked state is `window == activeWindow()` at
build time. A trigger first checks the window is still registered (the flash of
a background close while the menu is open), then calls `activateDocWindow()`.

### D5. Test hooks

Each row carries `objectName` `windowDocumentEntry` and the list separator
carries `windowDocumentSeparator`, so Qt Test can select both out of the full
Window menu deterministically.

## Risks / Trade-offs

- [A window closes after the menu is built but before its row is triggered] →
  the trigger checks `windows().contains(window)` before activating, and the
  next open rebuilds. This is the same guard `showTabMenu` uses, so a reused heap
  address while the menu is open is accepted as there.
- [The row text changes while the menu is open, e.g. a zoom change] → rows keep
  the label captured at open time, which is the state the user saw; the next
  open refreshes it.
- [Two documents share a display name] → their rows look identical, as their
  tabs do today; paths stay out of the menu to match CS6.
- [`windowMenuControlSitsBelowOptions` in `tst_ui_scale` asserts the UI Size row
  is the last visible row] → it opens no documents, so the list is empty there
  and the assertion still holds; the new `document-windows` cases prove the list
  follows the row when documents exist.

## Migration Plan

None: no stored state, catalogue id, or session format changes. `ROADMAP.md`
gets its `#923` item ticked when the change ships.
