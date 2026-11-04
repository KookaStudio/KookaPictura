# Test Plan

One row per scenario in the change's spec deltas. Coverage names the automated
test that proves the row; a Rust test is `pictura-app::<fn>`, a Qt Test case is
`tst_<suite>::<case>`. Ordering and bookkeeping rows target the Rust
`present_core` seam; signal-only and deferred-worker rows are Qt Test or
`manual:`.

## New capability: document/presentation

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `pr_one_value` | Run the committed edit, session cancel, restore, filter commit, and adjust poll producers on a `PictureViewRust`. | Each returns the same `Presentation` shape, and the edit presentation carries the normalized `Edit`. | `pictura-app::one_value_for_every_producer` |
| `pr_no_direct_signal` | Call each producer with no presenter running. | No Qt signal is reachable from the producers; the shell sees nothing until `present()` runs. | manual: producers are plain Rust methods with no QObject access; Qt signal counts are pinned by tst_document_session |
| `pr_bookkeeping_first` | Feed a presentation with dirty/revision and two regions through `plan()` and `present_core()`. | The trace is `Bookkeep, Blit, Blit, Notify`; no `Blit` precedes `Bookkeep`. | `pictura-app::bookkeeping_precedes_blit` |
| `pr_regions_first` | Feed a presentation with two regions and `notify: true` through `plan()` and `present_core()`. | The trace is `Bookkeep, Blit, Blit, Notify`; no `Notify` precedes a `Blit`. | `pictura-app::regions_precede_notify` |
| `pr_one_notify` | Feed presentations with 0..3 regions and `notify: true`. | Exactly one `Notify` per presentation. | `pictura-app::one_notify_per_presentation` |
| `pr_silent_region` | Feed a presentation with one region, `notify: false`, `rebuild: None`. | One `Blit`, no `Notify`. | `pictura-app::silent_region` |
| `pr_notify_only` | Feed a presentation with no regions, `notify: true`, `rebuild: None`. | No `Blit`, exactly one `Notify`. | `pictura-app::notify_without_regions` |
| `pr_rebuild_once` | Feed a presentation with `rebuild: Sync`. | One `Rebuild`, one `Notify`, and one display rebuild call. | `pictura-app::rebuild_once` |
| `pr_deferred_rebuild` | Feed a presentation with `rebuild: Deferred` on a large document. | The deferred pyramid job starts and one `Notify` follows. | manual: the exact-pyramid worker is Qt-only; the plan step is covered by pictura-app::deferred_rebuild_plan and the worker by tst_present_cache::largeDocumentPresentsThroughALevel |
| `pr_no_rebuild` | Feed a bounded presentation with `rebuild: None`. | No `Rebuild`; only the region is patched. | `pictura-app::bounded_does_not_rebuild` |
| `pr_rebuild_notifies` | Feed `rebuild: Sync, notify: false`. | One `Rebuild` and exactly one `Notify`. | `pictura-app::rebuild_always_notifies` |
| `pr_bookkeeping` | Present a committed edit through `present_core`. | `dirty` is true and `content_revision` advanced by one before the first `Blit` and the notification. | `pictura-app::presenter_applies_bookkeeping` |
| `pr_move_revision` | Present a topmost-pixel-layer move commit. | `dirty` is true and `content_revision` is unchanged. | `pictura-app::move_commit_keeps_revision` |
| `pr_restore_dirty` | Present a history restore from a clean saved state. | `dirty` reflects the saved identity and `content_revision` advanced once. | `pictura-app::restore_derives_dirty` |
| `pr_preview_presenter` | Run a live opacity, fill, and adjustment preview through the Rust preview helper. | Each returns a `Presentation`, `present_core` emits the region, and the helper emits no signal. | `pictura-app::display_preview_uses_presenter` |
| `pr_preview_shared` | Run an opacity preview and a fill preview on the same document through the shared Rust helper. | Both take the same preview and commit path, differing only in setter, kind, pending flag, and label. | `pictura-app::opacity_and_fill_share_path` |
| `pr_bounded_equivalent` | Commit a bounded edit and a bounded property edit. | The same region rect and the same change-notification count as before; canvas byte-identical. | `pictura-app::bounded_edit_equivalent` |
| `pr_unbounded_equivalent` | Commit an unbounded edit. | One full composite, no region, one change notification. | `pictura-app::unbounded_edit_equivalent` |

