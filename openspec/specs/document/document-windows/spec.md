# document/document-windows Specification

## Purpose
Document windows show an open document. A window can be a tab, a floating
window, or a tile pane. This capability covers moving windows between those
hosts, the `Window > Arrange` commands, several windows per document, how the
screen modes and the panel controls treat document windows, and the Interface
preferences that govern floating.

## Requirements

### Requirement: Documents are shown in document windows

Each open document SHALL be shown in at least one document window. Each window
SHALL have its own zoom and scroll position. Every window SHALL be hosted in
exactly one place: the document tab strip, a floating document window, or a
tile pane. Edits made through any window SHALL appear in every window of the
same document, including windows that are not active. Undo history, the
modified state, and the file path SHALL belong to the document, not to a window.
Moving a window to another host SHALL keep its zoom, its scroll position, and
the document's state. The status bar's magnification SHALL show the active
window's zoom. A document that is still loading SHALL NOT be floated, tiled,
torn out, or given another window.

#### Scenario: Moving a window keeps its view [dtw_host_move_keeps_view]
- **WHEN** a tabbed document at 200% zoom with a scrolled viewport is floated
  and then re-docked
- **THEN** the zoom and scroll position are the same after each move and the
  document's history is unchanged

#### Scenario: The status bar follows the active window's zoom [dtw_status_zoom]
- **WHEN** two windows of one document are at 50% and 200% and the user
  activates each in turn
- **THEN** the status bar's magnification reads 50% and then 200%

#### Scenario: An inactive visible window repaints [dtw_inactive_repaint]
- **WHEN** document `A` floats beside an active tabbed document `B` and `A`
  changes without being activated, for example when a background filter commit
  on `A` lands
- **THEN** `A`'s floating window shows the changed pixels

### Requirement: Tear a tab out and dock it back

Dragging a document tab out of the tab strip, past a short distance
perpendicular to the strip, SHALL float that window under the pointer and let
the drag continue as a window move. Dragging along the strip SHALL keep
reordering tabs. Dragging a floating document window by its header and releasing
or dropping it on the dock zone SHALL dock it as a tab at the drop position,
provided `Enable Floating Document Window Docking` is on. Tearing out SHALL also
work for the last remaining tab. A loading tab SHALL NOT tear out.

The dock zone SHALL be the tab strip's band. While the strip has no tabs, or its
bar is hidden, the dock zone SHALL be the band along the top of the document
area where the tab bar would be, as high as the tab bar's preferred height and
at least as high as a floating window's header. The dock zone SHALL be inactive
while the document windows are tiled and while
`Enable Floating Document Window Docking` is off.

While a drag that can dock a floating window is over the dock zone, the
application SHALL show a blue placement line in the drop indicator accent: a
vertical line at the insertion point between tabs while the strip has tabs, or
a horizontal line along the band while it has none. The line SHALL hide when
the drag leaves the zone, drops, is released, or is cancelled.

Where the compositor owns top-level placement (Wayland), a plain header drag of
a floating window SHALL start a drag-and-drop that carries the window and asks
the compositor to move the window with the pointer for the rest of the drag, so
that the window can leave the application frame. Dropping it on the dock zone
SHALL dock it at the drop position; a drop anywhere else, or outside the
application, SHALL leave the window floating where the compositor left it. A
tab torn out on Wayland SHALL float, become active, and continue as the same
window drag, the pointer holding the centre of the new window's header, so that
the window follows the pointer and a drop on the dock zone docks it again. A
compositor that can't move a window during a drag-and-drop leaves the window in
place while the drag runs; dropping on the dock zone still docks it.

On every platform, dragging a floating window's header with Ctrl held SHALL
start a drag-and-drop that carries the window. Dropping it on the dock zone
SHALL dock the window as a tab at the drop position, or at the end of the strip
when the drop is past the last tab, and activate it. The zone SHALL refuse the
drop while `Enable Floating Document Window Docking` is off. A drop anywhere
else SHALL leave the window floating where it was. File drops on the tab strip
SHALL keep opening the dropped files.

