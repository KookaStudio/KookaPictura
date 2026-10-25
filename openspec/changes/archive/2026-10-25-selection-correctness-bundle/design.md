# Design

## Context

`SelectionOptions` is a single family group shared by every selection tool (`tools.cpp`), and `ToolController::resolveSelectionMode` reads that one group. Quick Selection mutates it from `applyDab` to make its own New-to-Add switch, so the mutation is visible to the marquee. The options bar binds every selection page to the Marquee group at construction time and never re-reads the mode.

Quick Selection's preview path composites the document, floods the full image, combines the full selection, and retraces the outline for every pointer move (`impl_selection/quick.rs`, `tool_wand.cpp`).

The committed contour is traced from the document-sized mask and correctly includes canvas-border edges. The paint path then draws it inside `setClipRect(docClip)` with a cosmetic 1-px pen; at fractional pan/zoom the rasteriser can place the stroke outside the clip, so the border segment disappears on some frames.

Selection tool handlers derive `ToolHandler` but implement neither `hasCanvasContextMenu` nor `buildContextMenu`; the router returns false over the document and the right-click is a no-op there. Handlers have no way to invoke a registered command.

Refine Edge's large-document crash (#915) is out of scope: no bounded repro was obtained, so its requirement, delta, and verification moved to the follow-up.

## Goals / Non-Goals

**Goals:**

- Quick Selection and the marquee keep independent combine modes, with the options bar and persisted options following the active tool.
- An intra-region Quick Selection move is a no-op that does no full-image work.
- The committed outline at the canvas border is stable at fractional pan/zoom while the contour stays the traced boundary.
- Selection tools reuse the existing Select commands through a canvas menu, gated on an active selection.

**Non-Goals:**

