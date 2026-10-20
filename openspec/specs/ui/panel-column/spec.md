# panel-column Specification

## Purpose
The panel column host: groups and top tabs, the default groups, the registry, the width toggle, and compact or iconic modes.

## Requirements

### Requirement: Panel column host

The system SHALL host the right-hand panels in a custom `PanelColumn` widget
instead of the `QDockWidget` area, laid out as a vertical stack of `PanelGroup`s
whose heights the user can drag with splitter handles. In `normal` mode the
column SHALL enforce a sensible minimum width so it cannot be squeezed to
nothing. The column SHALL impose no hard panel minimum that forces the main
window taller, and when the available height is less than the groups' size hints
the column SHALL scroll rather than grow the window. Panel resize and scroll
state SHALL survive a relayout. The workspace SHALL support an ordered set of
`PanelColumn`s: zero or more on the left of the document tab area, the document
tabs, and zero or more on the right. A `PanelColumn` SHALL be creatable
dynamically as the result of a drop and SHALL be removed whenever it holds no
groups, on every path that can empty it — a committed drop, a group close, a
panel hide, a flyout restore that takes the last panel, or a group removal — not
only on a committed drop; the document tabs SHALL keep the splitter stretch. A
column's side SHALL be determined by its order in the layout relative to the
document tabs, not by its on-screen geometry. A whole widget column SHALL be
dockable to any side of the Tools toolbar, of another widget panel or column,
and of the workspace, not only to the far right, and SHALL use the existing drop
grammar and insertion indicator rather than a second drag system.

#### Scenario: The column hosts panel groups [m41_tabs]

- **WHEN** the frame starts with a fresh session
- **THEN** the right-hand area contains a `PanelColumn` of `PanelGroup`s and no
  right-hand panel `QDockWidget`

#### Scenario: A short window scrolls instead of forcing a minimum [m41_minwidth]

- **WHEN** the main window is shrunk below the column's size hint
- **THEN** the window shrinks and the column scrolls, and no panel forces a
  larger height minimum

#### Scenario: Group heights are draggable [m41_tabs]

- **WHEN** the user drags the handle between two groups
- **THEN** the groups' heights change accordingly

#### Scenario: Normal mode enforces a minimum width [m42_minwidth]

- **WHEN** the splitter handle is dragged to squeeze the column in `normal` mode
- **THEN** the column does not shrink below its normal-mode minimum width

#### Scenario: A new column is created on a side drop [m43_newcolumn]

- **WHEN** a floating group is dropped beside a column, beside a group at the
  workspace edge, or beside the Tools toolbar
- **THEN** a new `PanelColumn` is created on that side of the workspace and the
  group is placed in it

#### Scenario: An emptied column is removed [m43_newcolumn]

- **WHEN** the last group leaves a dynamically created column
- **THEN** the column is removed from the workspace and the document tabs keep
  the stretch

#### Scenario: A column emptied by any path is removed [m45_empty_column_removed]

- **WHEN** the last widget of a dynamically created column leaves it through a
  committed drop, a group close, or a panel hide
- **THEN** the column is removed from the workspace on every one of those paths,
  not only on a committed drop, and the document tabs keep the stretch

#### Scenario: A whole widget column docks to any side [m44_docksides]

- **WHEN** a whole widget column is dragged to a side of the Tools toolbar, of
  another widget panel or column, or of the workspace
- **THEN** a target on that side is resolved through the existing drop grammar
  and the column is placed there

### Requirement: Panel groups and top tabs

Each `PanelGroup` SHALL present its panels in a `QTabWidget` with the tab
position explicitly set to `QTabWidget::North`, so the tabs render on top of the
group on every platform and stylesheet. The tab text SHALL be the panel's title,
there SHALL be no separate group-title label widget, and a group containing a
single panel SHALL still show that panel's tab. In `normal` mode a tab SHALL show
its text only and no icon; the panel icon SHALL be retained for the iconic/icon
strip and shown there, not baked into the normal tab.

#### Scenario: Tabs are on top [m41_tabs]

- **WHEN** any group is shown
- **THEN** its tab position is `QTabWidget::North`

#### Scenario: The tab is the only group chrome [m41_tabs]

- **WHEN** a group's tab bar is inspected
- **THEN** the group has no title label beyond the tabs, and each tab's text is
  the panel title

#### Scenario: A single-panel group shows its tab [m41_tabs]

- **WHEN** a group contains exactly one panel
- **THEN** that panel's tab is shown

#### Scenario: Normal tabs carry no icon [pc_tabs_iconless]

- **WHEN** a panel group is shown in `normal` mode
- **THEN** each tab shows its text only with no icon, while the iconic/icon strip
  still shows the panel icon

### Requirement: Default panel groups

The system SHALL open a fresh session in the authentic CS6 Essentials
workspace: a wider main right-hand column holding the groups `Color | Swatches`,
`Adjustments | Styles`, and `Layers | Channels | Paths`, and a narrower
secondary right-hand column collapsed to icons holding `History` and
`Properties`. `Navigator | Histogram | Info` SHALL NOT be in a default visible
group. Every other registered panel — `Actions`, `Gradients`, `Patterns`,
`Notes`, `Brush`, `Clone Source`, and the type panels — SHALL remain registered
and reachable from `Window > Panels` but SHALL NOT be in a default visible
group. (This application does not provide a `Libraries` panel.)

#### Scenario: Default groups match CS6 Essentials [m41_groups]

- **WHEN** the frame starts with a fresh session
- **THEN** the main right-hand column contains the groups `Color | Swatches`,
  `Adjustments | Styles`, and `Layers | Channels | Paths`, and a second
  right-hand column to its left is iconic and holds `History` and `Properties`

#### Scenario: A folded panel stays reachable [m41_groups]

- **WHEN** the `Window > Panels > Navigator` toggle is invoked
- **THEN** the Navigator panel can be shown even though it is not in a default
  visible group

### Requirement: Panel registry and content widgets

Each panel SHALL be a plain `QWidget` content widget with a stable `objectName`
and SHALL be registered in a frame-owned registry mapping the panel to its
title, icon asset id, group id, visibility, iconic state, minimized state, and
order. The `objectName`, `setView`, `refresh`, and existing test hooks of every
panel SHALL be preserved where possible. The `Window > Panels` toggles SHALL
remain the single command path for panel visibility and SHALL reflect and drive
the registry.

#### Scenario: Panels are content widgets [m41_tabs]

- **WHEN** a panel object is queried
- **THEN** it is a `QWidget` content widget with its documented `objectName` and
  is not a `QDockWidget`

#### Scenario: The Window menu drives the registry [m41_groups]

- **WHEN** a `Window > Panels` toggle is invoked
- **THEN** the panel's registry visibility changes and the column reflects it

### Requirement: Column width toggle

The top of the column SHALL carry one thin width-toggle control with
`objectName` `panelColumnToggle` that switches the whole column between `normal`
and `iconic` modes, using the same visual language as the Tools panel's
`toolsColumnToggle`. Switching to `iconic` SHALL set the column to the smallest
width that shows the icon strip rather than keeping the prior splitter width,
and switching back to `normal` SHALL restore the normal-mode width. The control
SHALL replace the M24 `PanelRail` far-right toolbar. The normal-mode width SHALL
be recorded when the column leaves normal mode and SHALL be seeded from the
session's stored `railWidth` when the column is constructed, so a column that
starts in or is left in iconic mode still restores its real normal-mode width.

#### Scenario: The toggle switches modes [m41_iconic]

- **WHEN** the column width toggle is activated
- **THEN** the column switches from `normal` to `iconic` and back, and the
  control's state reflects the current mode

#### Scenario: Switching to iconic uses the smallest width [m42_iconic]

- **WHEN** the column is wide in `normal` mode and the toggle switches it to
  `iconic`
- **THEN** the column collapses to the smallest width that shows the icon strip
  and does not keep the prior splitter width

#### Scenario: The toggle replaces the rail [m41_rail]

- **WHEN** the frame is built
- **THEN** there is no `PanelRail` and no far-right panel rail toolbar

#### Scenario: The normal width survives an iconic round-trip and restart [lpr_iconic_width]

- **WHEN** a column is set to a normal width, switched to iconic, and the
  application is restarted while it is iconic
- **THEN** switching it back to normal restores the stored normal-mode width
  measured after the first layout

### Requirement: Compact and iconic mode

