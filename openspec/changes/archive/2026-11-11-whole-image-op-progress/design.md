# Design

## Runner

`PictureView::document_op(label, bar, op, land)`:

- `op: FnOnce(&mut Document) -> Option<Outcome> + Send` is the engine call.
  `None` is a refusal that leaves the document unchanged. `Outcome` carries a
  merged layer's path and the count of layers it replaced.
- `land: FnOnce(&mut PictureViewRust, &Outcome) -> Edit` finishes the view
  state, such as the selection or the layer target, and names the history edit.

When the view is **armed** and the document has at least 4 MP, the document is
cloned and `op` runs on a worker under a `JobProgress` token. Otherwise both
closures run inside one `edit` transaction at once, as the bridges did before.

Only the shell arms the view, around one bridge call, so every other caller (the
API, action replay, self-test, Rust tests) is synchronous by construction. The
arm call also clears the last outcome, so a bridge that refuses before reaching
the runner reads as "not applied".

`document_job_finish` joins the worker and, inside one `edit`, installs the new
document and runs `land`. After a cancel it drops the job, so nothing changes.
A reader or writer that arrives while a job is pending (the API, say) lands it
synchronously through `settle_pending_composite`, as part 1 does for transforms.
Closing or replacing the document drops the job.

## Progress

`JobProgress` stages are left at one. Each engine that can count opens a stage
with its unit total and advances through it:

| Command | Units | Where |
|---|---|---|
| Image Size | resampled output rows over every plane | `resize_document` (total), `pictura_ops::resize` (per row) |
| Rotate / Flip | layers | `for_each_layer_job` |
| Merge / Flatten | composited rows | `composite_channels` |

A second stage opened by the same command holds the bar where it is: the value
is clamped to the last stage and never decreases. A command that cannot count
reports `-1` through the bridge, and `runCancellableJob` shows a busy bar for it.

## Cancellation

Each counted loop checks the flag, and so do the mode passes' layer walk and the
banded compositor. The tone maps cannot stop early. A cancelled one finishes on
its detached worker and its result is dropped (`ponytail:` in
`take_document_job`). The GUI never waits for that worker.

## Mode preview

Indexed Color's live preview swaps a converted document in. The commit restores
the base and recomposites before it starts the job, so a cancelled conversion
leaves the canvas on the base document and not on the preview.
