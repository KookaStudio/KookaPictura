# Design

## Context

See `proposal.md` — Why. The current refresh chain and the four measured
duplicates are described there; this document records the choices that are not
forced by the specs.

Constraints from the existing code:

- `PictureViewRust` already uses manual `Default` (`state.rs:160`), and
  `DocumentSession` owns the selection (`document_session/session.rs:37`). The
  selection is written from roughly thirty points across the bridge modules,
  including history restore, crop/transform clears, content-aware move, and
  Refine Edge.
- `refresh()` is connected to `PictureView::changed` (`frame_canvas.cpp:112`) and
  is also called directly by frame lifecycle code. The 120 ms panel timer is
  shared by the `changed` and region paths (`frame.cpp:198-201`,
  `frame_canvas.cpp:122-144`).
- A notifying edit emits its region before `changed` (`edit.rs:100-118`,
  `178-193`); `end_paint` takes the stroke session before `edit()`
  (`impl_paint.rs:802-810`), so a stroke commit's region is presented while not
  painting and is followed by no `changed`.
- `retargetDock` currently re-runs `setView`/`setCanvas` on every debounced pass
  (`frame_session.cpp:30-105`). Several panels' `setView` already ends with a
  refresh, and `retargetDock` then calls `refresh()` again for History,
  Navigator, Info, and Histogram.

## Goals / Non-Goals

**Goals:**

- One contour computation per selection write, not per edit.
- One Layers rebuild per committed edit.
- One command-table recomputation per 120 ms of region preview activity, and
  still one synchronous recomputation per notifying edit.
- Ants ticks repaint only the outline region.
- No observable change to pixels, history, panel contents, or command state.

**Non-Goals:**

- Splitting `Edit::Meta` into selection and metadata variants. The issue names it
  as a follow-up; this change keeps the transaction enum intact.
- Per-panel change scoping (only refreshing the panels a particular edit
  affects). The pass still refreshes the same panel set as today, just without
  rebinding and without the duplicates.
- Hiding or skipping panel refresh when a panel widget is not visible.
- GPU or compositing changes.

## Decisions

### Cache the encoded contour in Rust, keyed by a selection revision

`DocumentSession` gains `selection_revision: u64` and two helpers:
`set_selection(Option<Selection>)` and `take_selection() -> Option<Selection>`,
both bumping the revision. Every direct `session.selection` write is converted;
the in-place writes that finish with a non-`Meta` edit (`healing.rs:275`,
`impl_selection.rs:186/238`, `refine.rs:608`) get an explicit
`touch_selection()`. `DocumentSession::restore` bumps once after the history step
returns, because it hands the selection to the step as `&mut`. History restore
counts as a selection write: an undo or redo retraces the contour even when the
restored mask equals the current one; the extra trace is accepted so the setter
audit stays mechanical.

`PictureViewRust` keeps:

```
selection_contour_cache: RefCell<Option<(u64, Vec<i32>)>>
selection_contour_computes: Cell<u64>
```

`selection_contour_extents` clears the cache when the selection is absent, returns
the cached encoding on a revision match, and otherwise runs
`pictura_select::contour` and stores the encoding. The count is exposed through a
bridge getter for the acceptance test.

Alternative considered: cache only in C++. It would avoid the bridge copy and
the `QPolygonF` rebuild, but it would move the cache away from the only expensive
operation and create a second identity mechanism. The chosen split lets the
bridge API itself stay cheap; the remaining FFI copy is small next to the contour
scan and is a deliberate ceiling.

Alternative considered: bump the revision only in the transaction for
`Meta`/`SelectionPreview`. That would miss content-aware move, Refine Edge, move
selection content, and history restore, and would retrace after unrelated
metadata edits. The explicit setter is worth the mechanical diff.

### The canvas ignores a contour revision it already shows

