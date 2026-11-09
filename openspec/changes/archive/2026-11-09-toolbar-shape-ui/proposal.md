# Tools panel shape and header

## Why

The Tools panel (the toolbar) does not match CS6: it has no visible drag
handle, its docked body scrolls when the workspace is short, and its header
shows only the column toggle. Issue #1264 reports the missing drag icon and the
scrollbars, and asks for exactly two layouts — a compact single column
and an expanded two-column layout — both draggable and dockable on either side
of the canvas.

The current tools column hosts its content in a `QScrollArea`, so a workspace
shorter than the tool grid grows a vertical scrollbar instead of behaving like
a panel. The column header is draggable but gives no visual affordance, and it
has no close control.

## What Changes

- The Tools panel has exactly two layouts: **compact** (one tool column) and
  **expanded** (two tool columns), toggled by the CS6 double-arrow in the
  header. Neither layout shows a scrollbar.
- The header becomes a CS6-style title bar: the double-arrow toggle, a close
  (`×`) control, and a full-width `•••` drag grip below the title row matching
  the iconic History/Properties panel-group grip. Both the grip and the title
  bar drag the whole column, including out of the main window as the existing
  frameless `Qt::Tool` overlay.
- The close control hides the Tools panel; `Window > Panels > Tools` restores
  it.
- When the docked workspace is shorter than the current layout needs, the
  Tools panel stays visible and clips at the workspace bottom instead of
  scrolling or hiding. A floating panel keeps hugging its content.
- A floating panel moves freely on the desktop and only shows a dock indicator
  when the pointer is inside the workspace's left or right edge band.
- The existing left/right docking, fixed content width, atomic (ungroupable)
  column, and 1↔2 column persistence are unchanged.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ui/application-shell`: the Tools panel gains its two named layouts, the
  CS6 header (grip, toggle, close), a no-scrollbar rule, short-window clipping,
  and an inside-the-workspace-only floating re-dock rule.

## Impact

- **C++ app**: `panels/panel_column.{h,cpp}`, `panels/panel_column_drag.cpp`,
  `panels/panel_column_test.cpp`, `panels/panel_float.cpp`, `toolbox.{h,cpp}`,
  `frame_build.cpp`, `frame_columns.cpp`, `frame.cpp`, `frame.h`, and the theme
  stylesheet.
- **Tests**: a new `tst_toolbar_shape` Qt suite plus updates to the existing
  suites that assume the toolbar is visible at 1100×700.
- **No document-format change, no new dependency, no `docs/` edit.**