A drop that docks SHALL leave no floating window behind, and the docked tab
SHALL become active.

Double-clicking a floating window's header SHALL dock it as the last tab and
activate it, whether or not docking is enabled.

#### Scenario: Tear out [dtw_tear_out]
- **WHEN** two documents are tabbed and the user drags the second tab well below
  the tab strip and releases
- **THEN** the second document is in a floating document window, the first stays
  tabbed, and the floating window is active

#### Scenario: Tear out the last tab [dtw_tear_out_last]
- **WHEN** one document is tabbed and the user drags its tab well below the strip
- **THEN** the document floats and the tab strip has no tabs

#### Scenario: Reordering is unchanged [dtw_tear_reorder]
- **WHEN** the user drags a tab sideways within the strip
- **THEN** the tabs are reordered and nothing floats

#### Scenario: Dock back [dtw_dock_back]
- **WHEN** the header drag moves the window by hand and the user drags a
  floating window's header over the tab strip and releases
- **THEN** the document becomes a tab at the drop position and becomes active

#### Scenario: A header drag on Wayland leaves the frame [dtw_wayland_system_move]
- **WHEN** the platform is Wayland with a compositor that moves a window during
  a drag-and-drop, and the user drags a floating window's header past the edge
  of the application frame and drops it there, then tears a tab out and drops
  it back on the strip
- **THEN** each window follows the pointer for the whole drag, the
  header-dragged window stays floating where it was dropped, and the torn-out
  window becomes a tab again

#### Scenario: Tear out on Wayland [dtw_wayland_tear_out]
- **WHEN** the platform is Wayland and the user drags a tab well below the strip
- **THEN** the document floats in a visible top-level window that is active, the
  first document stays tabbed, and a drag-and-drop that carries the new window
  starts

#### Scenario: A Wayland header drag carries the window [dtw_wayland_header_drag_mime]
- **WHEN** the platform is Wayland and the user drags a floating window's header
  past the drag distance without Ctrl
- **THEN** a drag-and-drop starts whose data carries the document window, the
  floating window's native window, and the press position in that window, and
  the application does not move the window itself

#### Scenario: A torn-out window docks again by drop [dtw_wayland_tear_out_drag]
- **WHEN** the platform is Wayland, the user tears the second of two tabs out,
  and the drag that carries it drops on the first tab
- **THEN** the document becomes the tab at index 0, no floating window is left,
  and it is active

#### Scenario: The dock line between tabs [dtw_dock_indicator]
- **WHEN** two documents are tabbed, a third floats, and a drag carrying the
  float moves over the second tab
- **THEN** a 2 px vertical dock line shows at the second tab's left edge, as
  high as the strip

#### Scenario: The dock line without tabs [dtw_dock_indicator_no_tabs]
- **WHEN** every document floats and a drag carrying one of them moves over the
  top band of the document area
- **THEN** a 2 px horizontal dock line shows along that band

#### Scenario: Dropping into the empty strip docks [dtw_dock_drop_no_tabs]
- **WHEN** every document floats and a drag carrying one of them drops on the
  top band of the document area
- **THEN** the drop is accepted, the document becomes tab 0 and is active, no
  floating window is left for it, and the dock line hides

#### Scenario: The dock line hides when the drag leaves [dtw_dock_indicator_hidden_on_leave]
- **WHEN** a drag carrying a floating window shows the dock line and then leaves
  the strip
- **THEN** the dock line hides

#### Scenario: The hand drag shows the dock line [dtw_hand_drag_indicator]
- **WHEN** the header drag moves a floating window by hand and the pointer is
  over the tab strip, and the user then releases
- **THEN** the dock line shows while the pointer is over the strip, hides on the
  release, and the window docks