`selection_api::SelectionContour` carries the view's `selection_revision`.
`ImageView::setSelectionContour` returns early when that revision equals the last
one it applied, so `refreshSelectionOverlay` on a notifying edit neither repaints
the canvas nor restarts the ants timer. `clearSelectionContour` resets the stored
revision so a later contour is always applied.

This is a presentation guard, not a second contour cache: the encoded extents
are cached, and the loop list is stored and repainted only when the revision
advances. The encoding is still converted to loops on a cache hit (the accepted
ceiling). A canvas created later has a default revision and therefore receives
the contour on its first refresh.

### Rebind on view, canvas, or generation; refresh otherwise

`PicturaMainWindow` remembers the last bound `PictureView*`, `ImageView*`, and
`document_generation`. `refreshPanels` rebinds through `retargetDock` only when
one of the three differs; otherwise it calls a new `refreshDock` that invokes each
panel's `refresh()` without `setView`/`setCanvas`.

The bind path calls each panel's `setView` once and drops the explicit
`refresh()` calls that follow `setView` for History, Navigator, Info, Character,
Paragraph, ParagraphStyles, CharacterStyles, Paths, and Histogram. The
`aDocumentSwitchRebindsTheDock` case asserts one Layers refresh and a correct
Adjustments pattern button on a switch. A bind marks `panelsFreshlyBound_`; the
next debounced pass consumes that marker instead of refreshing the dock again,
so the trailing `refresh()` in add/remove paths cannot double-refresh. A
`changed` or active region clears the marker.

A document replacement can change `document_generation` without emitting
`changed` (for example `new_document`); the frame refreshes and runs the panel
pass from `sessionCancelled` when the replaced view is active, so a generation
change is observed.

When a filter commit is running on the new view, the bind is deferred until that
commit lands; the in-place refresh is skipped in the meantime.

`removeDocument` rebinds only when the removed view is the currently bound or
active view; closing an inactive document does not rebind.

The refresh-only path also refreshes pattern-dependent controls: the
`patternFill_`/pattern-button state sync moves out of `setView` into refresh
methods (`LayersPanel::refresh` and a new `AdjustmentsPanel::refresh`). Both
cache the view pointer and document generation, so the pattern-resource scan
runs only when the document surface changes; a same-view edit does not decode
the patterns again. The pattern resources belong to the document, so the switch
case covers the control; a same-view edit does not change them.

The active canvas is part of the key because `canvasAt()` follows
`activeWindow_` (`frame.cpp:348-361`): two windows of one document share a view and
generation but need different Navigator canvases.

`registerTypeFallbacks()` currently rides on `retargetDock`. It moves to (or is
also called from) the unconditional part of `refreshPanels`, otherwise the first
timer at startup would never register the bundled/fallback fonts.

### `changed` and region presentations carry two flags into the debounced pass

- `panelsRefreshedByChanged_` is set by a second connection to
  `PictureView::changed` (`wireDocument`) only when the emitting view is the
  active view (or the flag tracks the emitting view), and is cleared by an
  active-view `regionDamaged`/`regionBlitted` presentation and by an
  active-surface change.
- `refreshDock` skips Layers and History when that flag is set, because both
  panels subscribe to `changed` and have already refreshed themselves (Layers
  through its queued `singleShot(0)`, History through `history_api`).
- When the flag is clear — a stroke commit, or a region preview tick — the pass
  includes Layers and History, so the thumbnail and the new undo state are
  current. The panel pass is skipped while `is_painting()`; an in-stroke region
  only arms the deferred present, so a long stroke does not rebuild Layers
  every interval.
- A paint commit presents the pending region when there is one; if every dab was
  already presented, it presents the stroke's union instead, so the commit still
  produces the region signal that arms the panel pass.
- `registryRefreshPending_` is set by `afterPresent` after the in-stroke guard and
  consumed by the same pass. `refresh()` clears it after its synchronous
  recomputation, so a notifying edit that also presented a region does not pay a
  second recomputation 120 ms later.

