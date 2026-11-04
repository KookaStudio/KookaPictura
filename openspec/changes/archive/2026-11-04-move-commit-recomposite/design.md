# Design

## Measurements

`move_gesture_profile` (ignored), 16507×16196 with a canvas-sized top layer.
The commit was measured at half size per side (`PICTURA_PROFILE_DIVISOR=2`,
8253×8098) and scales with area:

| Phase | Half size | Full size, ×4 |
|---|---|---|
| Translate the layer | < 1 ms | < 1 ms |
| Commit composite and display | 1031–1321 ms | ≈4.1–5.3 s |

The exact composite is inherently whole-document, so a cheaper synchronous
path cannot meet the interaction target. It moves off the GUI thread.

## Deferred composite

The expensive half of `refresh_region_buffer` is `composite_region_active` and
`buffer_to_srgb`; the GUI half is patching `doc.composite`, level 0 and the
pyramid. The commit splits them:

- `composite_region(doc, rect, gpu)` is pure: it returns the clamped rect, the
  working-space buffer and its sRGB frame.
- The worker clones the document (a refcount bump over shared planes), runs
  `composite_region` for each dirty rectangle, and queues the result.
- Landing patches `doc.composite` (replacing it whole when the region is the
  document), folds the sRGB frame into level 0 and the damage account, then
  captures the move's history state and emits `regionDamaged` and
  `moveCommitted`. A whole-document region on a document over 16 MP installs a
  coarse pyramid and leaves the exact pyramid to a worker, as a restore does
  (#1098): the canvas presents the coarse level until `pyramidSettled`. A
  partial region, or a document at or under 16 MP, updates the pyramid
  synchronously.

A `generation` numbers each job. A job lands only when its generation is still
pending; a later commit, a settle, or a document replacement clears the pending
state and drops the result. This is the `deferred_pyramid` pattern (#1098).

## Threshold

A commit defers when the document and the union of its dirty rectangles each
reach 16 MP, the same size at which a restore defers its pyramid. Below it the
thread hand-off costs more than the composite. A live session, a pending
preview or a soloed mask disables the deferral, because their display state
needs the full rebuild the synchronous path performs.

## Keeping the preview

`commit_move` returns as soon as the layer is translated. The C++ Move tool no
longer calls `ImageView::endMovePreview()` on release; it clears the preview on
`PictureView::moveCommitted`, which is emitted immediately by a synchronous
commit and by the landing of a deferred one. A nudge (`translate_layer`) has no
preview; the signal is harmless.

## Settling

`settle_move_composite` finishes the pending job on the GUI thread by running
the same `composite_region` calls and then landing them, so a reader never sees
a stale composite and never waits on the worker. It is called before:

- the edit transaction (`edit`, `commit_move_edit`, `preview`),
- a Move press or duplicate preview,
- `undo`, `redo` and `history_jump` (before their `can_*` checks),
- a save (native, raster and PDF),
- a plain `sample_argb` (the eyedropper and the info panel),
- a filter proxy / pane composite.

The `picture()` accessor also reports a pending composite as not current, so
every remaining consumer composites exactly instead of reading stale bytes.

The Info panel's hover readout is not a settle point: it reads the maintained
composite through `composite_argb`, so a pointer move never blocks on a pending
move. The readout catches up on the next refresh after the worker lands.

History capture is part of landing/settling, not of the commit. A history diff
therefore always sees the patched composite, and a pending move is one history
state by the time the next edit begins. `can_undo` counts a pending move as an
undoable state; `can_redo` is false while it is pending.

## Limits

- The worker composites the whole dirty union, even when the moved layer is
  smaller than the document. The union is what the region path already used.
- A settle triggered immediately after a commit pays the full composite on the
  GUI thread. Correctness comes first; the common path lets the worker land.
- The worker result holds the working-space and sRGB buffers until landing,
  the same transient memory the synchronous path already held.
