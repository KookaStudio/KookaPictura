# Design

## Context

See `proposal.md` for motivation. The implementation already has two overlay
kinds: `PanelFloat` hosts either a whole `PanelColumn` or a single `PanelGroup`,
and `PanelColumn::floats_` owns the group overlays. `PanelColumn::columnFloat_`
owns the whole-column overlay. The same `serializeWorkspaceLayout()` shape feeds
both the session store and the named-workspace store, and
`regroupAndApply()` is the only layout path that can re-group panels.

Three current constraints shape the work:

- `serializeWorkspaceLayout()` folds or skips a floating column before the tools
  branch (`frame_workspace_controller.cpp:51-58`), so a floating Tools column is
  currently dropped.
- `savePanelState()` already writes `"floating": true` for group floats but no
  geometry; a store may therefore carry the marker without a restorable rect.
- The session restore path does more than adopt groups by name: it places the
  Tools column, seeds widths and rail modes, applies
  `autoCollapseIconic`/`autoShowHidden`, and schedules deferred widths
  (`frame_session.cpp:294-422`). The workspace path does not.

The flyout is currently a `Qt::Popup`, and its hosted group can come from a
`PanelFloat`, where `setFloating(true)` has already shown the float top bar.
`PanelGroup`'s corner menu button also carries its own
`QGraphicsOpacityEffect`; inside `PanelFloat`, which has an overlay-wide effect,
Qt cannot paint the nested child effect, so the button is laid out and
hit-testable but invisible.

## Goals / Non-Goals

**Goals:**

- One layout-restore path that can reproduce docked columns and both float kinds.
- A backwards-compatible, additive JSON shape for session and workspace stores.
- A persistent flyout with one header, explicit close/toggle controls, correct
  child-mode placement, and no stale overlays.
- Preserve focus, History-panel shortcuts, hide-all, auto-collapse flags, and
  the existing drag grammar.

**Non-Goals:**

- Persisting whether a flyout is open; flyouts remain transient UI.
- Frame-global flyout exclusivity; the contract is one flyout per column.
- Changing the Rust engine, document formats, or the existing drag/drop rules.
- Adding a new icon asset; the requested chevrons-right glyph already exists as
  `panel.columnsTwo`.

## Decisions

### D1. Extend the existing per-column layout instead of a second float store

Add two additive fields to the layout shape, keeping it shared by the session
and workspace stores:

- a column entry may carry `"floating": true` and
  `"floatGeometry": [x, y, w, h]`;
- a group entry may carry `"floatGeometry": [x, y, w, h]` alongside the existing
  `"floating": true` flag.

A float is restored only when both the marker and a valid geometry are present;
a marker without geometry (the shape written by the current implementation)
loads docked. `floatGeometry` is the overlay's global top-left plus size.
Restored rectangles are clamped to the available screen or frame and to the
overlay's minimum size; position-only clamping is not sufficient.

*Alternative considered:* a top-level `floats` array. It would duplicate the
panel ownership data already in `savePanelState()` and would need a second
lookup from group to column on restore. The additive form reuses the existing
serializer and round-trip rules.

### D2. One restore path: build docked, then detach floats

`regroupAndApply()` gains a `persist=true` parameter and remains the single
layout builder. It saves the previous `restoringPanelSession_` value and
restores it on exit; it calls `saveSession()` only when `persist` is true and the
guard was not already set. After it has built and styled every group and applied
the per-column width/rail mode, it:

1. harvests group floats first, then whole-column overlays, so a group float
   owned by a floating column is never left pointing at a column that is about
   to be deleted;
2. detaches groups whose spec carries `floating` and a valid `floatGeometry`
   into `PanelFloat`s owned by their built column;
3. detaches whole columns whose entry carries `floating` and a valid
   `floatGeometry` into column overlays.

`buildInto()` records a per-entry `PanelColumn*` map so the whole-column detach
step knows which built column belongs to which layout entry. Groups are built
docked first so the existing panel pool, tab order, visibility, minimized, and
collapsed logic is reused. Missing or invalid geometry is treated as "not
floating" so a partially written or pre-change store stays docked.

