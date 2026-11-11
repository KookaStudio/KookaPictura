# Proposal

## Why

Issue #1278, second of two pull requests. Part 1 (#1280) added the shared
cancellable job window and moved Free Transform commit onto it. The
whole-document commands still run on the GUI thread: on a large document Image
Size, Canvas Size, Rotate/Flip, a mode or depth conversion, Merge and Flatten
each freeze the window for seconds, with no progress and no way to cancel.

## What Changes

- **One document-job runner.** A whole-document command runs its engine on a
  worker against a clone of the document (whose planes are shared, so the clone
  is cheap). The document and history change only when the job lands. The
  command records the same single history state as before.
- **Shell entry point.** `runDocumentCommand` arms the view around the
  command's bridge call and runs the job through `runCancellableJob`: wait
  cursor at once, window after 250 ms, Cancel and Esc. Interactive menu and
  panel commands use it. Replayed actions, the API, and documents under 4 MP
  stay synchronous.
- **Progress where the engine can count.** Image Size reports resampled rows.
  Rotate and Flip report layers. Merge and Flatten report composited rows.
  Canvas Size and the mode, depth and HDR conversions make passes they cannot
  weigh, so they show a busy bar. Canvas Size's per-layer step only offsets
  rects, so a per-layer bar would sit at its end.
- **Cancellation in the engines.** The row, layer and band loops stop once the
  job is cancelled, and the cancelled result is dropped.

## Capabilities

- **ADDED** to `document/long-operation-progress`: whole-document commands run as
  cancellable jobs; a job that cannot count its work shows a busy bar.

## Impact

- `pictura-ops`: `resize` reports and honours a job per row.
- `pictura-render`: `for_each_layer_job` for Rotate/Flip; `resize_document`
  declares its rows; `composite_channels` reports bands and `stream_region`
  stops on cancel; the mode passes skip layers once cancelled.
- `pictura-app`: a `document_job` module and its bridge functions; the Image
  Size, Canvas Size, Rotate/Flip, mode, depth, HDR, Indexed, Bitmap, Merge and
  Flatten bridges go through it.
- Shell: `document_command.{h,cpp}`; the busy bar in `runCancellableJob`; the
  menu and Layers/Properties panel call sites.