#### Scenario: No dock zone while tiled or with docking off [dtw_dock_zone_inactive_tiled_or_off]
- **WHEN** docking is off, or the windows are tiled, and a drag carrying a
  floating window moves over the strip
- **THEN** the drag is refused and no dock line shows

#### Scenario: Ctrl+drag docks at the drop position [dtw_ctrl_drag_docks]
- **WHEN** two documents are tabbed, a third floats, and the user Ctrl+drags the
  float's header and drops it on the second tab
- **THEN** the third document becomes the tab at index 1, the strip has three
  tabs, and the third document is active

#### Scenario: Ctrl+drag is refused while docking is off [dtw_ctrl_drag_refused]
- **WHEN** docking is disabled and the user Ctrl+drags a floating window's
  header onto the tab strip
- **THEN** the strip refuses the drop and the window stays floating

#### Scenario: A Ctrl+drag dropped elsewhere leaves the window floating [dtw_ctrl_drag_elsewhere]
- **WHEN** the user Ctrl+drags a floating window's header and drops it on a
  tabbed document's canvas
- **THEN** the drop is refused and the window stays floating where it was

#### Scenario: File drops on the strip still open files [dtw_ctrl_drag_keeps_file_drops]
- **WHEN** a document floats and the user drops an image file on the tab strip
- **THEN** the file opens as a new document and the floating window is unchanged

#### Scenario: Double-clicking the header docks [dtw_header_double_click_docks]
- **WHEN** docking is disabled, a document floats beside a tabbed one, and the
  user double-clicks the float's header
- **THEN** the document becomes the last tab and is active

### Requirement: Floating document window

A floating document window SHALL show a header carrying the document window's
full label, elided only to the header's width, followed by the modified marker
while the document is modified, and a close control. The close control SHALL
follow the tab close rules. The window SHALL stay above the application frame
and hide with it. A floating window's header SHALL stay inside the document
area when the window is placed by a command, so that no command produces an
unreachable window.

On every platform, a floating document window SHALL be a frameless top-level
window that is transient for the application frame. Where the compositor owns
top-level placement (Wayland), the compositor SHALL place the window and move
it when its header is dragged.

#### Scenario: Header label follows the document [dtw_float_header_label]
- **WHEN** a floating document is modified
- **THEN** its header shows the label followed by the modified marker

#### Scenario: Hosting follows the platform [dtw_float_hosting]
- **WHEN** a document is floated with in-frame hosting forced for testing, and
  again with the platform's own hosting
- **THEN** the window is a child overlay of the frame in the first case and a
  top-level tool window parented to the frame in the second, whatever the
  platform

#### Scenario: A float on Wayland is a top-level window [dtw_float_toplevel_wayland]
- **WHEN** the platform reports Wayland and a document is floated
- **THEN** the window is a top-level tool window transient for the frame, and a
  header drag that drops on the tab strip docks it

#### Scenario: Closing the last window of a floating document [dtw_float_close]
- **WHEN** an unmodified document's only window floats and the user uses its
  header close control
- **THEN** the floating window and the document close, and the empty workspace
  shows when no other document is open

### Requirement: Floating window header menu

Right-clicking a floating document window's header SHALL open a menu with, in
order: `Duplicate…`, `Image Size…`, `Canvas Size…`, a separator, `File Info…`, a
separator, and `Print…`. Choosing an entry SHALL first activate that window,
then run the matching menu-bar command on its document. Each entry SHALL be
enabled exactly when its menu-bar command is enabled while that window is
active.

#### Scenario: Header menu entries [dtw_header_menu_entries]
- **WHEN** the user right-clicks a floating window's header
- **THEN** the listed entries appear in the listed order

#### Scenario: Header menu targets its window [dtw_header_menu_target]
- **WHEN** a tabbed document is active and the user chooses `Image Size…` from a
  floating window's header menu
