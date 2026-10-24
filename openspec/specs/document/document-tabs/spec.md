# document-tabs Specification

## Purpose
The tabbed document area, active-document targeting, the modified-marker title, and close behaviour.

## Requirements

### Requirement: Tabbed document area
The system SHALL show each open document in at least one document window, and
each window SHALL have its own canvas. A document window hosted in the tab strip
SHALL be a tab, and tabs SHALL be closable and switchable. Opening or creating a
document SHALL add a document window for it and make that window active. With
`Open Documents As Tabs` on and no tile layout active, that window SHALL be a
tab; the `document/document-windows` capability defines the other hosts. A
document that is still loading SHALL be shown as a loading tab, as the
`document/document-lifecycle` loading-tab rules require, whatever host it gets
once it is ready. Every path that adds a document SHALL follow this rule: New,
Open and its variants, a workspace file drop, Duplicate, and a smart object's
Edit Contents.

#### Scenario: Opening a document adds a tab
- **WHEN** a document is opened while another is already open and `Open
  Documents As Tabs` is on
- **THEN** a second tab appears and each tab shows its own composited document

#### Scenario: Switching tabs changes the canvas
- **WHEN** the user selects a different tab
- **THEN** the canvas and status bar show that document

### Requirement: Active-document targeting
Menu commands, the editable dock controls, and the status bar SHALL operate on
the active document. The active document SHALL be the document of the active
document window, which is the most recently activated window, whether that
window is a tab, a floating document window, or a tile pane. Activating a window
in any host SHALL have the same effects as selecting its tab: the panels and the
status bar retarget, a live transform session on another document is cancelled
as a tab switch cancels it, and, for the loading-tab rules, it counts as the user
selecting another tab. With no document open, document-requiring commands SHALL
be disabled.

#### Scenario: Commands target the active document
- **WHEN** the user runs a command with several documents open
- **THEN** only the active document is changed

#### Scenario: No documents open
- **WHEN** the last document is closed
- **THEN** document-requiring commands are disabled and the application does not crash

#### Scenario: A floating window becomes the target [dtw_target_float]
- **WHEN** one document is tabbed, another is in a floating document window,
  and the user activates the floating window
- **THEN** the panels, status bar, and menu commands target the floating
  window's document

#### Scenario: Activating a floating window during a load [dtw_target_float_loading]
- **WHEN** an open is in flight in a loading tab and the user activates a
  floating document window before it finishes
- **THEN** the floating window stays active when the open succeeds

### Requirement: Tab title and modified marker

Each tab SHALL show the document's file name, or an untitled name when it has no
path, and SHALL append a modified marker while the document is dirty. The window
title SHALL track the active document. A document whose mode and bit depth are
known SHALL show the title as `name (Mode/Bits)`, for example
`photo.jpg (RGB/8)`, where `Mode` is the document's color mode (such as `RGB`,
`Grayscale`, or `CMYK`) and `Bits` is its bit depth. An opened raster file SHALL
use its file name including the file extension as the tab's display name rather
than a generated `Untitled-N` name or a name stripped of its extension. A
document with no name SHALL continue to show a generated untitled name.

#### Scenario: Untitled name

- **WHEN** a document has no path
- **THEN** its tab shows a generated untitled name

#### Scenario: Modified marker

- **WHEN** the active document is modified
- **THEN** its tab title carries the modified marker

#### Scenario: Marker clears on save

- **WHEN** the document is saved
- **THEN** the modified marker is removed

#### Scenario: The title carries the mode and bit depth [ldt_mode_bits]

- **WHEN** a document with a known mode and depth is shown
- **THEN** its tab title is `name (Mode/Bits)` plus the modified marker while
  dirty

#### Scenario: An opened raster keeps its full display name [ldt_display_name]

- **WHEN** a raster file such as `image01.jpg` is opened
- **THEN** the tab shows the file name with its extension and its mode and bit
  depth, for example `image01.jpg (RGB/8)`, rather than `Untitled-N` or a name
  without the extension

#### Scenario: A renamed untitled document updates its title [ldt_rename]

- **WHEN** an untitled document's name changes
- **THEN** its tab title reflects the new name if the title is shown

