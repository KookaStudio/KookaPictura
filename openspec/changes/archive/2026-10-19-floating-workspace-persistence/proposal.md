# Proposal

## Why

Issue #451 groups six defects in the floating-panel/workspace subsystem. The
largest one is data loss: workspace layouts and the session store do not record
which groups or columns are floating, or where their overlays sit, so a
workspace load or an application restart re-docks every float. The other five
are chrome and interaction regressions around the same subsystem: a flyout that
dismisses on any outside click, an unthemed float header, a collapsed initial
float height, an invisible-but-clickable group menu button, and no width-resize
affordance on an iconic flyout.

Fixing these together is worthwhile because they share the same overlay,
serialization, and panel-header code paths; splitting them would duplicate the
same tests and leave the persistence format half-defined.

## What Changes

- Persist floating groups and floating whole columns, including their screen
  geometry and their owner column, in workspace layouts and in the session
  store. Restore them floating on workspace load/switch and on application
  restart.
- Persist a torn-off Tools column's floating position and restore it across
  sessions. Applying a workspace still leaves the Tools column untouched.
- Turn the panel-icon flyout into a persistent, non-grabbing overlay: clicking
  the workspace or pressing Escape does not dismiss it. It closes only through
  its new chevrons-right close button or by clicking a panel icon in the same
  column (same icon toggles, another icon switches). Each column owns at most
  one flyout. The flyout hides itself when its hosted group leaves, its column
  is removed, or all panels are hidden, and it hides a floating group's own top
  bar while hosting it.
- Add the flyout close button immediately left of the per-widget context-menu
  button, using the existing Lucide `chevrons-right` asset. The button appears
  only while the group is hosted in the flyout.
- Give the floating group's top bar the shared panel-header background.
- Keep the two-thirds initial height when a docked column is torn off in iconic
  mode; snap to the icon-row size only when the rail mode is toggled while the
  overlay is already floating.
- Paint the per-widget context-menu button in a floating group by removing the
  button-level opacity effect that Qt cannot render under the overlay's own
  effect.
- Let the iconic flyout be width-resized from its canvas-facing edge, with the
  opposite edge anchored, symmetric to the docked column width grip.

**BREAKING**: the flyout is no longer a `Qt::Popup`; click-away dismissal is
replaced by explicit close controls. Existing stores load all floats docked
because they carry no floating geometry; group entries may already carry the
older `floating` marker, but a marker without geometry is treated as docked.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ui/panel-column`: floating group/column state and geometry, the persistent
  flyout and its close control, the float top-bar surface, the iconic tear-off
  height rule, and flyout width resizing.
- `ui/workspaces`: a workspace's remembered arrangement includes which groups
  and columns are floating and where their overlays sit.
- `document/workspace-persistence`: the session layout schema gains float flags
  and geometry for groups, widget columns, and the Tools column.

## Impact

- **C++ app**: workspace serialization/restore, session restore, panel column
  float ownership, `PanelGroup` header chrome, `PanelFlyout` hosting and resize
  grip, and the theme stylesheet.
- **Tests**: new Qt Test coverage for float round-trips, restart, flyout
  persistence/toggle/close, float-header painting, iconic tear-off height, and
  flyout width resizing.
- **No Rust engine change, no document-format change, no new dependency.**
- **Stores**: additive JSON fields; older stores load with every panel docked.