- **THEN** the floating window becomes active and Image Size opens for its
  document

### Requirement: Keys work in floating windows

Every menu shortcut, tool shortcut, and frame-level key (such as the brush-size
brackets, `Space` panning, `F`, `Esc`, and arrow-key nudging) that works while a
tabbed document is active SHALL work while a floating document window has focus,
and SHALL act on that window's document.

#### Scenario: Undo from a floating window [dtw_float_shortcut]
- **WHEN** a floating window has focus, its document has an undoable edit, and
  the user presses `Ctrl+Z`
- **THEN** that document's last edit is undone

#### Scenario: A menu shortcut from a floating window [dtw_float_menu_shortcut]
- **WHEN** a top-level floating window has focus and the user presses `Ctrl+A`
- **THEN** the floating window's document has a full-canvas selection and no
  other document's selection changes

#### Scenario: Tool shortcut from a floating window [dtw_float_tool_shortcut]
- **WHEN** a top-level floating window has focus and the user presses `B`
- **THEN** the Brush tool becomes active

#### Scenario: A frame key from a floating window [dtw_float_frame_key]
- **WHEN** a top-level floating window has focus, the Brush tool is active, and
  the user presses `]`
- **THEN** the brush size increases as it does from a tabbed document

### Requirement: Float and consolidate commands

`Window > Arrange > Float in Window` SHALL float the active document window.
`Float All in Windows` SHALL float every tabbed window and every tile pane.
`Consolidate All to Tabs` SHALL move every floating window and tile pane into the
tab strip, keeping the active window active. `Float in Window` SHALL be disabled
when the active window already floats or its document is loading. All three
SHALL be disabled with no documents open.

#### Scenario: Float in Window [dtw_arrange_float]
- **WHEN** two documents are tabbed and the user chooses `Float in Window`
- **THEN** the active document floats and the other stays tabbed

#### Scenario: Float All then Consolidate [dtw_arrange_float_all]
- **WHEN** three documents are tabbed and the user chooses `Float All in
  Windows`, then `Consolidate All to Tabs`
- **THEN** all three float, and then all three are tabbed again with the same
  document active

#### Scenario: Disabled without documents [dtw_arrange_disabled]
- **WHEN** no document is open
- **THEN** every `Window > Arrange` command is disabled

### Requirement: Cascade

`Window > Arrange > Cascade` SHALL float every document window, give them the
same size, and request positions stepped diagonally from the top-left of the
document area, in cycle order, with the active window on top. When the next step
would put a window's header outside the document area, the cascade SHALL
restart from the top-left. Where the compositor overrides the requested
positions of top-level windows, the windows SHALL still float and share a size.

#### Scenario: Cascade positions [dtw_cascade]
- **WHEN** three documents are tabbed, in-frame hosting is forced, and the user
  chooses `Cascade`
- **THEN** three floating windows of equal size exist and each window's
  top-left is offset from the previous one by the same positive step in x and y

#### Scenario: Many windows stay reachable [dtw_cascade_reachable]
- **WHEN** twelve documents are open in a small frame, in-frame hosting is
  forced, and the user chooses `Cascade`
- **THEN** every floating window's header lies inside the document area

### Requirement: Tile

`Window > Arrange > Tile` SHALL lay out every document window, floating ones
included, as a tile pane in a grid that fills the document area, replacing the
tab strip. The grid SHALL use `ceil(sqrt(n))` columns and as many rows as needed
for `n` windows, filled in cycle order. Each tile pane SHALL show a header with
the window's label and a close control. Opening or creating a document while
tiled SHALL add a pane and re-tile, whatever `Open Documents As Tabs` is set to,
because Tile is a layout the user chose.
Closing a pane SHALL re-tile the rest; when the last pane closes, the tile
layout SHALL end and the empty workspace SHALL show. `Consolidate All to Tabs`
SHALL end the tile layout. `Tile` SHALL be disabled with fewer than two
document windows.