### Requirement: Closing a tab
The system SHALL close the selected document window when its tab close control
is used. `File > Close` SHALL close the active document window under the same
rules. When that window is the last window of its document, the system SHALL
apply the unsaved-changes prompt and close the document. When the document has
other windows, the system SHALL close only that window, without a prompt.
Closing one document SHALL NOT affect the others.

#### Scenario: Close the active tab
- **WHEN** the user closes a tab whose document is unmodified
- **THEN** that tab is removed and the remaining documents stay open

#### Scenario: Close with unsaved changes
- **WHEN** the user closes a modified document and the prompt is cancelled
- **THEN** the tab remains open

#### Scenario: Closing one of several windows of a document does not prompt [dtw_close_extra_window]
- **WHEN** a modified document has two windows and the user closes one of them
  with its close control
- **THEN** no unsaved-changes prompt appears, the other window stays open, and
  the document stays modified

#### Scenario: File > Close closes only the active window [dtw_file_close_window]
- **WHEN** a modified document has two windows, one of them is active, and the
  user chooses `File > Close`
- **THEN** no prompt appears, only the active window closes, and the document
  stays open in its other window

### Requirement: Document tab label weight and padding

The document tab bar SHALL style its tab labels with a medium font weight
(`font-weight: 500`) and SHALL add a few pixels of right padding to each
`QTabBar#documentTabBar::tab`, so an active label does not touch the close
control or the next tab. The style rule SHALL be scoped to
`QTabBar#documentTabBar::tab` and SHALL NOT change the unscoped panel
`QTabBar::tab` rules or the panel tab bar. The change SHALL be a stylesheet
edit only: no tab-position, ordering, or close-control geometry changes, and the
self-test line that string-matches the theme SHALL NOT be edited.

#### Scenario: Active tab label is medium weight [ldt_tab_weight]

- **WHEN** a document tab is shown active
- **THEN** its label uses font weight 500 and the tab carries right padding

#### Scenario: Panel tabs are unaffected [ldt_tab_scope]

- **WHEN** a panel group tab is shown
- **THEN** its weight and padding are unchanged from before

### Requirement: Tab tooltip shows the full label

A document tab's tooltip SHALL be the tab's full label before elision: the
display name, the window ordinal suffix when the window has one, the zoom
suffix, and the mode/bit-depth suffix. A loading tab's label is its name alone.
This SHALL hold for untitled and saved documents alike, and the tooltip SHALL
NOT be the file path. The tooltip SHALL NOT carry the modified marker. Eliding
the visible tab text SHALL be unchanged.

#### Scenario: Untitled long name [dtw_tooltip_untitled]
- **WHEN** an untitled document's label is longer than the tab can show
- **THEN** the tab text is elided, the tooltip equals the full unelided label,
  and the tooltip differs from the tab text

#### Scenario: Saved long name [dtw_tooltip_saved]
- **WHEN** a saved document's label is longer than the tab can show
- **THEN** the tooltip equals the full label including its zoom and
  mode/bit-depth suffix, not the file path

### Requirement: Cycling document windows with Ctrl+Tab

`Ctrl+Tab` SHALL activate the next document window and `Shift+Ctrl+Tab` the
previous one, wrapping at either end. The cycle order SHALL cover every document
window: the tab-strip windows in their visual order, followed by floating
windows and tile panes in the order they entered that host. The shortcuts SHALL
work when the focus is on the canvas, the tab strip, a panel, or a text field of
a panel, and when a floating document window is active. They SHALL NOT act while
a dialog window is active. They SHALL NOT change the panel visibility that plain
`Tab` controls. With fewer than two document windows the shortcuts SHALL do
nothing. Cycling to another window SHALL affect a live edit session exactly as
selecting that window's tab does, so a live type-edit session behaves as it does
on a tab switch today. The Keyboard Shortcuts dialog SHALL treat both sequences
as reserved workspace keys, so that no command can be given them.

#### Scenario: Forward and back over tabs [dtw_cycle_tabs]
- **WHEN** three documents are tabbed, the first is active, the tab strip has
  focus, and the user presses `Ctrl+Tab` twice and then `Shift+Ctrl+Tab` once
- **THEN** the active document goes to the second, then the third, then back to
  the second

#### Scenario: Wrap-around [dtw_cycle_wrap]
- **WHEN** the last window in the cycle is active and the user presses `Ctrl+Tab`
- **THEN** the first window becomes active

