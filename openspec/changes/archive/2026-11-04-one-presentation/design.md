# Design

## Context

See `proposal.md` for motivation. The current tree has several report shapes and presenters:

| Producer | Report | Presenter | Pixel application | Rebuild |
|---|---|---|---|---|
| `apply_edit` | `Applied` + `Effect` | `PictureView::edit` | producer | `composite_full` |
| `cancel_active_sessions` | `SessionChange` | `apply_session_change` | presenter | `recomposite` |
| `session.restore` | `Restored` | `show_restored` | presenter | `reset_pyramid_deferred` |
| `commit_finish` | `Refresh` / `Landed` | `filter_commit_finish` | producer | `reset_pyramid` |
| `preview_poll` | `AdjustRefresh` | `image_adjust_preview_poll` | presenter | `reset_pyramid` |

Three constraints shape the approach:

- `Refresh::Region` and `AdjustRefresh::Region` carry worker-computed pixels. Moving pixel application into the presenter would recomposite on the UI thread and could blit before `commit_finish` swaps `session.doc`.
- "Rebuild" currently means three things: `composite_full` (layers authoritative), `reset_pyramid` (display only, `doc.composite` already exact), and `reset_pyramid_deferred` (large restore on a worker).
- `SessionChange.regions[*].srgb` is always `None`; `SessionChange::blit` is the only constructor. The `Some(srgb)` arm in `apply_session_change` is dead.

## Goals / Non-Goals

**Goals:**

- One report value and one presenter for the five producers named in the proposal.
- Ordering enforced in one place and provable by a Rust test that does not need Qt.
- Delete the dead report machinery and the duplicated preview path.
- Preserve the existing region rects, notification counts, dirty values, and content-revision values.

**Non-Goals:**

- Changing the C++ shell, the signal set, or the composite engine.
- Changing when a region path is chosen over a full rebuild.
- Unifying `document/edit-transaction`'s outcome vocabulary or `history.rs`'s `Restored` type.
- Converting producers outside the five (`cancel_paint`, the unchanged `commit_display_preview` path, `adjustment_cancel`, `image_adjust_preview_cancel`, `path_gesture_abort`, `PictureView::record`); `recomposite` stays their presenter.

## Decisions

### Keep `Edit` and `Restored`; convert at the app boundary

`Edit` is the mutation declaration and its return value is used by 20+ call sites to decide follow-up work. `Restored` is the history layer's report and is embedded in `history.rs` and its tests. Neither is a presentation dialect; the dialect is the presenter input. Add `Presentation` above them and convert:

- `Edit -> Presentation` replaces `effect_of`/`Effect`. The presentation carries `declaration: Option<Edit>` holding the *normalized* edit (`apply_edit_with` rewrites `Inserted`/full `Structural` before it composites), so `PictureView::edit` keeps returning exactly the outcome callers use today.
- `Restored -> Presentation` lives in a new `PictureViewRust::restore_presentation` wrapper (the existing `restore` keeps stepping history), absorbing `show_region`, `refresh_from_composite`, and `refresh_mask_solo`.

Alternative rejected: replace `Restored` outright — a wide change to `history.rs` with no behavioral gain.

### Regions mean "already applied; just notify"

`Presentation.regions` is a list of document rectangles whose pixels the producer has already folded into level 0 and the pyramid. The presenter emits `region_damaged` for each; it does not composite or patch. This keeps worker-computed pixels off the UI thread and preserves the `commit_finish` swap-then-blit order.

Alternative rejected: presenter applies all pixels — purest pipeline, but it regresses the worker path and risks blitting pre-swap pixels.

### The presenter owns the rebuild

`Presentation` carries `rebuild: Rebuild { None, Sync, Deferred }`. Producers leave `doc.composite` exact and do not reset the display. Split `composite_full` into `composite_into_doc` (compute and store) and the existing `reset_pyramid`; producer paths that currently call `composite_full` or `reset_pyramid` switch to `composite_into_doc` or no display reset. The presenter runs `reset_pyramid` for `Sync` and `reset_pyramid_deferred` + `build_pyramid` for `Deferred`, exactly once.

Alternative rejected: keep rebuild in producers — smaller, but leaves three meanings and the wrong-owner problem the issue names.

### The presenter owns dirty and content-revision

`Presentation` carries the dirty value to apply and a revision flag. The presenter applies them *before* the region blits and before the notification, because `afterPresent` calls `updateTabTitle`, which reads `is_dirty`. To avoid a double bump:

- the transaction gains a new capture-only history method (`capture_live_without_bookkeeping`); `record`/`record_hinted` keep setting dirty and revision for revert, snapshot restore, and `PictureView::record`;
- the filter commit, adjust poll, and preview paths stop bumping inline;
- `DocumentSession::restore` stops calling `refresh_dirty` and stops advancing `content_revision`; it derives the dirty value from the saved identity and reports it with `revision: true`;
- `Edit::Record` reports `dirty: true, revision: true` to preserve today's two-write preview-then-commit path (the preview's bump and the capture's bump).

Alternative rejected: derive `dirty`/`revision` from the presentation shape — wrong for restores (derived dirty), move commits (no revision), and adjust previews (revision without dirty).

### Restore lifecycle signals stay outside `Presentation`

`session_cancelled` and `history_restored` depend on whether the restore cancelled a live session, not on the pixels. `show_restored` keeps emitting them after `present()`. `Presentation` covers only the pixel, rebuild, notify, and bookkeeping part.