#### Scenario: Tile four documents [dtw_tile_grid]
- **WHEN** four documents are open and the user chooses `Tile`
- **THEN** four panes appear in a 2×2 grid filling the document area and the tab
  strip is hidden

#### Scenario: Tile takes in floating windows [dtw_tile_floats]
- **WHEN** one document is tabbed, another floats, and the user chooses `Tile`
- **THEN** both are tile panes and no floating document window remains

#### Scenario: Open while tiled [dtw_tile_open]
- **WHEN** two documents are tiled, `Open Documents As Tabs` is on, and a third
  is created
- **THEN** three panes are shown and the new one is active

#### Scenario: Open while tiled with the tabs preference off [dtw_tile_open_pref_off]
- **WHEN** two documents are tiled, `Open Documents As Tabs` is off, and a third
  is created
- **THEN** three panes are shown, the new one is active, and no floating
  document window exists

#### Scenario: Closing panes [dtw_tile_close]
- **WHEN** two unmodified documents are tiled and the user closes one pane and
  then the other
- **THEN** the remaining pane fills the document area, and after the second
  close the empty workspace shows

#### Scenario: Consolidate ends tiling [dtw_tile_consolidate]
- **WHEN** documents are tiled and the user chooses `Consolidate All to Tabs`
- **THEN** every document is tabbed and the tab strip is shown

### Requirement: Match Zoom, Location, and All

`Window > Arrange > Match Zoom` SHALL set every other document window to the
active window's zoom. `Match Location` SHALL scroll every other window so its
viewport centre shows the same point as the active window's centre, expressed as
a fraction of the document's width and height. `Match All` SHALL do both. These
commands SHALL apply to windows in every host, tabbed ones included, and SHALL
be disabled with fewer than two document windows. CS6's exact behavior is not
verified; this rule is an approximation.

#### Scenario: Match Zoom [dtw_match_zoom]
- **WHEN** the active window is at 300%, another is at 50%, and the user chooses
  `Match Zoom`
- **THEN** both windows are at 300%

#### Scenario: Match Location [dtw_match_location]
- **WHEN** the active window's centre shows the document point at 25% width and
  75% height, and the user chooses `Match Location`
- **THEN** every other window's centre shows the point at 25% width and 75%
  height of its own document, within one device pixel

#### Scenario: Match All [dtw_match_all]
- **WHEN** two windows differ in zoom and centre and the user chooses `Match
  All`
- **THEN** both the zoom and the location of the other window match the active
  window

### Requirement: New Window For a document

`Window > Arrange > New Window For [file]` SHALL be labelled with the active
document's name and SHALL open another window of the active document and
activate it. The new window SHALL be hosted as a tab when `Open Documents As
Tabs` is on, as a floating window when it is off, and as a tile pane while the
tile layout is active. CS6's host for the new window is not verified; this rule
follows the preference.
Each window of a document after the first SHALL carry a `:N` suffix on its label,
where `N` is that window's position among the document's open windows, counting
from one; closing a window SHALL renumber the windows after it. The suffix
format is an approximation of CS6. The command SHALL be disabled with no
document open and while the active document is loading.

#### Scenario: Second window shares the document [dtw_new_window]
- **WHEN** the user chooses `New Window For` on `a.png` and then paints, runs a
  filter, and shows a mask overlay in the new window
- **THEN** both windows show each change, the new window's label ends in `:2`,
  and one undo removes the last change from both

#### Scenario: The new window follows the preference [dtw_new_window_host]
- **WHEN** the user chooses `New Window For` once with `Open Documents As Tabs`
  on and once with it off
- **THEN** the first new window is a tab and the second is a floating window

#### Scenario: Independent viewports [dtw_new_window_viewport]
- **WHEN** the user zooms the second window of a document
- **THEN** the first window's zoom is unchanged