In `iconic` mode the column SHALL collapse to a narrow vertical strip
containing one icon button per panel with group dividers between groups, and
SHALL show the panel labels when the strip is widened. Each strip icon button
SHALL be larger than the M41 strip and SHALL render its icon at a larger pixmap
size. Clicking a panel icon SHALL open a frameless flyout overlay that hosts the
**same `PanelGroup`** as the docked group: the group's full tab set, with the
clicked panel set current, styled and behaving exactly as when docked or
floating, with no parity differences between the three presentations, and the
group SHALL be restored to its column position when the flyout closes. The
flyout SHALL be a persistent overlay rather than a transient popup: it SHALL
remain open when the user clicks elsewhere in the workspace, and pressing Escape
SHALL NOT close it. The flyout SHALL open on the inner side of the column: to the
left of the strip when the column is on the right, and to the right of the strip
when the column is on the left. The icon whose flyout is open SHALL render in the
active/pressed state. The flyout SHALL NOT steal focus permanently. A column
SHALL have at most one flyout open: clicking the same panel icon again SHALL
close it, and clicking a different panel icon in the same column SHALL replace it
with the newly clicked panel's group. When the hosted group leaves the flyout
because it is dragged out, its last visible panel or group is closed, or its
owning column is removed, the flyout SHALL be hidden and its state cleared
rather than left as an empty overlay. Hiding all panels SHALL also hide the
flyout. The strip width SHALL be fixed while the column is iconic, so a
neighbouring pane's splitter handle drag SHALL NOT resize it. The strip icon
button and pixmap sizes SHALL be larger than the M42 sizes.
The flyout placement SHALL be derived from the actual button geometry and SHALL
clamp only the inner-side coordinate, so the flyout can never cross to the outer
side of the column. The compact strip SHALL show each group's icon buttons as one
visual unit, and SHALL expose a small drag-handle affordance above each group
that drags the whole group. The divider between compact groups SHALL be dark
grey, not white; the group container SHALL use the panel surface shade and the
drag dots SHALL be dark gray. Strip labels SHALL elide and appear as soon as any
room exists, rather than only once the full label width is available. A drop onto
or very close above/below a compact group SHALL insert into that group at that
place; a drop on a group's drag-handle grip SHALL create a new group directly
above that group; and a drop between groups, above the top group, or below the
bottom group SHALL create a new group at that boundary. Pressing and dragging a
compact icon SHALL tear that panel into a floating overlay that follows the
cursor until the mouse button is released and is committed on release, exactly
like a tab or group drag.

#### Scenario: Iconic mode collapses to a strip [m41_iconic]

- **WHEN** the column enters iconic mode
- **THEN** it shows a narrow icon strip with group dividers and does not show
  the full panel content

#### Scenario: The strip icons are larger [m42_iconic]

- **WHEN** the iconic strip is built
- **THEN** its icon buttons and icon pixmaps are larger than the M41 strip

#### Scenario: The popup hosts the whole group with the clicked panel active [m45_popup_group]

- **WHEN** a panel icon is clicked in the iconic strip
- **THEN** the flyout hosts the whole `PanelGroup` with all of the group's tabs,
  the clicked panel is current, and the group is restored to its column position
  when the flyout closes

#### Scenario: The flyout opens on the inner side [m42_flyout]

- **WHEN** a panel icon is clicked while the column is on the right
- **THEN** the flyout opens to the left of the strip, and when the column is on
  the left it opens to the right of the strip

#### Scenario: The compact icon grows again [m43_icon]

- **WHEN** the iconic strip is built
- **THEN** its icon buttons and icon pixmaps are larger than the M42 sizes

#### Scenario: The flyout uses the actual button geometry [m43_flyout]

- **WHEN** a panel icon's flyout is placed
- **THEN** the flyout's inner edge meets the clicked button's actual edge and the
  flyout is never placed on the outer side of the column

#### Scenario: The open panel's icon is active [m42_iconic]

- **WHEN** a panel icon's flyout is open
- **THEN** that icon renders in the active/pressed state

#### Scenario: The compact flyout matches the docked widget [m44_popupstyle]

- **WHEN** a panel is shown in a compact flyout and the same panel is docked
- **THEN** both present the same tab bar, background, and border styling

#### Scenario: The compact divider is dark grey [m44_compactdivider]

- **WHEN** the compact strip is built with more than one group
- **THEN** the divider between groups is dark grey and not white

#### Scenario: Strip labels elide as soon as there is room [m44_elide]

- **WHEN** the compact strip is made slightly wider than the icon button alone
- **THEN** the labels appear and elide to the available width rather than
  staying hidden until the full label width is available

#### Scenario: A compact drop onto a group inserts into it [m44_compactdrop]

- **WHEN** a dragged icon is dropped onto a compact group, or just above or
  below it
- **THEN** the panel is inserted into that group at that place

#### Scenario: A compact drop between or beyond groups creates a group [m44_compactdrop]

- **WHEN** a dragged icon is dropped between two compact groups, above the top
  group, or below the bottom group
- **THEN** a new group is created at that boundary containing the panel

#### Scenario: A group drag handle drags the whole group [m44_draghandle]

- **WHEN** the compact drag-handle above a group's icons is dragged
- **THEN** the whole group is dragged, not a single icon

#### Scenario: A compact icon drag follows the cursor [m47_compact_icon_float]

- **WHEN** a compact strip icon is pressed and dragged away from the strip
- **THEN** a floating overlay appears and follows the cursor until release, and
  the release commits the drop

#### Scenario: A compact panel drop on the grip creates a group above [m47_compact_grip_group]

- **WHEN** a single compact panel is dropped on a group's drag-handle grip
- **THEN** a new one-panel group is created directly above that group

#### Scenario: An iconic column is not resized by its neighbour [m47_iconic_fixed_width]

- **WHEN** a normal column and an iconic column are adjacent and the splitter
  handle between them is dragged
- **THEN** the iconic column keeps its strip width and only the normal column
  resizes

#### Scenario: The flyout stays open on outside clicks [pc_flyout_persists]

- **WHEN** a flyout is open and the user clicks elsewhere in the workspace
- **THEN** the flyout remains visible and the group remains hosted in it

#### Scenario: The flyout toggles and switches from its icon [pc_flyout_toggle]

- **WHEN** the icon whose flyout is open is clicked again, or another panel icon
  in the same column is clicked
- **THEN** the former closes the flyout, the latter replaces it with the other
  panel's group, and the column never shows two flyouts

#### Scenario: Escape does not close the flyout [pc_flyout_escape]

- **WHEN** a flyout is open and the user presses Escape
- **THEN** the flyout remains visible

#### Scenario: The flyout closes when its group leaves [pc_flyout_group_leaves]

- **WHEN** the hosted group is dragged out of the flyout, its last visible panel
  or group is closed, or its owning column is removed
- **THEN** the flyout is hidden and no empty overlay remains

#### Scenario: Hiding all panels hides the flyout [pc_flyout_hide_all]

- **WHEN** all panels are hidden with `Tab`
- **THEN** the flyout is hidden with them and restored only by reopening it

### Requirement: Panel tab context menu

Right-clicking a group's tab bar SHALL show exactly, in order, `Close`,
`Close Panel Group`, `Minimize`, `Collapse to Icons`, `Auto-Collapse Iconic
Panels` (checkable), `Auto-Show Hidden Panels` (checkable), and
`Interface Options…`. The two checkable items SHALL persist in the session.
`Interface Options…` SHALL open the Preferences dialog on its Interface pane.
An action that cannot apply SHALL be disabled rather than shown as an inert
stub.

#### Scenario: The tab menu has exactly the seven items [m41_menu]

- **WHEN** a group's tab bar is right-clicked
- **THEN** the menu contains exactly the seven named items in order

#### Scenario: The checkable items persist [m41_session]

- **WHEN** `Auto-Collapse Iconic Panels` is toggled and the session is saved and
  reloaded
- **THEN** the setting is restored

#### Scenario: Interface Options opens the Interface pane [m41_prefs]

- **WHEN** `Interface Options…` is chosen
- **THEN** the Preferences dialog opens on the Interface pane

### Requirement: Minimize and collapse to icons

`Minimize` SHALL roll a group up so only its tab bar remains visible with the
content hidden, and a minimized group SHALL occupy only the height its tab bar
needs rather than the expanded height it had. `Collapse to Icons` SHALL reduce
the group to its iconic representation. The two actions SHALL be distinct and
SHALL persist their state per group in the session. The group's tab menu entry
SHALL read `Expand Panel` while the group is minimized and `Minimize` otherwise,
so the label names the action it performs.

#### Scenario: Minimize hides the content [m41_minimize]

- **WHEN** `Minimize` is chosen for a group
- **THEN** the group shows only its tab bar and its content is hidden

#### Scenario: Minimize collapses the group to its tab bar [m45_minimize_collapse]

- **WHEN** a group is minimized and the layout is forced
- **THEN** the group's own height is the tab-bar height and it does not keep the
  expanded height

