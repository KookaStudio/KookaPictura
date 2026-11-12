# Design

## D1. One cost model for the fills

`memory_gate::fill_cost(doc, refresh, fork, working)` returns a `CommitCost`
whose `spillable` term is the forked tiles (`fork_bytes`). Its `working` term
is the fill's own resident buffers plus the refresh's dense composite.

- **Gradient, Edit > Fill:** working = refresh.
- **Paint Bucket:** working = `3 · flood_bytes(bounds)` + refresh. The flood
  and its visited bits, then the anti-aliased copy beside the mask, then the
  mask beside the fill. This is never below the old resident terms.
- **Edit > Stroke:** working = `3 · flood_bytes(window)` + refresh(window).

`refresh_working(doc, bounds)` is `refresh_bytes(bounds)` (6 bytes a pixel)
where `refresh_display_rect` builds a dense region composite: an
authoritative composite, a document at or under `SYNC_PIXELS` (16 Mpx), or a
region at or under `REPAIR_PIXELS` (4 Mpx). Elsewhere it is 0: the refresh
only damages the region's display tiles, and the canvas rebuilds the visible
ones within the display budget, which the governor already holds back.

The constants stay in `tiles.rs`. The gate reads them through
`pub(in crate::cxxqt_object)` re-exports, so the two cannot drift apart.

The gate reads only the document and the region. `refresh_display_rect` also
reads the view: a forced-async or painting view (`tiles_sync`), and whether a
current tile lies under the region. The gate ignores both, so it can only
over-charge, never under-charge.

ponytail: a refresh that finds no current tile under a small region, or runs
on a forced-async small document, also only damages it. Those cases are still
charged as dense, which is conservative.

## D2. Gate on the GUI thread, engine on the worker

The gate's refusal flags and its test pins are per thread, so the gate must
run on the GUI thread. `apply_job` takes the fill as two parts:

- `gate(&Document, &str, &FillTarget) -> bool` runs on the GUI thread, before
  any job starts.
- `edit(&mut Document, &str, &FillTarget, Option<&TiledMask>) -> Option<PsdRect>`
  is `Send + 'static`.

When the view is armed (`runDocumentCommand`), the document is 4 MP or more,
and the target is a pixel layer (not a mask and not Quick Mask), `apply_job`
gates and then starts a `document_op` job. The job clones the document (its
planes are shared), runs `edit` on the worker with an owned clone of the
selection, and lands through `document_job_finish`. One `edit()` transaction
installs the document and records the label.

Everywhere else, `apply_job` calls `apply` with a closure that runs `gate`,
then `edit`, and records the outcome, so `runDocumentCommand`'s `finish`
reports it.

`Outcome` gains `rect: Option<PsdRect>`, the fill's damage. The landing turns
it into `Edit::regions`, or `Edit::Full` when a styled layer could carry the
change past it, as `refresh_edit` decides.

The bar is busy: the fill engines count no rows. A cancel drops the worker's
result (the existing `take_document_job` ponytail), so the document and
history are unchanged.

## D3. Paint Bucket's All Layers sample leaves the view

`sample_tiles(doc, view)` reads one fact from the view: whether the stored
composite is current. Its body moves to `sample_tiles_from(doc, stored)`,
which `sample_tiles` calls, so its other four callers are unchanged. Paint
Bucket reads `stored_is_current` on the GUI thread and passes the flag. With
that, the closure parameter `&PictureViewRust` of `apply` has no reader and is
removed; the shapes and path callers ignored it.

## D4. Refusal message

`fillMemoryRefusalMessage()` names the scratch disks when
`edit_refused_for_scratch_full()` is set, as `moveMemoryRefusalMessage()`
does.
