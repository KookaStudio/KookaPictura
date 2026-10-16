# Design: frameless-dialogs-keyboard-shortcuts

## Dialog chrome

`installDialogChrome()` installs one `QApplication` event filter (idempotent).
On a top-level `QDialog`'s `Polish` it sets `Qt::FramelessWindowHint` (before
the native window exists, so nothing is re-created) and adds two children:

- `dialogTitleBar`: title label and a `panel.close` button that calls
  `reject()`. A press starts `QWindow::startSystemMove()`, which also works on
  Wayland where a client `move()` is ignored. A manual move is the fallback
  when the platform refuses (offscreen).
- `dialogFrame`: a mouse-transparent outline drawn over the dialog's edge.

The chrome is not in the layout. The filter adds the title height to the
dialog layout's top margin (the original margin is remembered on the layout),
and moves both children on `Resize`. This leaves every dialog's layout
untouched, so 60-odd dialogs need no edits. `Show` re-checks the margin
because `QMessageBox` rebuilds its layout when its text changes.

Resizable dialogs (minimum ≠ maximum size, layout not `SetFixedSize`) get
edge resizing: within 5 px of an edge the cursor changes, and a press calls
`startSystemResize(edges)`.

Exempt: windows that are already frameless, popups, non-window dialogs, and a
native `QFileDialog` / `QColorDialog`, whose frame belongs to the platform. A
picker counts as Qt-drawn when `DontUseNativeDialog` is set or its widget tree
(a `QDialogButtonBox`) already exists at polish: with no native helper, Qt
builds the widgets and polishes the picker inside its constructor, before the
caller can set the option. (Offscreen has no helper, so the native branch has
no automated check.)

## Shortcut overrides

`CommandRegistry` keeps each command's default bindings and its current
binding list. `setShortcuts(id, list)` updates the entry and the live
`QAction` (`QAction::setShortcuts`). `shortcutOverrides()` returns only the
ids whose list differs from the default; an empty list means "none". Tool
letters follow the same pattern in `tool_catalog.cpp` (`toolShortcut`,
`setToolShortcut`, `toolShortcutOverrides`). `toolGroupForKey` and the
toolbox read the effective letter, and the frame rebuilds its per-letter
`QShortcut`s after a change.

The session JSON gains `shortcuts` (`id → [portable text, …]`) and
`toolShortcuts` (`tool name → letter`). On load, unknown ids are dropped, so a
renamed command loses only its override.

## Dialog

Laid out after CS6: Keyboard Shortcuts / Menus tabs (Menus disabled),
Shortcuts For and Set on one row, then the tree, the button column (Accept,
Undo, Use Default, Add Shortcut, Delete Shortcut, Summarize), the information
area, and OK / Cancel beside the tabs.

`KeyboardShortcutsDialog` edits a working copy. Each command binding is a
row: the command's label sits on its first row, and a command with no
binding gets one blank row. Selecting a row puts a focused key field in its
Shortcut cell, so the next chord is captured, not eaten by the tree's
type-ahead. The chord stays pending: Accept commits it; choosing another row
also commits it unless it conflicts.

A conflict (another command holds the chord; for tools, a tool outside the
edited tool's group holds the letter) is reported in the information area,
with Accept and Go To Conflict and Undo Changes beside it. Leaving the row
drops an unaccepted steal. Undo drops a pending chord first, then pops
accepted edits (whole working-copy snapshots).

Conflicts are checked across both lists, because a tool letter L also binds
Shift+L (the group's cycle key) in the window: two shortcuts on one key are
ambiguous in Qt, and neither fires ("Ambiguous shortcut overload"). A menu
chord equal to L or Shift+L conflicts with the group holding L; accepting
takes L from that group. A tool letter conflicts with any command holding L
or Shift+L; accepting removes those bindings. Keys the workspace binds itself
(`workspaceKeys()`: brush `[ ]`, D, X, F, Q, Tab, Delete, Backspace,
Shift+F1/F2) are refused in both lists. As a backstop for stores written
before these checks, `rebuildToolShortcuts()` skips any tool key a menu
command holds.

Tool shortcuts accept only a single letter A–Z. OK commits any pending chord and writes the working copy
through the registry and the catalog. The frame then persists it and
refreshes the toolbox.

## Ceilings (`ponytail:`)

- Edge resizing only fires where the dialog itself, not a child, is under the
  pointer (the layout margin). A dialog with zero margins resizes only from its
  exposed edges.
- One implicit user set over the defaults: no named sets, Summarize, Panel
  Menus, legacy channel keys, or Menus tab.
- `workspaceKeys()` lists the window's own `QShortcut`s by hand; a test
  fails when the window gains a key missing from it.