`applyPanelSession()` stops adopting groups by name and calls
`regroupAndApply(..., persist=false)` for non-empty session layouts. It keeps
the legacy empty-layout path (`panelGroups` / no-op reset) unchanged. Before
calling, it normalizes old entries that lack `width`/`railMode`, seeding the
first right-hand column from the legacy `railWidth` and every entry from
`panelRailMode`. After the rebuild it re-applies
`autoCollapseIconic`/`autoShowHidden` to every docked and floating column,
places the Tools column, and restores its floating overlay when recorded.

*Alternative considered:* extend the session-only adoption path. It would need
a second group-builder and still fail when two groups share a name, which is
exactly the tear-off case this change must support.

### D3. Preserve the last docked side for floating columns

The serializer currently asks the frame for a column's side, which falls back to
"right" once the column leaves the splitter. Use `PanelColumn::side()` instead;
it already remembers `lastSide_`. The existing `order` field remains the
tie-breaker for rebuilding docked order; a floating column's own entry no longer
folds into the primary column.

### D4. Tools column floats ride the same session path

Handle `isToolsColumn()` before the floating-fold/skip branch so a floating
Tools column always emits its `{side, order, tools: true, groups: []}` entry
plus `floating`/`floatGeometry`. Session restore re-inserts the tools column at
its recorded side and, when the entry carries a floating marker, calls the same
column-overlay restore. Tools overlays remain non-resizable and content-sized,
so only their position is restored. `regroupAndApply()` continues to skip tools
entries, which preserves the workspace contract that applying a workspace leaves
Tools unchanged.

### D5. Flyout host: focusable frameless overlay, not `Qt::Popup`

`PanelFlyout` uses the same platform-adaptive hosting as `PanelFloat`: a
frameless `Qt::Tool` top-level parented to the frame where client positioning is
available, and an in-window child of the frame otherwise. It stays focusable
(no `WindowDoesNotAcceptFocus`) and is explicitly activated after `show()` so
the History panel can keep keyboard focus and the existing Ctrl-Z routing test
continues to pass. Because it is no longer a popup, outside presses and Escape
do not hide it.

`placeFlyout()` must branch on the host mode. Top-level mode uses global
coordinates. Child mode parents the flyout to the frame, converts with
`mapFromGlobal`, clamps to the frame rect, and uses the same coordinate space for
the width grip. The flyout is laid out using its actual size after `resize()`,
not the requested size, so the inner edge still meets the icon button.

The flyout is hidden when its hosted group is dragged out, its last visible
panel/group is closed, its owning column is removed, or all panels are hidden.
`regroupAndApply()` closes every live flyout before harvesting groups. The
`flyout_` object is owned by its `PanelColumn` but parented to the frame, so
`PanelColumn::~PanelColumn()` deletes it explicitly after disconnecting
`onHidden`.

*Alternative considered:* keep `Qt::Popup` and reopen after the automatic hide.
That fights Qt's popup grab model and would still swallow the outside press;
vetoed.

### D6. Flyout close control in the group header corner

`PanelGroup` gains a flyout-only close button in `headerCorner_`, inserted
between the drag grip and the per-widget menu button. It reuses
`panel.columnsTwo` (Lucide chevrons-right), uses a matching icon size, tooltip,
and auto-raise, emits `flyoutCloseRequested`, and `PanelColumn::wireGroup()`
routes that to `closeIconFlyout()`. A new `setFlyout(bool)` toggles its
visibility; `openIconFlyout()` calls `setFlyout(true)` and every
`restoreFlyoutGroup()` exit, drag rehome, and `insertGroupAt()`/
`PanelFloat::setGroup()` path calls `setFlyout(false)` so the control cannot
leak into docked or floating presentations. `headerCornerWidthForTest()` is
updated to include it.

The icon-toggle rule lives in `openIconFlyout()`: if the same icon's flyout is
visible, close it; otherwise close any current flyout and open the new one. A
column has one `PanelFlyout`, so per-column exclusivity is structural.