`Edit::SelectionPreview` notifications (Quick Selection) keep the synchronous
registry refresh and are out of scope: the issue targets the region-blit path,
and narrowing the coalescing keeps this change small. A Layers panel refresh
during the pass can emit `selectionChanged`; the frame folds that registry
refresh into the same pass instead of recomputing inside `refreshDock`. A
Layers self-refresh caused by a notifying edit reuses the edit's synchronous
recomputation.

Alternative considered: a dedicated 120 ms registry timer. Two timers would add
another member and could fire apart from the panel pass; the existing single-shot
already gives the "at most once per interval" coalescing and is restarted by every
presentation.

Alternative considered: have Layers own a "refreshed since the timer armed"
counter. The frame-level flag is simpler and expresses the actual rule: a
`changed` already drove the self-subscribing panels.

### The ants tick updates the outline's widget rect

`ImageView` computes the union of the committed contour (only when
`selectionEdgesVisible_`), the live preview loops, and
`moveOutline_.translated(moveDelta_)` in image coordinates, adds the
move-selection contour offset, maps the result through
`viewRotation()/zoom_/offset_`, inflates by two device pixels for the cosmetic
pen, intersects it with the widget rect, and calls `update(rect)`. Empty or
off-screen bounds produce an empty update, which is a no-op. Unknown/empty
bounds fall back to a full update so no animated overlay is left frozen.

`paintEvent` stores `event->rect()` in a test-visible member; the timer already
stops on `hideEvent` and restarts on `showEvent`, so that part of the issue needs
no change.

### Test hooks are explicit counters

The acceptance needs counts that are independent of `PICTURA_METRICS` being set
at process start:

- `PictureView::selection_contour_compute_count()`
- `LayersPanel::refreshCountForTest()`
- `LayersPanel::patternActionEnabledForTest()`
- `PicturaMainWindow::dockRetargetCountForTest()`
- `ImageView::lastPaintRectForTest()` / `selectionContourSetCountForTest()` /
  `antsTicksForTest()`
- `HistogramPanel` gains a bin reader for the stroke-refresh assertion;
  Navigator and Channels already expose their thumbnail images.
  `AdjustmentsPanel::buttonByIdForTest()` already exposes the pattern button.

Alternative considered: read the `layers_panel_refresh` phase from
`metrics_snapshot`. It requires enabling metrics before the first C++ metric
timer is constructed and parsing JSON in the test; the counter is smaller and
tests the refresh itself rather than the metrics plumbing.

## Risks / Trade-offs

- **A selection write missed by the setter audit** leaves a stale contour.
  → Replace every write found by the audit, cover draw/deselect/move/refine/
  history in tests, and keep the cache key optional so a default revision cannot
  false-hit. History restore is a write, so an undo may retrace even when the
  mask is unchanged; that is accepted.
- **The `changed`/region flag races a burst.** A region after a `changed` clears
  the flag, so a stroke that follows a selection change still refreshes Layers;
  a `changed` after a region sets it, so a normal commit is not refreshed twice.
  → Both orders are covered by the committed-edit and stroke-commit tests.
- **Rebinding is skipped when only the active window changes** and the canvas is
  not in the key. → The key includes the canvas; a multi-window switch test
  covers it.
- **The timer pass runs while a filter commit owns the history.** Both flags stay
  pending; the commit's own refresh restarts the timer, as today.
- **Ants partial update misses a drawn overlay.** → Include every animated loop
  list and fall back to a full update when bounds are empty.
- **`registerTypeFallbacks` never runs at startup** if it stays only in the bind
  path. → Call it from the unconditional part of `refreshPanels`.
- The contour still crosses the bridge and rebuilds `QPolygonF` loops on a cache
  hit. → Accepted; the scan is the dominant cost. A frame-level loop cache is a
  later change if metrics show the conversion matters.

## Migration Plan

No data or file-format migration. The revision and caches are view state; they
are not saved, so opening an existing document starts with revision zero and an
empty cache. Rollback is a normal revert — no persisted state changes.