### Test seam: a pure plan plus a Rust `present_core`

Add `plan(&Presentation) -> Vec<PresentStep>` (`Bookkeep`, `Blit(rect)`, `Rebuild`, `Notify`) as a side-effect-free lowering, and `present_core(&mut PictureViewRust, &Presentation) -> Vec<PresentStep>` that applies bookkeeping and the rebuild and returns the executed steps. `PictureView::present()` wraps `present_core`, executes the Qt emission steps, and emits the signals.

The Rust test has three halves:

1. A table that drives each of the five producers on a `PictureViewRust`, then runs `present_core`, and asserts `Bookkeep` precedes every `Blit`, every `Blit` precedes `Notify`, and at most one `Notify` is executed.
2. An exhaustive sweep over rebuild kind × 0..3 regions × notify, asserting the same order and that a rebuild always notifies.
3. A bookkeeping-first case (`bookkeeping_precedes_blit`) and a rebuild-forces-notify case (`rebuild_always_notifies`).

No Qt, no property-test dependency. Signal-only and deferred-worker rows are Qt Test or `manual:` because `present()` and `build_pyramid` are QObject methods.

### Collapse the opacity/fill previews

Add one Rust helper on `PictureViewRust`, `preview_layer_property(rust, paths, value, kind) -> (i32, Option<Presentation>, Option<DisplayPreview>)`, and one `commit_layer_property(rust, paths, value, kind)`, parameterized by `PreviewKind`, the setter (`set_opacity_paths` / `set_fill_paths`), the pending flag (`opacity_changed` / `fill_changed`), and the history label. The returned `DisplayPreview` is the pre-gesture state the Qt wrapper restores *after* `present()`; a `Rebuild::Sync` presentation clears `display_preview`, so restoring it before `present()` would lose it. The Qt methods `preview_layers_opacity`/`preview_layers_fill` and `commit_layers_opacity`/`commit_layers_fill` keep their current `i32` signatures and call the helper plus `present()`. The display fast path builds `Presentation { regions: [rect], notify: false, dirty: true, revision: true, rebuild: None }` and calls `present()`.

### Delete the dead dialect

`Effect`, `effect_of`, `SessionChange`, `RegionBlit`, `SessionChange::merge`, `changed_display`, `Applied`, `Landed`, and `AdjustRefresh` are removed. The private worker payloads `WorkerRefresh` and `WorkerAdjustRefresh` remain: they carry worker-computed pixels to the presenter, and the rebuild kind replaces the old `Refresh` effect. `RegionBlit.srgb` is not carried into `Presentation`. The `SessionChange` call sites outside `cancel_active_sessions` — `cancel_path_gesture` (`session.rs`), `path_gesture_abort` (`paths/imp.rs`), `impl_history::restore`'s `changed_display` check, and `document_session/tests.rs` — are converted in the same task as the type deletion, not later.

## Risks / Trade-offs

- **Bookkeeping must precede the region blits.** `afterPresent` (`frame_canvas.cpp:169`) calls `updateTabTitle`, which reads `is_dirty` (`frame.cpp:983`), so a silent-region commit (paint/mask, `notify: false`) would show a stale dirty star if bookkeeping ran after `region_damaged`. The presenter applies bookkeeping first; `region_damaged` still precedes `changed`, so the signal counts are unchanged.
- **Rebuild moves from producer to presenter.** Producers switch from `composite_full`/`reset_pyramid` to `composite_into_doc`/no display reset; the presenter rebuilds once. History capture reads `doc`, not the display, so capture order is unaffected; verify paint commits and filter commits.
- **The adjust `Everywhere` baseline guard can go.** `reset_pyramid` clears the stroke and display previews, not the `FilterPreview` (`impl_core.rs:937-988`, `active.rs:311-319`), so the `take_filter_preview`/`set_filter_preview` take/restore around the old wrapper is a no-op. Confirm and delete it; if a future rebuild does clear the filter preview, move the guard into `present()`.
- **`composite_full` split.** Any caller that relied on `composite_full` doing both steps keeps calling it; only the presentation path calls `composite_into_doc`. Verify with the existing full-composite metrics in `damage_policy_tests.rs`.
- **Restore still needs Qt to start the deferred pyramid.** `build_pyramid` is a `PictureView` method, so the deferred branch stays in the presenter; the restore producer only reports `Rebuild::Deferred`.
- **Session cancel pixel work moves earlier.** The cancel producer now calls `refresh_regions` instead of returning a rect for the adapter. The composite and region are the same; verify `tst_document_session::regionBlitDoesNotEmitChanged`. Callers that replace the document and discard the report (`restore`, `revert`, `replace_document`, `history_restore_snapshot`) use `cancel_active_sessions_report_only`, which keeps the same report shape but skips the composite/refresh nobody reads.
- **Normalized `Edit` return.** `apply_edit_with` normalizes the declaration before compositing. Carrying the normalized `Edit` in `Presentation.declaration` keeps the existing caller contract; do not return the closure's pre-normalized value.

## Migration Plan

Land as one change; there is no persisted state or external contract to migrate. Rollback is a revert of the commit.

## Open Questions

None blocking. The exact `PresentStep` variants can shrink during implementation if `Bookkeep` and `Rebuild` collapse into one step; that does not change the specs or task breakdown.