## Modified capability: document/document-session

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `ds_report_region` | Cancel or commit a paint or mask stroke with bounded rectangles and no notification flag. | The presenter emits the region blit(s) and no full change notification. | `tst_document_session::regionBlitDoesNotEmitChanged` |
| `ds_report_region_notify` | Commit a healing, patch, red-eye, or layer-opacity/fill batch change with bounded regions and the notification flag. | The presenter emits the region blit(s) and exactly one full change notification. | `tst_document_session::regionBlitCanAlsoEmitChanged` |
| `ds_report_notify` | Abort a Work or Saved path gesture that changes no pixels. | The report carries a notification and no region; the presenter emits one change notification without a blit. | `tst_document_session::notifyOnlyEmitsChanged` |
| `ds_report_shape_path` | Abort a path gesture that edited a shape layer's path. | The report requests a rebuild or a bounded region plus a notification, and the presenter does not treat it as notification-only. | `pictura-app::a_shape_path_abort_requests_a_rebuild` |
| `ds_report_rebuild` | Cancel a mode preview and restore the pre-preview document. | The report requests a full rebuild, the presenter rebuilds from the restored composite, and one change notification is emitted. | `tst_document_session::rebuildEmitsChanged` |

## Modified capability: document/edit-transaction

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `et_one_owner` | Commit a document change through the transaction. | The declared compositing runs, exactly one history state is captured, and the document is left dirty with its content revision advanced. | `tst_edit_refresh::aCommittedEditRefreshesLayersOnce` |
| `et_reports_outcome` | Return each mutation outcome from the transaction. | The transaction returns the same normalized outcome and the presentation carries it. | `pictura-app::one_value_for_every_producer` |
| `et_mutation_inside` | Run a mutation that refuses before writing or changes nothing. | The document is unchanged, no history state is captured, and no change signal is emitted. | manual: a refused mutation is pinned by the command handlers; the transaction returns Edit::None without capture |
| `et_bounded` | Declare one or more bounded rectangles. | Only those rectangles are composited, pixels outside keep their value, and the result equals a full recomposite. | `pictura-app::bounded_edit_equivalent` |
| `et_paint_regions` | Declare several disjoint paint or mask dirty rectangles. | The regions are composited in one pass and exactly one region blit is emitted over their bounding box. | `tst_present_cache::documentRefreshArrivesAsDamage` |
| `et_meta` | Commit a metadata-only or selection-only edit. | One history state is captured, no compositor runs, no display damage is marked, and one change notification is emitted. | `tst_edit_refresh::aSaveSelectionNotifies` |
| `et_record` | Commit an already-presented preview. | One history state is captured, no compositor runs, and no additional change notification is emitted. | `tst_properties_preview::commitUsesTheLatestValue` |
| `et_bounded_notify` | Commit a bounded batch property, mask-refine, or smart-filter toggle. | The presenter emits exactly one change notification and the panel and registry refresh. | `tst_document_session::regionBlitCanAlsoEmitChanged` |
| `et_structural` | Commit a link-invalidating edit with bounded damage. | The link sets are cleared, only the declared rectangles are composited, one region blit and one change notification are emitted. | `tst_damage_bounds::structuralDeleteEmitsOneRegion` |
| `et_structural_full` | Commit a link-invalidating edit with no bound. | The link sets are cleared, one full document composite is computed, and the presenter rebuilds the whole display once. | `tst_damage_bounds::unboundedStructuralRebuilds` |
| `et_unbounded` | Commit an edit whose pixel effect cannot be bounded. | The whole document is composited once and the presenter rebuilds the display to match. | `pictura-app::unbounded_edit_equivalent` |
| `et_inserted` | Commit a content-neutral layer insertion. | The link sets are cleared, one history state is captured, one change notification is emitted, and the composite, pyramid, and frame revision are unchanged. | `tst_edit_refresh::newLayerDoesNotRecomposite` |
| `et_capture_after_refresh` | Undo a committed edit. | The restored document and display are byte-identical to the pre-edit state and the undo path runs no full composite. | `tst_document_api::undoRedoThroughTheAdapter` |
| `et_preview` | Run a preview repeatedly and then commit it. | The previews add no history states and the commit adds exactly one. | `tst_properties_preview::adjustmentDragCoalescesPreview` |
| `et_move_revision` | Move and commit the topmost pixel layer. | One history state is captured and the content revision is unchanged. | `pictura-app::move_commit_keeps_revision` |
| `et_notify_once` | Commit an edit whose outcome notifies. | The shell is notified exactly once and no second notification follows. | `tst_edit_refresh::aCommitNotifiesOnce` |
| `et_no_double_present` | Commit an edit the transaction has already presented from a menu command. | The handler presents nothing further and the window is refreshed once. | `tst_edit_refresh::aCommandRefreshesOnce` |
| `et_meta_presented` | Commit a metadata-only edit such as saving a selection into a channel. | The shell is notified so the history state and command table reflect the change. | `tst_edit_refresh::aSaveSelectionNotifies` |

## Manual checks

- `pr_no_direct_signal` — producers are Rust methods with no QObject access; the Qt signal counts are pinned by `tst_document_session`.
- `pr_deferred_rebuild` — the exact-pyramid worker is Qt-only; the plan step is covered by `pictura-app::deferred_rebuild_plan`, and the worker by `tst_present_cache::largeDocumentPresentsThroughALevel`.
- `et_mutation_inside` — a refused mutation is pinned by the command handlers; the transaction returns `Edit::None` without capture.