#### Scenario: Menu label follows the active document [dtw_new_window_label]
- **WHEN** `b.png` is the active document
- **THEN** the menu entry reads `New Window For b.png`

#### Scenario: Ordinals renumber [dtw_new_window_renumber]
- **WHEN** a document has three windows and the user closes the second
- **THEN** the remaining windows' labels have no suffix and `:2`

### Requirement: Screen modes and panel controls leave document windows coherent

In Full Screen Mode With Menu Bar and in Full Screen Mode, only the active
document window SHALL be visible; other floating windows and tile panes SHALL be
hidden. When the active window floats, its canvas SHALL fill the screen-mode
canvas area without its header, as a tabbed document does. Returning to Standard
Screen Mode SHALL show every window again in the host and position it had. `Tab`, `Shift+Tab`, `Window > Workspace` switching, and
`Reset` of a workspace SHALL NOT hide, move, dock, or close any document window.

#### Scenario: Full screen shows only the active window [dtw_fullscreen_active]
- **WHEN** one document is tabbed, two float, one of the floats is active, and
  the frame enters Full Screen Mode With Menu Bar
- **THEN** only the active document window is visible

#### Scenario: An active float fills the screen [dtw_fullscreen_float_fills]
- **WHEN** a floating window is active and the frame enters Full Screen Mode
- **THEN** its canvas fills the canvas area, its header is not shown, and no
  other document window is visible

#### Scenario: Standard mode restores the layout [dtw_fullscreen_restore]
- **WHEN** the frame returns from a full-screen mode to Standard Screen Mode
- **THEN** every document window is visible again in the host and position it
  had before

#### Scenario: Hiding panels keeps floating documents [dtw_tab_keeps_floats]
- **WHEN** a document floats and the user presses `Tab`
- **THEN** the panels hide and the floating document window stays visible

#### Scenario: Switching workspace keeps floating documents [dtw_workspace_keeps_floats]
- **WHEN** a document floats and the user switches to another workspace
- **THEN** the floating document window stays floating at the same position

### Requirement: Interface preferences for document windows

Preferences > Interface SHALL offer `Open Documents As Tabs` and `Enable
Floating Document Window Docking`, both on by default and kept across restarts.
A session store written before these options existed SHALL load both as on.
With `Open Documents As Tabs` off and no tile layout active, a newly created
document, and an opened document once its load has finished, SHALL be shown in
a floating document window. With `Enable Floating Document Window Docking` off, dropping a floating
window on the tab strip SHALL NOT dock it, while the menu commands that
consolidate SHALL still work.

#### Scenario: Open as floating [dtw_pref_open_floating]
- **WHEN** `Open Documents As Tabs` is off and the user creates a document
- **THEN** the document opens in a floating window

#### Scenario: Opened files float after loading [dtw_pref_open_floating_loaded]
- **WHEN** `Open Documents As Tabs` is off and the user opens a file
- **THEN** a loading tab shows while it loads, and the document is in a
  floating window once the open succeeds

#### Scenario: Docking disabled [dtw_pref_no_docking]
- **WHEN** docking is disabled and the user drops a floating window on the tab
  strip
- **THEN** the window stays floating, and `Consolidate All to Tabs` still docks
  it

#### Scenario: Preferences persist [dtw_pref_persist]
- **WHEN** both options are turned off, the session is saved, and the session
  store is loaded again
- **THEN** both options are still off

#### Scenario: An older store defaults both on [dtw_pref_defaults]
- **WHEN** a session store without either key is loaded
- **THEN** both options are on and the store's other keys are kept

### Requirement: Window layout is not persisted

Floating windows, tile panes, extra windows of a document, and their geometry
SHALL NOT be saved across sessions. Documents reopened by crash recovery SHALL
open as tabs, one window each, whatever `Open Documents As Tabs` is set to.
Closing the application or using `Close All` SHALL visit the documents in every
host and SHALL prompt at most once per modified document.