- Transform Selection (#526), interactive Refine Edge brushes (#612), engine-wide antialiasing or channel limits (#614), and selection save/load (#615).
- Separate option groups for Magic Wand; it stays on the shared selection group.
- Refine Edge's large-document crash (#915); it is tracked in the follow-up.
- A general-purpose command bus for all tools.

## Decisions

### #913: Quick Selection owns its option group

Add one `SelectionOptions` instance for Quick Selection in the `ToolController` constructor and register it with the Quick Selection handler. Change `resolveSelectionMode` to resolve `optionsFor<SelectionOptions>(active_)` and fall back to the shared group when the active handler has none (Type, content-aware move, patch keep their current behavior).

Resolve the options-bar group per page with `optionsFor<SelectionOptions>(id)` instead of the constructor-cached `selection_`, and pass the group into `addModeButtons`. Every field on the page uses that group, including Quick Selection's Tolerance spin. Pages whose handler has no `SelectionOptions` (Patch, Content-Aware Move) fall back to the shared group, preserving their combine buttons. Persist the Quick Selection group under a new `quickSelection` key using the existing `SelectionFields` helpers; an older session simply has no key and starts from defaults.

Add a `combineModeChanged` callback to `SelectionOptions::setCombineMode` that fires only when the mode changes, mirroring `magneticWidthChanged`, and have the Quick Selection page's mode buttons re-read the group. The OptionsBar sets the callback on the Quick Selection page's group only and clears it when the page is destroyed (or captures the buttons with `QPointer`), so a group that outlives the bar cannot call into freed widgets. The shared group's pages keep their current refresh behavior. Without the callback the Quick Selection page keeps showing New after the first dab while the tool behaves as Add.

Alternatives considered: deleting the mutation breaks the `qs_new_to_add` requirement; restoring the mode on tool switch does not let Quick Selection read Add between strokes; a per-tool mode map inside the shared group keeps the same hidden coupling and adds state.

### #914: skip dabs that are already selected

Extend `quick_select_preview` and its bridge (`impl_selection/quick.rs`, `impl_selection/bridge.rs`) with an `is_move` flag; the C++
handler passes `ctx.dragCommitted()`, so the first press always floods. On a move
in Add mode whose pixel is inside the document and already has coverage 255 in
the current selection, return true before `quick_select_shape` calls
`current_buffer` or `magic_wand`, and do not call `edit`. No
`PictureView::changed` signal is emitted. New, Subtract, Intersect, and the first
press still flood, and partial coverage still floods so a soft edge can grow.

This is the cheapest semantics-preserving lever: the flood from a pixel inside the selected region normally adds nothing, and most move events of a stroke stay inside the region. It also removes the per-move composite, full-selection combine, and contour retrace. Mark the accepted ceiling with a `ponytail:` comment: an Add-mode move inside a pre-existing selection is skipped too, so a seed whose local colour differs can no longer contribute a few extra pixels; the stroke semantics do not require that growth.

Measure the press path and an intra-region move with an ignored Rust profiler following `tests_profiles.rs` (4000×4000 composite): report composite, flood, combine, and contour times separately, plus the first-dab and intra-region-move totals. Proposed targets: first dab ≤ 250 ms, intra-region move ≤ 16 ms, new-region move ≤ 250 ms on the development machine. If the profile shows compositing dominates and a target misses, record the measurement and open a follow-up for a cached stroke composite; do not add that cache in this change.

The regression check is deterministic, not wall-clock: count `PictureView::changed` emissions during the move-only part of a drag that stays inside one region and assert the count does not grow with the number of move samples.

### #912: one device pixel of drawing slack at the clip edge

Keep `canvasDoc` and the content clip unchanged. For the block that draws the committed ants and the live preview (they share one `save`/`setClipRect`), expand the painter clip by `1 / zoom_` image units on each side, so the cosmetic stroke can straddle the document boundary. The contour and its vertices are untouched. Only the committed-outline requirement is claimed and tested; the live preview may share the one-pixel slack.

Alternatives considered: omitting border segments in the contour violates the outline requirement; insetting the drawn contour by half a pixel changes the traced boundary; antialiasing the pen does not recover the clipped outer half and softens the ants.

Test by rendering the canvas at several fractional pan offsets and zoom levels with the border inside the viewport, then scanning each of the four border runs for a dark dash pixel. The document and the canvas background are both white in the fixture, so a dark pixel near an edge can only be the black dash; the white base is invisible and the pasteboard cannot be mistaken for ink. Before and after each grab, compare `ImageView::selectionContours()` (the stored loops the painter draws) with `selection_api::contour(view).loops`; they must match and stay unchanged. The check is presence at every sample, not exact dash phase.

### #884: command dispatch plus a router gate

Add `ToolServices::requestCommand(const QString&)`, implemented by `ToolController` as a `commandRequested` signal, and connect it in the frame (`frame_build.cpp`) to `CommandRegistry::dispatch`. `ToolController` is the only `ToolServices` implementer, so the pure virtual has one override. This reuses the registered `SelectDeselect`, `SelectInverse`, `SelectModifyFeather`, and the Refine Edge command (`commandIdForPath({Select, Refine Edge…})`), including their dialogs and history behavior.

Put one `selectionCanvasMenu(ToolServices&, const QPointF&)` helper in `tool_selection.cpp` and declare it in `tools.h`; the six handler classes (covering all seven tool ids: rectangular and elliptical marquee share one handler, then lasso, polygonal lasso, magnetic lasso, Magic Wand, Quick Selection) call it from `buildContextMenu` and return true from `hasCanvasContextMenu`. The menu labels are `Deselect`, `Inverse`, `Feather…`, and `Refine Edge…`; the helper resolves the Refine Edge id with `commandIdForPath` and connects every action to `requestCommand`.

Gate availability in `ToolController::canvasMenuHandler` after the clicked document is activated: for the seven selection tool ids, require the active view to have a non-empty selection. This uses a new `isSelectionMenuTool` predicate; `isSelectionTool` is not extended because it also drives selection-move drags. `hasCanvasContextMenu` returns true unconditionally so the clicked document is activated before the gate reads its selection. With no selection the gate makes `contextMenuForTest` return null and `routeCanvasContextMenu` return false; over the document that shows no menu, and on the pasteboard the frame's background fallback still opens. The gate is selection-only, so with a selection the menu also opens over the pasteboard, consistent with the general tool-menu precedence.

Transform Selection is omitted until #526. The menu has four entries and no disabled row.

### #915: split out

No bounded repro or backtrace was obtained. The split path in the #915 issue applies: remove `tools/refine-edge` from this change and move the engine work, the spec delta, and the repro to the follow-up, so this archive does not claim an unverified behavior.

## Risks / Trade-offs

- **#913 option-group split changes the persisted shape** → write and read the optional `quickSelection` key; old sessions fall back to defaults. Add a persistence test.
- **#914 the skip changes the exact flood union in rare tolerance-transitivity cases and also skips an Add-mode move inside a pre-existing selection** → accepted for the latency win; New, Subtract, Intersect, and the first press still flood, and partial-coverage pixels still flood.
- **#912 ants bleed one device pixel onto the pasteboard** → the stroke is cosmetic 1 px; document content stays clipped.
- **#884 a right-click on a non-active document activates it even when the gate then finds no selection** → same activation behavior as the existing router; the test asserts the menu behavior, not the tab switch.
- **#915 is deferred and remains unverified at document scale** → tracked in #915; this change makes no spec claim.
- **`tools.cpp` and `options_bar.cpp` are close to the 1200-line cap** (1199 and 1185 after this change) → the menu predicate lives in `tools_selection_move.cpp` and the options-bar change stays under the cap; split along a seam before adding more.
- **Render/perf tests can be flaky** → render only presence at fixed offsets; measure latency in an ignored profiler and keep the deterministic check as the gate.

## Migration Plan

No data migration. The session JSON gains an optional `quickSelection` object; older builds ignore it and new builds default when it is absent. Rollback is a revert of the change. No document-format or public API change.
