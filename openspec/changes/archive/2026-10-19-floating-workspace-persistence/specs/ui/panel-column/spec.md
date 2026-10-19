# Spec Delta

## MODIFIED Requirements

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

## ADDED Requirements

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