#### Scenario: Recovery reopens as tabs [dtw_recovery_tabs]
- **WHEN** a document that was floating is restored by crash recovery with
  `Open Documents As Tabs` off
- **THEN** it opens as a tab

#### Scenario: One prompt per document [dtw_close_all_prompts]
- **WHEN** a modified document has two windows, one tabbed and one floating, and
  the user chooses `Close All`
- **THEN** exactly one unsaved-changes prompt appears for that document

### Requirement: Multi-window Hand and Zoom options

The Hand tool's options bar SHALL offer `Scroll All Windows`, and the Zoom
tool's options bar SHALL offer `Resize Windows To Fit` and `Zoom All Windows`.
All three SHALL be off by default and kept across restarts with the other tool
options. Preferences > General SHALL offer `Zoom Resizes Windows`, off by
default and kept across restarts.

- With `Scroll All Windows` on, a Hand drag (including the `Space` form) in one
  window SHALL pan every other visible document window by the same screen
  distance. Holding `Shift` during the drag SHALL have the same effect with the
  option off.
- With `Zoom All Windows` on, a Zoom tool click or marquee in one window SHALL
  change every other document window's zoom by the same factor. A
  `Shift`+click with the option off SHALL have the same effect. View commands and
  keyboard zoom SHALL NOT be affected.
- With `Resize Windows To Fit` on, a Zoom tool zoom of a floating window SHALL
  resize that window to fit the zoomed image, clamped to the document area.
  `Zoom Resizes Windows` SHALL extend the same rule to `View > Zoom In` and
  `Zoom Out` and their shortcuts. Tabbed windows and tile panes SHALL keep
  their size.

These options SHALL NOT add history states. The docs' `Match All` with a
`Shift`+click or drag is approximated by the `Shift` forms above: the gesture
is applied to every window.

#### Scenario: Scroll All Windows [dtw_scroll_all]
- **WHEN** two documents float side by side, `Scroll All Windows` is on, and the
  user Hand-drags 40 pixels to the right in one of them
- **THEN** both windows' views move 40 pixels to the right

#### Scenario: Shift scrolls all windows [dtw_scroll_all_shift]
- **WHEN** `Scroll All Windows` is off and the user Hand-drags with `Shift` held
  in one of two floating windows
- **THEN** both windows' views move by the drag distance

#### Scenario: Scrolling one window only [dtw_scroll_all_off]
- **WHEN** `Scroll All Windows` is off and the user Hand-drags without `Shift`
- **THEN** only the dragged window's view moves

#### Scenario: Zoom All Windows [dtw_zoom_all]
- **WHEN** `Zoom All Windows` is on, one window is at 100% and another at 50%,
  and the user zooms in with a Zoom tool click in the first
- **THEN** the second window's zoom changes by the same factor as the first's

#### Scenario: Shift-click zooms all windows [dtw_zoom_all_shift]
- **WHEN** `Zoom All Windows` is off and the user `Shift`+clicks with the Zoom
  tool in one of two windows
- **THEN** both windows' zoom changes by the same factor

#### Scenario: Resize Windows To Fit [dtw_resize_to_fit]
- **WHEN** `Resize Windows To Fit` is on and the user zooms a floating window in
  with the Zoom tool
- **THEN** the floating window grows to fit the zoomed image and stays inside the
  document area, and a tabbed window zoomed the same way keeps its size

#### Scenario: Zoom Resizes Windows extends to keyboard zoom [dtw_zoom_resizes_pref]
- **WHEN** `Zoom Resizes Windows` is on and the user presses `Ctrl+=` with a
  floating window active, and again with it off
- **THEN** the window resizes in the first case and keeps its size in the second

#### Scenario: Options persist [dtw_hand_zoom_options_persist]
- **WHEN** the three options and `Zoom Resizes Windows` are turned on and the
  session store is loaded again
- **THEN** all four are still on