#### Scenario: The menu reads Expand Panel while minimized [m45_minimize_collapse]

- **WHEN** a group is minimized and its tab menu is shown
- **THEN** the minimize entry reads `Expand Panel`, and a non-minimized group's
  entry reads `Minimize`

#### Scenario: Collapse to icons is distinct from minimize [m41_iconic]

- **WHEN** `Collapse to Icons` is chosen for a group
- **THEN** the group is represented iconically rather than rolled up to its tab
  bar

#### Scenario: The minimized state persists [m41_session]

- **WHEN** a group is minimized, the session is saved, and the store is reloaded
- **THEN** the group is still minimized

### Requirement: Panel drag and drop rules

The system SHALL support dragging a panel tab and dragging a group, and SHALL
distinguish them for the whole drag. Dragging a tab SHALL drag only that panel;
dragging the empty header area to the right of the tabs SHALL drag the whole
group. Dragging a tab within its own group SHALL reorder it; dragging a tab onto
another group's tab bar SHALL move the panel into that group at the target index;
dragging onto the column background or between groups SHALL insert a new group at
that boundary; and dragging past the workspace edge SHALL tear the dragged panel
or group off into a frameless overlay whose hosting SHALL be platform-adaptive.
Where the platform lets a client position its own top-level windows, the overlay
SHALL be a frameless `Qt::Tool` top-level window parented to (transient for) the
main window — no title bar, no window decorations, and no taskbar entry — rather
than an in-window child widget, movable outside the main window with its movement
clamped to the available screen geometry. Where the platform does not let a
client position its own top-level windows, the overlay SHALL instead be an
in-window child of the frame that follows the cursor by parent-relative
coordinates and is clamped to the frame rect, so it cannot be lost. A
single-panel drag SHALL float a one-panel float, and a group drag SHALL float the
group. The same tab-versus-header distinction SHALL hold for a floating group. A
floating overlay SHALL be re-dockable into a column, into another group, or as a
new column by dropping it on the target. When a tab is dragged outward to float,
the drag SHALL continue until the mouse button is released: the float SHALL keep
following the cursor, exactly like a group or column drag, and SHALL only be
committed or re-docked on release. A whole widget column SHALL be draggable to
any side of the Tools toolbar, of another widget panel or column, and of the
workspace. Every drop that moves the last panel or group out of a widget column
SHALL leave no empty column behind, and this SHALL apply to the primary column as
well as to drop-created columns: when the last visible panel is removed or moved
out, the column SHALL be torn down, and a later drop or `Window`-menu show SHALL
recreate a host column rather than leaving a bare strip.

#### Scenario: A tab reorders within its group [m41_drag]

- **WHEN** a tab is dragged to a different index in the same group
- **THEN** the panel order in that group changes

#### Scenario: A tab regroups [m41_drag]

- **WHEN** a tab is dropped on another group's tab bar
- **THEN** the panel joins that group at the drop index and leaves its old group

#### Scenario: A drop between groups inserts a new group [m41_drag]

- **WHEN** a dragged item is dropped on the column background between two groups
- **THEN** a new group is inserted at that boundary containing the dragged item

#### Scenario: A group tears off into a frameless overlay and re-docks [m41_tearoff]

- **WHEN** a group is dragged past the column edge and then dropped back on the
  column
- **THEN** it floats in a frameless overlay and, on the drop, re-docks into the
  column

#### Scenario: A single-tab drag floats only that panel [m43_tabdrag]

- **WHEN** a tab title is dragged out of the column and torn off
- **THEN** a one-panel float is created for that panel only, and the source group
  keeps its other panels

#### Scenario: A header drag floats the whole group [m43_tabdrag]

- **WHEN** the empty header area to the right of the tabs is dragged out and torn
  off
- **THEN** the whole group floats, containing all of its panels

#### Scenario: A floating group separates a tab drag from a header drag [m43_tabdrag]

- **WHEN** a floating group's tab is dragged
- **THEN** only that panel moves, and when its empty header is dragged the whole
  group moves

#### Scenario: A floated tab keeps following until release [m44_floatdrag]

- **WHEN** a tab is dragged outward until a float is created and the mouse keeps
  moving before the button is released
- **THEN** the float keeps following the cursor, and it is committed or re-docked
  only on release

#### Scenario: The primary column is removed when its last panel closes [m46_primary_empty]

- **WHEN** the primary widget column's last visible panel is closed or moved out
- **THEN** the column is removed, and a later show or drop recreates a host
  column instead of leaving a bare empty strip

### Requirement: Drop insertion indicator

During any panel drag the target column or group SHALL draw a thick blue
insertion line at the candidate drop position: a boundary between groups for a
column/group drop, the target tab index for a tab/regroup drop, or a full-height
line at the workspace edge for a new-column drop. The indicator SHALL be drawn
by the drop target under the pointer and SHALL disappear when the drag leaves or
is cancelled. The M41/M42 indicator look SHALL be kept. The indicator SHALL be
positioned from the same resolved `DropTarget` the commit uses, in the column
that owns that target, so the line and the drop can never disagree: a drop on
the right of a group's tab bar SHALL draw the line at that tab index on the
right, including when the group has hidden tabs so the index maps past them; a
drop into another column's group, group body, or group boundary SHALL draw the
line in that target column; a drop as the rightmost tab SHALL draw the line at
the rightmost tab position and not at the far left; a drop at the bottom boundary
SHALL draw the line at the bottom of the last visible group, clamped inside the
scroll viewport so the line is never entirely clipped; and a whole-group drag in
compact mode SHALL draw the line above the group's drag-handle dots, not inside
the group below them. The column that owns a new-column line SHALL be the column
adjacent to the workspace edge the new column will occupy, not the drag source.
The same single indicator SHALL mark any-side docking targets beside the Tools
toolbar, beside another widget panel or column, and beside the workspace. The
new-column line SHALL be drawn above any floating overlay that follows the drag,
so an in-window overlay that tracks the cursor cannot hide it. The atomic Tools
column SHALL draw no line for a drag over its own body that resolves no
beside-column target; a drag that resolves a new column immediately beside the
Tools column SHALL draw the new-column mark at that edge like any other column.

#### Scenario: The blue line marks the target boundary [m41_drop]

- **WHEN** a drag hovers over the column between two groups
- **THEN** a thick blue insertion line is drawn at that boundary

#### Scenario: The indicator clears on cancel [m41_drop]

- **WHEN** a drag is cancelled or leaves the column
- **THEN** no insertion line is drawn

#### Scenario: The indicator marks a new-column target [m43_newcolumn]

- **WHEN** a drag hovers beyond the workspace edge on a side
- **THEN** the blue indicator marks the new-column position on that side

#### Scenario: The indicator marks an any-side dock target [m44_docksides]

- **WHEN** a whole widget column is dragged near a side of the Tools toolbar,
  another widget panel or column, or the workspace
- **THEN** the single blue indicator marks the target position on that side

#### Scenario: The indicator side matches the drop [m45_indicator_side]

- **WHEN** a tab is dragged onto the right side of a group's tab bar
- **THEN** the blue line is drawn on the right at the resolved insertion index,
  and the committed drop lands at that same index

#### Scenario: A group with hidden tabs still draws the right line [m46_indicator_hidden_tab]

- **WHEN** a tab is dragged onto the right side of a group that contains a hidden
  tab
- **THEN** the blue line is drawn at the right insertion position and the drop
  lands at that same index

#### Scenario: A cross-column drop shows the indicator in the target column [m45_indicator_cross_column]

- **WHEN** a tab is dragged from one widget column onto another widget column's
  tab bar, group body, or group boundary
- **THEN** the blue line is drawn in the target column at the resolved insertion
  point, and the committed drop places the tab there

#### Scenario: The rightmost-tab drop draws the line at its own index [m45_indicator_rightmost_tab]

- **WHEN** a tab is dropped as the rightmost tab of a group
- **THEN** the blue line is drawn at that tab's insertion position and not at the
  far left of the tab bar

#### Scenario: The bottom-boundary drop draws the line at the bottom [m45_indicator_bottom]

- **WHEN** a group or panel is dragged to the bottom boundary of the column
- **THEN** the blue line is drawn inside the scroll viewport at the bottom of the
  last visible group, not clipped below it

#### Scenario: The compact group drag draws the line above the dots [m45_compact_group_line]

- **WHEN** a whole group is dragged in compact mode and the target is a boundary
  before a group
- **THEN** the blue line is drawn above that group's drag-handle dots rather
  than inside the group below them

#### Scenario: The new-column mark stays above a following overlay [fp_edge_mark_above_overlay]

- **WHEN** a floating overlay is dragged over a dock spot while still floating
- **THEN** the new-column mark is drawn above the overlay and remains visible,
  instead of being hidden under the overlay that follows the cursor