#### Scenario: Cycling reaches a floating window [dtw_cycle_float]
- **WHEN** one document is tabbed and another floats, the tab is active, and the
  user presses `Ctrl+Tab`
- **THEN** the floating window becomes the active window

#### Scenario: Panels stay as they are [dtw_cycle_panels]
- **WHEN** the user presses `Ctrl+Tab` with panels shown
- **THEN** the panels stay shown

#### Scenario: Focus in a panel text field [dtw_cycle_text_focus]
- **WHEN** a panel's text field has focus and the user presses `Ctrl+Tab`
- **THEN** the next document window becomes active

#### Scenario: A modal dialog blocks cycling [dtw_cycle_modal]
- **WHEN** a modal dialog is open over two documents and the user presses
  `Ctrl+Tab`
- **THEN** the active document does not change

#### Scenario: One window does nothing [dtw_cycle_single]
- **WHEN** only one document window is open and the user presses `Ctrl+Tab`
- **THEN** the same window stays active and nothing else changes

#### Scenario: A live type edit behaves as on a tab switch [dtw_cycle_type_session]
- **WHEN** a type-edit session is live in one of two tabbed documents and the
  user presses `Ctrl+Tab`
- **THEN** the session and the layer are in the same state as after selecting
  the other document's tab

#### Scenario: The sequences are reserved [dtw_cycle_reserved]
- **WHEN** the Keyboard Shortcuts dialog lists the reserved workspace keys
- **THEN** `Ctrl+Tab` and `Shift+Ctrl+Tab` are among them

### Requirement: Document tab context menu

Right-clicking a document tab SHALL open a menu with, in order: `Consolidate All
to Here`, `Close`, `Close All`, `Close Others`, `Move to New Window`, a
separator, `New Document…`, `Open Document…`, a separator, and `Reveal in File
Manager`. The entries SHALL act on the clicked tab's window, not on the active
one:

- `Consolidate All to Here` moves every floating window and tile pane into the
  tab strip and activates the clicked tab.
- `Close` closes the clicked window under the tab close rules.
- `Close All` closes every document, with the usual prompts.
- `Close Others` closes every other document window under the tab close rules.
  It is disabled when the clicked window is the only one.
- `Move to New Window` floats the clicked window. It is disabled while the
  clicked window's document is loading.
- `New Document…` and `Open Document…` run `File > New…` and `File > Open…`.
- `Reveal in File Manager` opens the folder that contains the document's file in
  the desktop file manager. It is disabled for a document with no file path.

#### Scenario: Menu entries and order [dtw_tab_menu_entries]
- **WHEN** the user right-clicks a document tab
- **THEN** the menu shows the listed entries in the listed order

#### Scenario: Acts on the clicked tab [dtw_tab_menu_target]
- **WHEN** the second of two tabs is active and the user chooses `Move to New
  Window` from the first tab's menu
- **THEN** the first tab's document floats and the second stays tabbed

#### Scenario: Close Others with a single window [dtw_tab_menu_close_others]
- **WHEN** only one document window is open and the user right-clicks its tab
- **THEN** `Close Others` is disabled

#### Scenario: Close Others keeps the clicked window [dtw_tab_menu_close_others_run]
- **WHEN** three unmodified documents are tabbed and the user chooses `Close
  Others` on the second
- **THEN** only the second document stays open

#### Scenario: Reveal needs a file [dtw_tab_menu_reveal]
- **WHEN** the user right-clicks an untitled document's tab
- **THEN** `Reveal in File Manager` is disabled

#### Scenario: Reveal opens the containing folder [dtw_tab_menu_reveal_run]
- **WHEN** the user chooses `Reveal in File Manager` on a saved document's tab
- **THEN** the folder that contains the file is opened in the desktop file
  manager

#### Scenario: A loading tab cannot move [dtw_tab_menu_loading]
- **WHEN** the user right-clicks a loading tab
- **THEN** `Move to New Window` is disabled

#### Scenario: Consolidate All to Here [dtw_tab_menu_consolidate]
- **WHEN** two documents float, one is tabbed, and the user chooses `Consolidate
  All to Here` on the tab
- **THEN** all three documents are tabbed and the clicked tab is active
