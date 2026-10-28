# Proposal

## Why

Issue #974. Every notifying edit currently repeats work that cannot have changed:
the selection contour is retraced from the full mask, the Layers panel rebuilds
twice, every docked panel is re-bound through `setView`, and region preview ticks
recompute the command table on every tick. On a 6000×4000 document a noisy
selection contour costs an estimated 100–300 ms per edit, and the docked panels
add tens of milliseconds plus a second full Layers rebuild.

This is the refresh path, not a new feature. The canvas already carries a
revision for its present cache and the Layers panel already coalesces its own
queued refresh; the selection outline has no revision-keyed cache, and the
docked panels have no bind/refresh split.

## What Changes

- **Selection contour cache.** The document session gains a selection revision,
  bumped whenever the selection is written. The outline is cached by that
  revision, so a paint stroke, filter, or metadata edit that does not write the
  selection does not retrace it. The canvas also ignores a contour whose revision
  it already shows, so the edit does not trigger a full canvas repaint for the
  ants. History restore counts as a selection write, so an undo may retrace; that
  is accepted.
- **Dock rebind split.** `retargetDock` (the `setView`/`setCanvas` bind) runs only
  when the active view, active canvas, or document generation changes. On the
  same view, the debounced pass refreshes panels in place instead of re-binding
  all of them. Panels that already refresh themselves from `changed` (Layers,
  History) are not refreshed a second time by that pass. The `changed` marker is
  set only when the emitting view is the active view (or it tracks the emitting
  view) and is cleared on a surface change, so a background document cannot
  suppress the active panels.
- **Stroke-commit refresh.** A paint stroke commits through the region path with
  no `changed` notification, so the debounced pass MUST still refresh Layers and
  History after it; otherwise thumbnails and the undo list go stale. An
  active-view region presentation clears the "self-refreshing panels already
  ran" marker.
- **Command registry coalescing.** A region presentation requests a command-table
  refresh through the same 120 ms pass instead of recomputing synchronously on
  every region tick. A notifying committed edit still recomputes once from its
  own notification, synchronously. `Edit::SelectionPreview` notifications (Quick
  Selection) keep their synchronous refresh and are out of scope.
- **Marching ants partial repaint.** The ants timer repaints only the widget
  region covered by the animated outline(s), not the whole canvas unless the
  outline itself spans it.
- No new `Edit` variant, no new dependency, and no change to what is composited,
  saved, or undone. The hidden-canvas ants behaviour already stops the timer on
  `hideEvent`; that part needs no change.

## Capabilities

### New Capabilities

- `ui/panel-refresh-lifecycle`: when docked panels are re-bound versus refreshed
  in place, which panels the debounced pass refreshes after a notifying edit and
  after a region-only stroke commit, and how a mid-stroke present feeds the pass.

### Modified Capabilities

- `tools/selection-outline-performance`: the committed outline is cached by a
  selection revision and is not retraced by an edit that does not write the
  selection; viewing an unchanged contour does not repaint the canvas; the ants
  animation repaints only its outline region.
- `document/command-registry`: command-table recomputation caused by region
  presentations is coalesced to at most once per 120 ms, while a notifying
  committed edit keeps its exactly-once synchronous recomputation. Quick
  Selection's `SelectionPreview` notifications stay synchronous and are out of
  scope.

## Impact

- `crates/pictura-app/src/cxxqt_object/document_session/session.rs`: selection
  revision and the selection write helpers.
- `crates/pictura-app/src/cxxqt_object/state.rs`, `impl_selection.rs`,
  `impl_selection/quick.rs`, `impl_selection/refine.rs`, `selection_move.rs`,
  `healing.rs`, `impl_history.rs`, `clipboard.rs`, `crop_group.rs`,
  `hdr_conversion.rs`, `image_adjust/*`, `impl_transform/*`,
  `filter_tools/lens_blur_depths.rs`: every selection write routes through the
  helper; the contour cache and its compute counter live on the view state.
- `crates/pictura-app/src/cxxqt_object.rs`: bridge getters for the selection
  revision and the contour compute count.
- `crates/pictura-app/cpp/api/selection_api.*`: the typed contour carries its
  revision.
- `crates/pictura-app/cpp/image_view.*`: revision guard on
  `setSelectionContour`, partial-update rect for the ants tick, paint/timer test
  hooks.
- `crates/pictura-app/cpp/frame.h`, `frame.cpp`, `frame_canvas.cpp`,
  `frame_session.cpp`: bind/refresh split, the debounced flags, the registry
  coalescing, and `retargetDock`/refresh counters.
- `crates/pictura-app/cpp/panels/layers_panel.*`,
  `panels/adjustments_panel.*`, `panels/histogram_panel.*`: refresh, pattern
  action, and bin test hooks; the pattern-dependent control sync moves into the
  refresh path.
- `crates/pictura-app/cpp/selftest_paint_perf.cpp`: the stroke-commit registry
  check now waits for the debounced pass.
- Tests: `tst_edit_refresh` grows the contour, dock, layer-refresh, stroke,
  inactive-close, and region-debounce cases; `tst_selection_preview` (or
  `tst_selection_api`) gains the ants repaint region case; a Rust unit test
  covers the contour cache.
- No `docs/` or `ROADMAP.md` change. The historical performance notes describe
  the superseded behaviour and are intentionally left alone.