#### Scenario: The atomic Tools column draws no widget drop line [fp_edge_mark_not_on_tools]

- **WHEN** a widget panel is dragged over the docked Tools column's body and
  resolves no beside-column target
- **THEN** no new-column mark is drawn on the Tools column

#### Scenario: A bare workspace-edge mark is drawn on the requested side [fp_edge_mark_workspace_side]

- **WHEN** a column is dragged to the leftmost workspace side while the only
  other column is on the right
- **THEN** the mark is drawn at the central area's left edge, not at the
  right-hand column's edge, and it is committed at the splitter head

#### Scenario: A drag beside the docked Tools column shows the mark [wpx_tools_side_mark]

- **WHEN** a widget panel or group is dragged into the new-column band
  immediately beside the docked Tools column
- **THEN** the thick blue new-column mark is drawn at that side of the Tools
  column, on the side the new column would occupy

### Requirement: Panel column session state

The session store SHALL advance to schema version 10 and SHALL persist the
per-column layout, where each column records its side, its order, its width, its
rail mode (`normal` or `iconic`), whether the column and its groups are
floating, the overlay geometry, and its groups' order, visibility, minimized
state, and collapsed state (the version-5 per-group shape nested per column). The
store SHALL also persist `panelRailMode`, `railWidth`, `autoCollapseIconic`, and
`autoShowHidden` as legacy values. A store that is missing a field or older than
version 10 SHALL load that field's documented default while preserving every
field it already has; a store with no per-column layout SHALL load a single
right-hand column built from the legacy per-group state, a version-7 per-column
entry with no rail mode SHALL load the mode from the legacy top-level
`panelRailMode` for every column, a version-6 per-column entry with no width
SHALL load the default width (seeded from the legacy `railWidth` for the primary
column), a version-9 entry without the floating fields SHALL load docked, and the
load-then-write path SHALL preserve unknown keys and the version-4
`toolsColumns` and `useShiftKeyForToolSwitch` values. The system SHALL restore
each column's own rail mode and floating state, and SHALL NOT apply one column's
mode to every column.

#### Scenario: Panel column state round-trips [m41_session]

- **WHEN** the mode, strip width, and per-group state are changed, the session
  is saved, and the store is reloaded
- **THEN** the changed values are restored

#### Scenario: An older store loads the defaults [m41_session]

- **WHEN** a schema-4 store with none of the v5 fields is loaded
- **THEN** the mode is `normal`, `autoCollapseIconic` and `autoShowHidden` are
  off, and the existing keys are unchanged

#### Scenario: The multi-column layout round-trips [m43_session]

- **WHEN** a left column and a right column with different groups and different
  rail modes are arranged, the session is saved, and the store is reloaded
- **THEN** each column's side, order, width, rail mode, floating state, overlay
  geometry, and group state are restored

#### Scenario: A version-5 store loads a single right-hand column [m43_session]

- **WHEN** a schema-5 store with a top-level per-group state but no per-column
  layout is loaded
- **THEN** the workspace shows one right-hand column containing those groups and
  the existing keys are preserved

#### Scenario: A version-6 store loads a default width [lpr_v7_width]

- **WHEN** a schema-6 store with per-column entries that carry no width and a
  legacy top-level `railWidth` is loaded
- **THEN** the primary column's width is the legacy `railWidth` and every other
  column loads the default width

#### Scenario: A version-7 store seeds every column's rail mode [lpr_v7_railmode]

- **WHEN** a schema-7 store with per-column widths but no per-column rail mode
  and a legacy top-level `panelRailMode` is loaded
- **THEN** every column loads the legacy `panelRailMode` value

#### Scenario: A non-primary column's mode round-trips [lpr_column_railmode]

- **WHEN** a non-primary column is toggled from `iconic` to `normal`, the session
  is saved, and the store is reloaded
- **THEN** that column restores as `normal` while the other columns keep their
  own modes

### Requirement: Preferences dialog

The system SHALL provide a Preferences dialog with `General` and `Interface`
panes, reachable from the tab menu's `Interface Options…` and, when the command
tree's `Edit > Preferences > General` leaf is implemented, from
`Edit → Preferences…`. The Interface pane SHALL expose `Use Shift Key For Tool
Switch`, `Auto-Collapse Iconic Panels`, and the column width-toggle default;
applying a change SHALL persist it and apply it to the toolbox and column
without a restart. The remaining CS6 preference panes SHALL stay disabled and
unimplemented.

#### Scenario: The Interface pane carries the preferences [m41_prefs]

- **WHEN** the Preferences dialog is opened on the Interface pane
- **THEN** it shows `Use Shift Key For Tool Switch`, `Auto-Collapse Iconic
  Panels`, and the column width-toggle default

#### Scenario: A preference round-trips and applies [m41_prefs]

- **WHEN** `Use Shift Key For Tool Switch` is toggled off in the dialog and the
  session is saved and reloaded
- **THEN** the toolbox honours the off value and the store reads it back

#### Scenario: Unimplemented panes stay disabled [m41_prefs]

- **WHEN** the Preferences dialog is shown
- **THEN** only the General and Interface panes are enabled

### Requirement: Iconic strip drag and reorder

The iconic strip SHALL support dragging a panel icon to reorder it within the
strip, and SHALL support dropping a dragged icon onto the normal-mode group
stack to move the panel into the target group. The strip drag SHALL reuse the
`PanelColumn` drag machinery used by tabs and groups rather than a second drag
system, and a strip reorder SHALL persist through the existing per-group panel
order.

#### Scenario: Dragging a strip icon reorders it [m42_dragstrip]

- **WHEN** a panel icon in the iconic strip is dragged to another position in
  the strip
- **THEN** the strip and the stored panel order reflect the new order

#### Scenario: Dropping a strip icon on the group stack moves the panel [m42_dragstrip]

- **WHEN** a panel icon is dragged from the strip onto a normal-mode group's tab
  bar or the space between groups
- **THEN** the panel moves into that group, and the column returns to displaying
  the group stack

### Requirement: Floating panel overlay

