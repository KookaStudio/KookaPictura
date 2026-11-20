# Spec Delta

## ADDED Requirements

### Requirement: Window menu document list

The Window menu SHALL list every open document window as its last section,
below the UI Size row and separated from the row by a separator. The list SHALL
follow the `Ctrl+Tab` cycle order: the tab-strip windows in their visual order
first, then the floating windows and tile panes in the order they entered that
host. Each entry SHALL show the window's `fullLabel()` with literal ampersands
escaped for the menu: the display name and, unless the document is loading, the
`:N` ordinal suffix for a document's later windows, the zoom suffix, the
mode/bit-depth suffix, and the watermark marker. The entry for the active window
SHALL be checked.
Choosing an entry SHALL activate that window, whatever host holds it. The list
SHALL reflect the current windows whenever the Window menu opens, including
documents opened, closed, renamed, or moved between hosts. With no document
open the Window menu SHALL NOT show a list entry or a list separator.

#### Scenario: The list follows the cycle order [dtw_menu_list_order]

- **WHEN** documents `A` and `B` are tabbed, `B` is moved before `A` in the strip,
  `C` floats, and the Window menu is opened; then the windows are tiled and the
  menu is opened again
- **THEN** the entries read the labels of `B`, `A`, `C` in that order, tabs first,
  and the tiled menu reads the same cycle order

#### Scenario: Entries show the full label [dtw_menu_list_label]

- **WHEN** a document has two windows and the second is the active one
- **THEN** its entry is the window's `fullLabel()`, whose display-name part ends in
  `:2` and is followed by the zoom and mode/bit-depth suffixes, and the first
  window's entry has no ordinal

#### Scenario: The active window is checked [dtw_menu_list_active]

- **WHEN** the Window menu is opened while two documents are open, and again
  after `Ctrl+Tab` makes the other window active
- **THEN** the active window's entry is checked each time and the other is not

#### Scenario: Choosing an entry activates its window [dtw_menu_list_activate]

- **WHEN** a tab window and a floating window are open and the user chooses each
  one's entry in turn, and then the windows are tiled and the user chooses a
  pane's entry
- **THEN** each chosen window becomes the active window in its host

#### Scenario: The list follows document changes [dtw_menu_list_updates]

- **WHEN** the Window menu has been opened once, a document is renamed, another
  is opened, a third moves from the tab strip to a floating window, and a fourth
  is closed, and the menu is opened again
- **THEN** the renamed entry shows the new name, the opened document has an
  entry, the moved window appears after the tabs, and the closed document has no
  entry

#### Scenario: The list is the menu's last section [dtw_menu_list_last]

- **WHEN** at least one document is open and the Window menu is opened
- **THEN** the UI Size row is below the Options item and separated from the
  document list by a separator, and the document list is the menu's last section

#### Scenario: No documents, no list [dtw_menu_list_empty]

- **WHEN** no document is open and the Window menu is opened
- **THEN** the menu shows no document entry and no list separator after the UI
  Size row
