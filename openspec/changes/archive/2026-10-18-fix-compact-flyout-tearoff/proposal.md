# Proposal

## Why

Dragging a panel group out of the compact-strip flyout leaves the flyout's
empty `Qt::Popup` shell visible as a grey rectangle over the workspace (#732).
The flyout hosts the real `PanelGroup`; a group drag rehomes that group into a
floating overlay, but nothing hides the popup, so it stays on screen with no
content. The screenshot in the issue shows the History/Adjustments float beside
the empty rectangle, anchored exactly where the flyout opened.

## What Changes

- Close the compact-strip flyout when a group drag tears its hosted group out of
  the popup, before the overlay follows the cursor.
- Keep the flyout open when a single tab is dragged out and the group still has
  visible panels; the popup then shows the remaining tabs.
- Close the flyout when a tab drag leaves its group with no visible panels.
- Add Qt Test cases for the group tear-off, the tab tear-off that keeps the
  popup, and the tab tear-off that empties it.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ui/panel-column`: adds a requirement for the drag handoff out of the compact
  flyout.

## Impact

- `crates/pictura-app/cpp/panels/panel_column_drag.cpp` — group drags close the
  flyout after capturing the grab offset, so the float appears where the group
  was.
- `crates/pictura-app/cpp/panels/panel_column_float.cpp` — a tab drag that
  empties the flyout closes it.
- `crates/pictura-app/cpp/tests/tst_panels_parity.cpp` — three Qt Test cases.
- No bridge, API, dependency, or data-format changes.
- No `design.md`: the fix is a small state handoff between two existing drag
  paths, with no cross-cutting or migration decision.
