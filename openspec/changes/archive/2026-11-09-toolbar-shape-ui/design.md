## Context

The Tools panel is the atomic `PanelColumn` (`toolsContent_`) hosted in the
central splitter. Its content is the `Toolbox` widget, wrapped in a
`QScrollArea` (`toolsScroll_`). The column header currently carries only the
1↔2 column toggle and is the whole drag handle. Issue #1264 asks for the CS6
shape: a `•••` grip, a close control, exactly two layouts, no scrollbars, and
hiding instead of scrolling when the workspace is too short.

## Decisions

### Two layouts, one content widget

Keep `Toolbox::setColumns`/`reflow` as the single layout switch. Compact is one
column; expanded is two. The header double-arrow already calls the frame's
toggle callback and the mode is already persisted as `toolsColumns`, so no new
state is needed. `Toolbox::contentWidth` remains the one width source; the
tools column stays min==max pinned to it.

### No scrollbars: the scroll host stays, but never scrolls

The scroll host is still needed so the tools content cannot force the main
window's minimum height (the existing `las_tools_free_height` contract). Set
its vertical policy to `AlwaysOff` (horizontal is already `AlwaysOff`). When the
workspace is shorter than the current layout, the scroll host clips the content
at its bottom; the column stays visible, matching the reporter's rule that the
toolbar is present on startup even when the window cuts it off.

`PanelColumn::toolsRequiredHeight()` still reports the header, grip, content
minimum, and layout margins for the current mode, but only the floating overlay
uses it to hug the full grid. The tools content is top-aligned and fills the
docked height as before, so the existing `las_tools_top_align` contract is
unchanged.

### Header: title row + grip

`PanelColumn` builds one header row for every column. For a tools column it
reorders the toggle to the left, adds a close `QToolButton` on the right, and
inserts a `ToolbarDragGrip` widget below the title row. The grip is the same
`•••` label the iconic History/Properties panel group uses, so the two drag
affordances read alike, but its band is taller so the dots have vertical padding
above and below. It carries a move cursor and tooltip. Widget
columns keep their existing header (stretch + toggle, no close, no grip).

The existing header drag path in `panel_column_drag.cpp` is shared: the grip is
watched by the same event filter and sets the same press/move/release state, so
there is one column-move implementation. A grip click without a drag does
nothing (the title-row click still opens the column menu).

The close button emits `PanelColumn::closeRequested()`. The frame hides the
tools column, refreshes the command registry (so `Window > Panels > Tools`
reflects the state), and saves the session.

### Floating and docking

`PanelFloat` already hosts the whole column as a frameless `Qt::Tool` top-level
on X11 and as an in-window child on Wayland. Because the grip and title row
drive the same `floatColumn` path, dragging out of the window needs no new
overlay code. `syncToContent` still hugs the tool content in both modes.

A floating Tools column resolves a re-dock target through
`PicturaMainWindow::newColumnSideAtInsideWorkspace()` rather than the x-only
`newColumnSideAt()`. The point must be inside the workspace rectangle and within
the same edge band, so dragging the panel on the desktop beside the window no
longer draws the blue indicator or snaps it back to the side it floated from.

The tools scroll host stays widget-resizable while floating. `syncToContent`
sizes the overlay to the full grid first, so the host refits the toolbox widget
to that viewport instead of leaving it at the previous 2-column width. Without
that, a 2→1 flip kept the narrow overlay but the wide toolbox, centering the
single tool column partly outside the overlay and clipping its right edge.

## Alternatives

- **Drop the scroll host entirely.** The content minimum would become the
  column's minimum height and force the main window taller, breaking the
  existing free-height contract.
- **Auto-switch to two columns when one does not fit.** Rejected by the
  reporter: the compact layout must stay a single column, and the panel clips
  when that layout cannot fit.
- **Draw the grip inside the toolbox body.** The grip must drag the whole
  column (including while floating) and must sit above the content in both
  modes, so it belongs to the column chrome, not the tool grid.