When a group from a floating icon row enters the flyout, `setFloating(false)` is
called so the float top bar and its close control disappear; returning it to the
origin float restores `setFloating(true)`. A flyout therefore always has exactly
one header and one close path.

### D7. Flyout width grip reuses the float grip pattern

Generalize the existing private `PanelResizeGrip` to take a target widget and an
edge mode, or add a sibling class in the same translation unit. The flyout uses
a 4 px edge grip on its canvas-facing side: left edge for a right-hand column or
floating row, right edge for a left-hand one. The drag adjusts only the width
and moves the left edge when the grip is on the left, so the opposite edge stays
anchored. The minimum width is the shared normal-mode floor (300 px), applied
before the first `resize()` so the flyout also opens at or above it. In child
mode the new rect is clamped to the frame.

*Alternative considered:* a stock `QSizeGrip`. It targets the top-level window
and cannot anchor the opposite edge, which is the behaviour the issue asks for.

### D8. Keep the icon-row body edge resizing unchanged

The flyout is only width-resized; it has no height grip and keeps its
content-driven height. The docked column's workspace-edge grip and the floating
overlay's corner grip are untouched.

### D9. Chrome fixes are local, not architectural

- Remove the per-button `QGraphicsOpacityEffect` from the corner menu button.
  The float overlay's single effect then paints the button normally; no new
  rendering path is needed.
- Add `QWidget#panelFloatHeader` to the theme with the same
  `${surface-header}` background as the docked column header, and set
  `WA_StyledBackground` on the float top bar.
- Skip `syncToContent()` for an already-iconic widget column at tear-off time.
  `setRailMode()` still calls it when the rail is toggled while floating, which
  preserves the snap behaviour for that transition.

### D10. Startup restore is ordered and does not persist intermediate state

Overlay geometry is restored after the window geometry is applied, or is
re-clamped when the frame is resized, so child-mode overlays are not clamped
against the default frame. `regroupAndApply()` and the session path suppress
`saveSession()` until the first layout when they are running as part of startup
restore; `persistedWidth()` is not allowed to overwrite a stored width with a
zero before the splitter has been laid out.

## Risks / Trade-offs

- **Nested graphics effects recur** -> keep one effect per overlay; the
  regression test grabs the whole float and samples the menu button, so grabbing
  the button alone cannot hide the failure.
- **Stale flyout after group movement** -> close it on drag-out, last-panel
  close, column removal, and hide-all; add lifecycle tests.
- **Old workspace files lose float state** -> accepted: geometry is new, and a
  store without geometry opens all-docked. Panels are not lost.
- **Duplicate group names after tear-off** -> restore uses the built group
  pointer from the layout spec, not a name lookup, so duplicate names are safe.
- **Floating primary identity** -> the serializer does not add a primary marker;
  the first right-hand entry still becomes the primary column. The visible
  arrangement and panel contents are unaffected.
- **Flyout focus on platforms that ignore activation** -> top-level mode calls
  `activateWindow()`; child mode keeps focus inside the frame. The existing
  History shortcut test is the canary. Existing tests that walk from the column
  to `findChild("panelIconFlyout")` need a `PanelColumn` accessor instead of a
  parent-chain assumption.
- **Screen/monitor changes** -> restored rects are clamped in both position and
  size (available screen or frame, then overlay minimum), so a saved position on
  a removed monitor remains reachable.
- **Startup ordering** -> overlay restore and the first layout are explicit
  phases; no intermediate save is allowed.
- **Tools workspace semantics** -> tools floating state is restored only by the
  session path; workspace apply remains a no-op for Tools by design.

## Migration Plan

1. Write session schema version 10 with the additive float fields.
2. Load older stores normally: no geometry means no overlays; version-9 and
   older stores keep width, rail mode, group state, and auto flags, and load
   every panel docked.
3. Workspace files need no index migration; the layout is opaque and a missing
   geometry field reads as docked.
4. Rollback is safe: an older build ignores the new fields and opens the layout
   docked; no files are deleted or rewritten destructively.

## Open Questions

None.