A group torn off a column SHALL float in a frameless overlay that follows the
cursor and re-docks on release, and the overlay SHALL be able to cross and be
placed around the docked or pane-hosted Tools panel. The overlay's hosting SHALL
be platform-adaptive. Where the platform lets a client position its own
top-level windows, the overlay SHALL be a frameless `Qt::Tool` top-level window
parented to (transient for) the main window — no title bar, no window
decorations, and no taskbar entry — rather than an in-window child widget, so it
SHALL NOT be clipped to the main window, and its movement SHALL be clamped to
the available geometry of the screen under the target point (falling back to the
frame's screen, then the primary screen) and SHALL NOT be clamped to the main
window rect. Where the platform does not let a client position its own top-level
windows, the overlay SHALL instead be an in-window child of the frame that
follows the cursor by parent-relative coordinates and is clamped to the frame
rect, so it cannot be lost. Either way the overlay SHALL offer a resize grip in
its bottom-right corner that resizes the overlay itself by the drag delta,
clamped to the overlay's minimum size, and SHALL NOT resize the main window. The
overlay SHALL show a close control at the rightmost side of its header; closing
SHALL hide the group's panels while keeping the group restorable from
`Window > Panels`, then remove the overlay. The overlay SHALL support a
non-resizable mode in which the grip is absent and the overlay takes the minimum
its content needs; the floating Tools column SHALL use that mode.

#### Scenario: A torn-off group floats and re-docks [m41_tearoff]

- **WHEN** a group is dragged out of a column and dropped back on a column
- **THEN** it floats in a frameless overlay and re-docks

#### Scenario: A floating group can be closed [m47_float_close]

- **WHEN** a floating group's header close control is activated
- **THEN** the overlay is removed, its panels are hidden, and the group is still
  present in a column so `Window > Panels` can restore it

#### Scenario: The overlay can cross the Tools panel [m47_float_over_tools]

- **WHEN** a group is dragged toward the docked Tools panel
- **THEN** the overlay follows the cursor over the Tools panel instead of
  stopping at the central area edge

#### Scenario: The resize grip resizes the overlay [fp_grip_resize]

- **WHEN** the floating overlay's corner grip is dragged
- **THEN** the overlay's size changes by the drag delta and is not shrunk below
  its minimum, while the main window's size does not change

#### Scenario: The floating Tools column is not resizable [fp_tools_float_fixed]

- **WHEN** the Tools column is floating
- **THEN** no resize grip is offered and the overlay is sized to the minimum the
  tool grid's content needs, taking its content width and minimum height

#### Scenario: The overlay is clamped to the screen [fp_float_screen_bounds]

- **WHEN** a floating overlay is moved toward or past the edge of its screen
- **THEN** it is clamped inside that screen's available geometry and may sit
  outside the main window's rect

#### Scenario: Hosting follows the platform capabilities [fp_float_host_adapt]

- **WHEN** a panel or group is torn off
- **THEN** it hosts as a frameless top-level window on a platform that permits
  client positioning of top-level windows, and as an in-window child of the
  frame otherwise

#### Scenario: The in-window overlay follows the cursor [fp_float_child_follow]

- **WHEN** the overlay is hosted in-window and dragged
- **THEN** it moves parent-relative by the cursor delta, stays inside the frame
  rect, and is clipped to the frame

### Requirement: Per-widget panel header action menu

Each panel group's tab header SHALL carry a context-action button at its far
right. Activating it SHALL show the menu of the panel that is current in the
group, so the menu is per-widget rather than per-group. Each menu SHALL contain
that panel's researched CS6 entries as recorded in `docs/dev/m42-panel-menus.md`,
in a stable order, and SHALL also offer `Close` (close the group's active tab)
and `Close Group` (close the whole group) at the end. An entry that is not
implemented SHALL be shown disabled with the tooltip `<label> — not implemented
yet` and SHALL NOT fake behaviour. `Close` and `Close Group` SHALL close the
active panel and the group through the same close path the tab context menu
uses. The Layers panel's `Panel Options…` entry SHALL open the existing Panel
Options behaviour.

#### Scenario: The header button shows the current panel's menu [m42_widgetmenu]

- **WHEN** the tab-header action button is activated for a group whose current
  panel is Layers
- **THEN** the Layers per-widget menu is shown

#### Scenario: Unimplemented entries are disabled with a tooltip [m42_widgetmenu]

- **WHEN** a per-widget menu contains an entry whose feature is not implemented
- **THEN** that entry is disabled and its tooltip is
  `<label> — not implemented yet`

#### Scenario: Close entries are present and act [m42_widgetmenu]

- **WHEN** a per-widget menu is shown and its `Close` or `Close Group` entry is
  chosen
- **THEN** the menu contains both entries, and choosing `Close` hides the active
  tab while `Close Group` hides the whole group

#### Scenario: Layers Panel Options still opens [m42_widgetmenu]

- **WHEN** `Panel Options…` is chosen from the Layers per-widget menu
- **THEN** the existing Layers Panel Options behaviour runs

### Requirement: Multi-column drop targets

The drop resolver SHALL be extended, not duplicated, to resolve drops against
the ordered column set: a drop left or right of an existing column, of a group
at the workspace edge, or of the Tools toolbar SHALL resolve to a new column on
that side; a whole widget column SHALL resolve to a new-column or boundary target
on any side of the Tools toolbar, another widget panel or column, and the
workspace; a drop inside an existing widget group SHALL resolve to a tab insert
into that group, including in compact mode; and a drop above or below an
existing group SHALL resolve to a boundary insert, including in compact mode. A
cross-column result SHALL carry the column that owns the target, so the
insertion indicator and the commit both act on that column: a tab insert, a
group-body insert, and a boundary insert in another column SHALL each name the
target column. In compact mode a drop onto a group or within a small band just
above or below it SHALL resolve to an insert into that group, and a drop between
groups or beyond the top/bottom group SHALL resolve to a new group at that
boundary. The resolver SHALL cover both normal and iconic modes and both floating
and non-floating drags, and SHALL use one insertion indicator rather than a
second indicator system.

#### Scenario: Dropping left of a column creates a left column [m43_newcolumn]

- **WHEN** a dragged group is dropped to the left of an existing column
- **THEN** a new column is created on the left and the group is placed in it

#### Scenario: Dropping right of a column creates a right column [m43_newcolumn]

- **WHEN** a dragged group is dropped to the right of an existing column
- **THEN** a new column is created on the right and the group is placed in it

#### Scenario: Dropping beside the Tools toolbar creates a column [m43_newcolumn]

- **WHEN** a dragged group is dropped beside the Tools toolbar
- **THEN** a new column is created on that side and the group is placed in it

#### Scenario: Dropping inside a group inserts a tab [m43_intogroup]

- **WHEN** a dragged panel is dropped inside an existing widget group
- **THEN** the panel becomes a tab of that group

#### Scenario: Dropping inside a compact group inserts a tab [m43_intogroup]

- **WHEN** the workspace is in compact mode and a dragged panel is dropped
  inside an existing group
- **THEN** the panel becomes a tab of that group

#### Scenario: Dropping above or below a group inserts at the boundary [m43_boundary]

- **WHEN** a dragged item is dropped above or below an existing group
- **THEN** a new group is inserted at that boundary containing the dragged item

#### Scenario: Dropping above or below a compact group inserts at the boundary [m43_boundary]

- **WHEN** the workspace is in compact mode and a dragged item is dropped above
  or below an existing group
- **THEN** a new group is inserted at that boundary containing the dragged item

#### Scenario: A whole widget column resolves to any-side targets [m44_docksides]

- **WHEN** a whole widget column is dragged to a side of the Tools toolbar,
  another widget panel or column, or the workspace
- **THEN** the resolver returns a target on that side and the column is docked
  there through the existing commit path

#### Scenario: A compact group-relative drop is resolved [m44_compactdrop]

- **WHEN** the workspace is in compact mode and an icon is dropped onto a group
  or just above/below it, or between/beyond groups
- **THEN** the resolver returns an into-group target for the former and a new
  boundary group for the latter

#### Scenario: A cross-column target names its owning column [m45_indicator_cross_column]

- **WHEN** a drag resolves to a tab, group-body, or boundary target inside
  another widget column
- **THEN** the resolver result names that target column, and the indicator is
  drawn and the drop committed in it

### Requirement: Panel tab bar active colour

The panel-group tab bar SHALL carry an `objectName` (`panelTabBar`) and SHALL be
styled by a selector scoped to that name. The active tab's background SHALL match
the panel/widget background (the `QTabWidget::pane` `${base}` colour), and an
inactive tab's background SHALL NOT match it (using `${window}` with `${hover}`
on hover). The direction of this mapping is a correctness requirement: the
previous implementation painted the active tab with the inactive `${window}`
colour and vice versa, and that inversion SHALL be corrected rather than
reintroduced. The document tab bar SHALL be unaffected unless its styling is
deliberately changed.

#### Scenario: The active tab matches the pane background [m43_tabcolors]

- **WHEN** a panel group's active tab is inspected
- **THEN** its background is the pane `${base}` colour

#### Scenario: An inactive tab does not match the pane background [m43_tabcolors]

- **WHEN** a panel group's inactive tab is inspected
- **THEN** its background is the `${window}` colour and differs from the pane
  `${base}` colour

#### Scenario: The document tabs are unaffected [m43_tabcolors]

- **WHEN** the panel tab styling is applied
- **THEN** the document tab bar keeps its existing styling

#### Scenario: The active and inactive colours are not swapped [m44_tabswap]

- **WHEN** a panel group's active tab and an inactive tab are compared
- **THEN** the active tab's background matches the pane background and the
  inactive tab's does not, with neither using the other's colour

### Requirement: Corner action button visibility

Each panel group's header SHALL always show the full tab bar and the corner
context-action button together, including at the column's minimum width. Tab
text SHALL elide rather than push the corner button out, the header SHALL reserve
the corner button's width, and the column's minimum-width calculation SHALL
account for the corner button. The corner button SHALL NOT be clipped or pushed
outside the group header at the minimum width. The corner button SHALL use the
bundled Lucide `menu` glyph (asset id `panel.menu`) rather than a text character,
SHALL be vertically centred in the header row, and SHALL keep a right margin from
the group's right edge.

#### Scenario: The corner button stays visible at minimum width [m43_corner]

- **WHEN** the column is at its minimum width
- **THEN** the corner context-action button is fully inside the group's
  tab-header row and is not clipped

#### Scenario: Tab text elides instead of hiding the corner button [m43_corner]

- **WHEN** the group's available tab-bar width is less than the tab text needs
- **THEN** the tab text elides and the corner button remains visible

#### Scenario: The corner button uses the menu glyph [pc_corner_menu_glyph]

- **WHEN** a group header's corner button is shown
- **THEN** it draws the `panel.menu` Lucide glyph, centred vertically with a
  right margin from the group's right edge

### Requirement: Panel widget theme and borders

Panels and columns SHALL draw a darker grey border from the shared theme: the
Tools toolbar, and both the normal and the compact widget panels. The divider
between two widget groups SHALL be thicker and darker grey than the default
splitter handle. The line below a widget column's header and the last line at the
column's bottom SHALL each be 3 px. The column collapse toggle's double-chevron
icon SHALL match the action it performs: the collapse-to-icons icon in `normal`
mode and the expand icon in `iconic` mode, with the two icons not inverted.
Border and divider dimensions SHALL be single named constants rather than
per-site literals.

#### Scenario: Panels draw the darker grey border [m44_panelborder]

- **WHEN** the Tools toolbar and a normal-mode widget panel are shown
- **THEN** each draws a darker grey border from the shared theme

#### Scenario: The group divider is thicker and darker [m44_divider]

- **WHEN** two widget groups are stacked in a column
- **THEN** the handle between them is thicker and darker grey than the platform
  default

#### Scenario: The column header and bottom rules are 3 px [pc_column_rule_3px]

- **WHEN** a widget column is shown
- **THEN** the line below its header and the last line at its bottom are each
  3 px from the shared constant

#### Scenario: The collapse toggle icon matches its action [m44_chevrons]

- **WHEN** the column collapse toggle is inspected in `normal` and in `iconic`
  mode
- **THEN** each mode shows the icon for the action it performs and the two icons
  are not inverted

### Requirement: Widget presentation parity

A widget SHALL use the same colour scheme and style whether it is docked, shown
in a compact flyout, or floating. The three presentations SHALL share the same
scoped stylesheet selectors and the same container chrome (tab bar, background,
borders) rather than per-presentation colour literals. Within a group header, the
reserved drag area, the corner container, and the corner action button SHALL
share one header background, and the corner action button SHALL be vertically
centred in the header row. A collapsed group's icon row SHALL use the docked icon
strip's group-box chrome.

#### Scenario: The flyout and float match the docked widget [m44_popupstyle]

- **WHEN** the same panel is presented docked, in a compact flyout, and in a
  float
- **THEN** all three use the same background, borders, and tab-bar styling

#### Scenario: The corner button matches the header [m44_cornerstyle]

- **WHEN** a group header is shown
- **THEN** the reserved drag area, the corner container, and the corner action
  button share one background and the button is vertically centred

### Requirement: Default active panel is first

On a fresh session the first panel SHALL be the active one, not the last, in
whichever artefact carries the default active state — the default active tab of
a group, the default current panel of a column, or the default current page. The
first tab of each default group SHALL be current, and the column's current panel
SHALL be the first visible panel.

#### Scenario: The first panel is active by default [m44_defaultactive]

- **WHEN** the frame starts with a fresh session
- **THEN** the first tab of each default group is current and the first visible
  panel is the column's current panel

### Requirement: Panel column minimum width and no clipping

Every widget column in `normal` mode SHALL enforce a single shared minimum width
floor of at least 300 device-independent pixels, equal for all widget columns and
for every floating widget overlay, large enough that the column's content is
always fully visible horizontally, so no column can be resized below it or
disappear and no column ever displays a horizontal scrollbar or hides content on
its right edge. The floor SHALL be raised further by content that needs more,
capped at the shared maximum, and SHALL be applied to the column so a wider
column — not internal scrolling — is what keeps content visible. The column
SHALL elide its tab text and keep the header corner action button inside the
header rather than cutting either off. The iconic strip SHALL keep its own
narrow, fixed minimum separate from the shared normal-mode floor.

#### Scenario: All widget columns share one minimum width [m45_min_width_floor]

- **WHEN** two or more widget columns are present and their minimum widths are
  queried
- **THEN** every normal-mode column reports the same shared floor and none can be
  resized below it or vanish

#### Scenario: A column never horizontally scrolls or clips its content [m47_no_hscroll]

- **WHEN** a widget column is at its minimum width
- **THEN** its horizontal scrollbar policy is off, its content fits within the
  viewport, and no right-side content is hidden

#### Scenario: The shared floor is at least 300 pixels [wpx_min_width_300]

- **WHEN** any normal-mode widget column and any floating widget overlay are at
  their minimum width
- **THEN** each reports a minimum width of at least 300 pixels

### Requirement: Empty columns and ghost groups are cleaned up

A column that has no group with visible content SHALL be removed (dynamic) or
hidden (primary host) after a drop, even while it owns a live floating overlay;
before removal its floats SHALL be re-homed to the primary column so they stay
re-dockable. A group whose tabs are all hidden SHALL be hidden rather than left
as a visible but un-grabbable shell, and SHALL remain restorable from
`Window > Panels`.

#### Scenario: The source column disappears after its last group floats [m47_empty_after_float]

- **WHEN** the last group of a dynamic column is dragged out and left floating
- **THEN** the empty source column is removed and the float remains re-dockable

#### Scenario: A group with no visible tabs is hidden [m47_ghost_group]

- **WHEN** the last visible tab of a group is moved elsewhere
- **THEN** the emptied group is hidden instead of showing an empty tab bar, and
  its hidden panels can be re-shown from `Window > Panels`

### Requirement: Column header drag

The top header of a `PanelColumn` SHALL act as a drag handle for the whole
column — the row carrying the `panelColumnToggle`. A left-button press on the
header followed by movement past the drag threshold SHALL move the column and
SHALL show the same single insertion indicator the widget drag grammar uses; on
release the column SHALL land immediately to the left or right of the widget
column under the pointer, or at the workspace's outermost left or right position
when no column is under it — including to the left of the Tools panel when the
Tools panel is hosted as a splitter pane. A release that resolves no target SHALL
leave the column where it is. A left-button press and release on the header that
does not exceed the drag threshold SHALL open a header menu containing
`Collapse to Icons` (toggling the column's icon mode), `Auto-Collapse Iconic
Panels` and `Auto-show Hidden Panels` (checkable and bound to the column's
flags), and `Interface Options…` (opening the Interface preferences pane). A
click on the width toggle SHALL still switch the column's normal/icon mode and
SHALL NOT start a column drag or open the header menu.

#### Scenario: The header drags the column beside another [pc_col_header_move]

- **WHEN** the top header of a widget column is pressed and dragged onto another
  column
- **THEN** the insertion indicator is shown and on release the dragged column is
  placed on that side of the target column

#### Scenario: The column lands at the outermost position [pc_col_header_edge]

- **WHEN** the header is dragged to the workspace's outer left band with the
  Tools panel hosted as a splitter pane at that edge
- **THEN** the column lands at the splitter's outermost position, to the left of
  the Tools panel

#### Scenario: The header menu toggles the column options [pc_col_header_menu]

- **WHEN** the column header is clicked without dragging and a menu entry is
  chosen
- **THEN** `Collapse to Icons` toggles the column's icon mode, the two auto
  toggles flip their column flags, and `Interface Options…` opens the Interface
  preferences pane

#### Scenario: The toggle still toggles [pc_col_header_toggle]

- **WHEN** the column's width toggle is clicked
- **THEN** the column switches between normal and icon mode, no column drag
  starts, and no header menu opens

### Requirement: Panel group tab overflow

When a panel group's tabs do not fit its tab bar, the tabs SHALL compress and
elide to share the available width rather than showing scroll arrow buttons, and
the per-widget corner control SHALL keep its place.

#### Scenario: Many tabs squeeze instead of scrolling [pc_tabs_compress]

- **WHEN** a group has more visible tabs than fit the column width
- **THEN** the tabs are squeezed and elided and no scroll-arrow buttons appear

### Requirement: Reserved panel-group drag grip

Each panel group SHALL reserve a small blank drag area at the right of its tab
bar, before the per-widget menu control, and a press-drag on it SHALL drag the
whole group through the same drag grammar as the empty tab-bar area. The reserved
area SHALL remain present when the tabs fill or overflow the bar.

#### Scenario: The reserved grip drags the group [pc_group_grip]

- **WHEN** the reserved area to the right of the tabs is pressed and dragged
- **THEN** the whole group is dragged, even when the tab bar is full

### Requirement: Floating group top bar and icon mode

A floating panel group SHALL carry its own top bar with a normal/icon width
toggle and a close control, the toggle placed immediately to the left of the
close control on the bar's right side, and the bar SHALL drag the overlay. The
toggle SHALL collapse the group to its icon row and expand it again, and the
close control SHALL remain reachable while collapsed. While collapsed the
overlay SHALL use the smallest size its icon row needs on both axes — snapping
to the icon row's height and shrinking its width to the icon row's own width so
no normal-width body remains — and SHALL NOT offer the resize grip; expanding
SHALL restore the resize grip and the shared normal minimum width. The icon row
SHALL render as the docked icon strip's group box with the same container
styling, grip divider, and icon size, and SHALL stack its icons in a vertical
column, one icon per row, matching the docked strip. Each icon in the floating
row SHALL be draggable through the same drag grammar the docked strip uses, so a
panel can be torn out of the floating group and the group can be dragged and
dropped elsewhere; a click below the drag threshold SHALL still open the panel.
The overlay SHALL offer a resize grip that resizes the overlay itself and keep a
minimum size like a docked column. When the floating group holds a single visible
panel, dragging its tab SHALL move the whole overlay rather than tear off a
second overlay and leave a ghost. A dragged overlay SHALL be dimmed for the whole
drag, from the start of the gesture until it ends, not only while it is over a
valid drop target.

#### Scenario: The float top bar toggles and closes [pc_float_header]

- **WHEN** a group floats
- **THEN** its top bar shows a collapse toggle immediately left of a close
  control on the right, toggling collapses and expands the group, and dragging
  the bar moves the overlay

#### Scenario: The floating icon overlay snaps to the icon row [pc_float_icon_min]

- **WHEN** a floating group is collapsed to icons
- **THEN** the overlay snaps to the icon-row height (at least the icon-row
  minimum), shrinks its width to the icon row's natural width so no
  normal-width body remains, the icon row renders as a grouped panel-icon box,
  and the close control remains reachable

#### Scenario: A collapsed float shows no resize grip [wpx_float_icon_no_grip]

- **WHEN** a floating group is collapsed to icons
- **THEN** the overlay does not show its resize grip, keeps the smallest width
  and height its icon row needs, and shows the grip again when expanded

#### Scenario: The floating icon row stacks vertically [wpx_float_icon_vertical]

- **WHEN** a floating group with more than one icon is collapsed to icons
- **THEN** its icons are arranged one per row in a vertical column

#### Scenario: A floating icon drags its panel [wpx_float_icon_drag]

- **WHEN** an icon in a floating collapsed group is pressed and dragged past the
  drag threshold
- **THEN** that panel starts a drag through the same grammar the docked strip
  uses and can be dropped elsewhere, while a click below the threshold opens the
  panel

#### Scenario: The floating overlay is resizable [pc_float_resize]

- **WHEN** a floating group is shown expanded and its corner grip is dragged
- **THEN** it carries a resize grip that resizes the overlay itself and it keeps
  a minimum size comparable to a docked column

#### Scenario: A single-panel float moves as a whole [pc_float_single_drag]

- **WHEN** the tab of a floating group that holds one visible panel is dragged
- **THEN** the whole overlay moves and no second overlay or empty source overlay
  is left behind

#### Scenario: A dragged overlay dims for the whole drag [pc_float_dim]

- **WHEN** an overlay is dragged
- **THEN** it is dimmed from the start of the drag through release, including
  while the target is invalid, and it returns to full opacity when the drag ends

### Requirement: Whole-column in-window float

The system SHALL allow a whole widget `PanelColumn` to be dragged by its header
and torn off the workspace into a frameless floating overlay, through the same
drag machinery and the same single insertion indicator the panel and group drags
use. The overlay SHALL use the same platform-adaptive hosting as a torn-off
group: a frameless `Qt::Tool` top-level window parented to the main window where
the platform permits client positioning of top-level windows, and an in-window
child of the frame otherwise; in neither case SHALL it be a decorated
operating-system window. It SHALL keep a minimum width and SHALL offer a resize
grip. A widget-column overlay SHALL open at about two thirds of the column's
docked height, clamped to the overlay minimum, whether the column is in normal
or iconic mode. When the rail mode is toggled to iconic while the column is
floating, the overlay SHALL snap to the column's content height and its narrow
content width. A release over a valid column or workspace edge SHALL re-place the
column; a release with no valid target SHALL leave the overlay floating.

#### Scenario: Dragging a whole column floats it in-window [pc_col_float]

- **WHEN** a widget column's header is dragged out of the workspace
- **THEN** the whole column floats in a frameless overlay that follows the
  cursor and is committed on release

#### Scenario: The floating column keeps a minimum width and a grip [pc_col_float_grip]

- **WHEN** a floating whole-column overlay is shown
- **THEN** it carries a resize grip and cannot be resized below a minimum width

#### Scenario: The floating column opens below its docked height [pc_col_float_height]

- **WHEN** a widget column is torn off and floats
- **THEN** its default height is strictly less than its docked height

#### Scenario: An iconic column tear-off keeps the two-thirds height [pc_col_float_iconic_initial]

- **WHEN** a widget column that is already in iconic mode is torn off
- **THEN** the overlay opens at about two thirds of the docked height, clamped to
  the overlay minimum, rather than collapsing to the icon-row minimum

#### Scenario: An iconic column float snaps narrow [pc_col_float_iconic_snap]

- **WHEN** a floating whole-column overlay's rail mode is toggled to iconic
- **THEN** the overlay snaps to the column's content height and narrow content
  width instead of keeping the normal-width body

#### Scenario: The floating column re-places [pc_col_redock]

- **WHEN** a floating whole-column overlay is dropped on a column or workspace
  edge
- **THEN** the whole column re-places at that position and the overlay is removed

#### Scenario: A whole-column overlay follows the cursor in-window [pc_col_float_child]

- **WHEN** a whole-column overlay is hosted in-window and its header is dragged
- **THEN** it moves parent-relative by the cursor delta and stays clamped to the
  frame rect

### Requirement: Floating panel as a drop target

A floating panel overlay SHALL be a drop target in the same grammar as a column.
A dragged single panel dropped onto a floating overlay SHALL become a tab of that
overlay's group, and a dragged group dropped onto it SHALL merge into that
overlay's group. The overlay SHALL show the shared insertion indicator while it
is the resolved target.

#### Scenario: A dragged panel drops into a float [pc_float_drop_panel]

- **WHEN** a single panel is dragged onto an existing in-window floating panel
- **THEN** the panel becomes a tab of the floating group and the shared
  indicator marks the target

#### Scenario: A dragged group merges into a float [pc_float_drop_group]

- **WHEN** a whole group is dragged onto an existing in-window floating panel
- **THEN** the dragged group's panels join the floating group

#### Scenario: A floating icon group merges into another float [wpx_float_drop_icon_group]

- **WHEN** a collapsed floating group is dragged by the drag grammar onto another
  floating panel
- **THEN** its panels join that floating group's tabs

### Requirement: Group-on-group tabify outline

Dropping a whole group onto another group SHALL merge the two groups, with the
dragged group's panels joining the target group as tabs. While a dragged group is
resolved to a target group — whether the pointer is over that group's tab bar or
its body — the target group SHALL be highlighted with a blue outline drawn
around its whole region rather than only the thin insertion line, and the outline
SHALL clear when the drag moves away or is cancelled. A drop over the dragged
group's own body SHALL remain an above/below reorder boundary so groups can still
be reordered in place.

#### Scenario: A group dropped on a group tabifies into it [pc_group_tabify]

- **WHEN** a whole group is dropped onto another group
- **THEN** the dragged group's panels become tabs of the target group

#### Scenario: The target group is outlined [pc_group_outline]

- **WHEN** a dragged group is resolved to a target group
- **THEN** a blue outline is drawn around the target group's whole region, and it
  clears when the drag leaves or is cancelled

#### Scenario: A group body drop outlines the whole target group [wpx_group_body_outline]

- **WHEN** a whole group is dragged over another group's body, away from its tab
  bar
- **THEN** the blue outline is drawn around that whole group and releasing merges
  the dragged group into it

### Requirement: Group header full-width background

The panel-group header background SHALL match the inactive tab background and
SHALL span the full width of the group, including the area behind the right-hand
per-widget context-menu corner button, so the header reads as one continuous
surface with no gap, no differently coloured corner region, and no lighter
horizontal line above the tabs.

#### Scenario: The header background covers the corner button [pc_header_full_width]

- **WHEN** a panel group's header is shown
- **THEN** the header background matches the inactive tab background, extends
  behind the corner context-menu button with no gap or separate corner colour,
  and draws no lighter horizontal line

### Requirement: Whole-drag dim and cancel cleanup

A dragged panel, group, or column SHALL be dimmed for the entire drag, from the
start of the gesture until it ends, not only while the pointer is over a valid
drop target. When a drag is cancelled, the dragged source SHALL leave no ghost
overlay and the layout SHALL be unchanged.

#### Scenario: A dragged item dims for the whole drag [pc_whole_drag_dim]

- **WHEN** a panel, group, or column is dragged
- **THEN** it is dimmed from the start of the drag through release, including
  while the target is invalid

#### Scenario: A cancelled drag leaves no ghost [pc_drag_cancel_clean]

- **WHEN** a drag is cancelled
- **THEN** the dragged source returns to full opacity with no ghost overlay and
  the layout is unchanged

### Requirement: Floating icon row parity

A floating compact/icon row SHALL behave like the docked icon strip. Clicking an
icon in a floating icon row SHALL open the same persistent flyout hosting the
whole group with the clicked panel current, and the group's drag-handle grip
SHALL drag the whole group through the same drag grammar the docked strip uses.
The flyout SHALL obey the same persistence, close-control, toggle/switch, and
one-per-column rules as the docked strip's flyout. It SHALL open on the inner
side of the floating row and SHALL restore the group when it closes. While the
group is hosted in the flyout, its floating top bar and float close control
SHALL be hidden; returning the group to the floating row SHALL restore them.

#### Scenario: Clicking a floating icon opens the flyout [pc_float_icon_click]

- **WHEN** an icon in a floating compact row is clicked
- **THEN** the same flyout as the docked strip opens with the clicked panel
  current and remains open when the user clicks elsewhere

#### Scenario: The floating grip drags the group [pc_float_icon_grip]

- **WHEN** the drag-handle grip above a floating row's group is dragged
- **THEN** the whole group is dragged through the docked strip's drag grammar

#### Scenario: The flyout shows only its own header [pc_float_icon_single_header]

- **WHEN** a group from a floating icon row is hosted in the flyout
- **THEN** the floating top bar and its close control are hidden and only the
  flyout's own close control and tab-bar header are visible

### Requirement: Column-edge self-anchor docking

A panel, group, or whole column dragged to the edge band immediately to the left
or right of its own source column SHALL be able to anchor on that column, so a
widget docked on one side of a column can be dropped on the other side and vice
versa. The whole-column move target resolution SHALL keep its existing behaviour.

#### Scenario: A widget docks to the opposite side of its own column [pc_self_anchor_dock]

- **WHEN** a panel or group docked on the left of a column is dragged to the
  right edge band of that same column, or a right-docked one to the left edge
  band
- **THEN** the edge anchor accepts the source column and on release the item
  lands on that opposite side

### Requirement: Fixed flush Tools column

The Tools column SHALL sit in a fixed left slot adjacent to the workspace without
being a splitter pane, so it abuts the workspace directly with no resize handle,
no gap, and no drag seam between them, and its width SHALL NOT change on a
pointer drag. The widget columns beside the workspace SHALL keep their resize
handle and SHALL stay width-draggable from their workspace-facing edge.

#### Scenario: No seam beside the Tools column [pc_tools_flush]

- **WHEN** the Tools column is shown beside the workspace
- **THEN** there is no resize handle, gap, or drag seam between the Tools column
  and the workspace, and its width does not change on a pointer drag

#### Scenario: Widget columns stay draggable [pc_tools_flush_widget_drag]

- **WHEN** a widget column beside the workspace is dragged by its
  workspace-facing handle
- **THEN** its width changes and the workspace absorbs the difference

### Requirement: Iconic strip group separators

In the iconic/icon-strip mode each widget group SHALL draw a 1 px separator line
at its bottom, SHALL NOT draw a line between its drag grip and its icon items,
and the strip SHALL NOT draw a thicker divider between adjacent groups.

#### Scenario: Group bottom line, no grip line, no inter-group divider [pc_iconic_group_lines]

- **WHEN** the iconic strip is shown with several widget groups
- **THEN** each group has a single 1 px line at its bottom, there is no line
  between a group's drag grip and its icons, and there is no thicker divider
  above a group

### Requirement: Iconic column workspace-edge resizing

An iconic/icon-strip column SHALL be resizable from its workspace-facing edge
when it is docked on either the left or the right of the workspace — a
right-docked column from its left edge and a left-docked column from its right
edge — and the resulting width SHALL persist.

#### Scenario: Resize from the workspace edge on either side [pc_iconic_resize]

- **WHEN** an iconic column is docked on the right and its left edge is dragged,
  or docked on the left and its right edge is dragged
- **THEN** its width changes in both cases and the new width is kept

### Requirement: Flyout drag handoff

Dragging out of the compact-strip flyout SHALL hand the hosted `PanelGroup` back
to the drag system without leaving the popup behind as an empty shell. Dragging
the group out of the flyout SHALL close the flyout before the floating overlay
follows the cursor, and the overlay SHALL keep the grab offset the group had in
the popup. Dragging a single tab out of the flyout SHALL keep the flyout open on
the remaining visible panels; when the drag leaves no visible panel, the flyout
SHALL close instead. The existing tear-off, re-dock, cancel, and drop rules SHALL
apply unchanged to the drag.

#### Scenario: A group dragged out of the flyout closes it [cfd_group_tearoff]

- **WHEN** a group is dragged out of an open compact-strip flyout and dropped
  outside every column
- **THEN** the group floats in one overlay, the flyout is no longer visible, and
  no empty popup is left over the workspace

#### Scenario: A tab dragged out keeps the flyout on the remaining panels [cfd_tab_tearoff]

- **WHEN** a tab is dragged out of an open compact-strip flyout whose group still
  has other visible panels and dropped outside every column
- **THEN** only that panel floats, the flyout stays visible, and the flyout's
  group keeps the remaining panels

#### Scenario: A tab that empties the flyout closes it [cfd_empty_tearoff]

- **WHEN** a tab is dragged out of an open compact-strip flyout whose group has
  no other visible panel and dropped outside every column
- **THEN** the panel floats and the flyout is no longer visible

### Requirement: Flyout close control

While a group is hosted in a flyout, its header corner SHALL show a close
control immediately to the left of the per-widget context-menu button. The
control SHALL use the bundled Lucide `chevrons-right` glyph (asset id
`panel.columnsTwo`) and SHALL be hidden while the group is docked or hosted in a
floating overlay. Activating it SHALL hide the flyout and restore the group to
its column position. This control and clicking a panel icon in the same column
SHALL be the only user-initiated close paths; clicking elsewhere and pressing
Escape SHALL NOT close the flyout.

#### Scenario: The close control sits left of the widget menu [pc_flyout_close_placement]

- **WHEN** a group is shown in a flyout
- **THEN** a chevrons-right close control is visible immediately to the left of
  the per-widget context-menu button in the header corner

#### Scenario: The close control hides the flyout [pc_flyout_close_action]

- **WHEN** the flyout close control is activated
- **THEN** the flyout is hidden and the group returns to its column position

#### Scenario: The close control is flyout-only [pc_flyout_close_visibility]

- **WHEN** the same group is docked or floating
- **THEN** its flyout close control is not visible

### Requirement: Flyout width resize

A flyout SHALL be width-resizable by dragging its canvas-facing edge: the left
edge for a right-hand column or floating row, and the right edge for a left-hand
column or floating row. The opposite edge SHALL stay anchored while the width
changes, the height SHALL stay unchanged, and the width SHALL NOT shrink below
the shared normal-mode minimum width. The flyout SHALL open at or above that
minimum width. In child-overlay mode the flyout and its width changes SHALL stay
within the frame rect.

#### Scenario: A right-hand flyout resizes from its left edge [pc_flyout_resize_left]

- **WHEN** a flyout beside a right-hand column is dragged by its left edge
- **THEN** its width changes while its right edge stays fixed

#### Scenario: A left-hand flyout resizes from its right edge [pc_flyout_resize_right]

- **WHEN** a flyout beside a left-hand column is dragged by its right edge
- **THEN** its width changes while its left edge stays fixed

#### Scenario: The flyout keeps its minimum width [pc_flyout_resize_min]

- **WHEN** a flyout is dragged narrower than the shared normal-mode minimum width
- **THEN** it stops at that minimum and does not collapse below it

#### Scenario: A floating-row flyout resizes from its canvas edge [pc_flyout_resize_floating_row]

- **WHEN** a flyout opened from a floating icon row is dragged by its
  canvas-facing edge
- **THEN** its width changes and the opposite edge stays fixed

#### Scenario: The flyout opens at the minimum width [pc_flyout_resize_open_width]

- **WHEN** a flyout is opened
- **THEN** its initial width is at least the shared normal-mode minimum width

#### Scenario: The child-mode flyout stays inside the frame [pc_flyout_child_bounds]

- **WHEN** a flyout is hosted as an in-window child and opened or width-resized
- **THEN** its rect stays within the frame rect

### Requirement: Floating group header chrome

A floating group's top bar SHALL use the same panel-header surface as a docked
column header. The per-widget context-menu button in a floating group's tab-bar
corner SHALL be fully painted and hit-testable, with the same hit rectangle it
has while docked; it SHALL NOT be visually absent while retaining its hit area.

#### Scenario: The float top bar matches the docked header [pc_float_header_surface]

- **WHEN** a floating group's top bar and a docked column header are compared
- **THEN** both use the shared panel-header background

#### Scenario: The floating corner menu is painted and clickable [pc_float_corner_menu_painted]

- **WHEN** a group is floating
- **THEN** its corner context-menu button is painted, occupies the same hit
  rectangle as when docked, and opens its per-widget menu
