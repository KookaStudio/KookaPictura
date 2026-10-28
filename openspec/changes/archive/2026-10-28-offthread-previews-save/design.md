# Design

## Context

See `proposal.md` for motivation and the deltas under `specs/` for the
contract. The app already has the shape this change extends:

- `open_jobs.rs` starts a std thread, owns no Qt types on the worker, and posts
  its result with `CxxQtThread`; the view carries a request id so a stale result
  is dropped.
- `filter_tools/jobs.rs` keeps a latest-wins exact preview and a commit job; a
  worker filters a copy-on-write clone and the GUI swaps the result in.
- `Document` and `Layer` clone by `Arc` refcount (`Plane`), so a snapshot is
  cheap until a worker writes a plane.
- `Plane::ident()` is the established cache identity for bytes-derived work
  (`layer_source`, `mask_refine`); it is assigned lazily and cleared by every
  mutable access.
- `recovery.rs` already writes snapshots on a worker, but it calls
  `write_psd`, which re-encodes every channel.
- `filter_preview_dialog.cpp` already coalesces ticks and polls an exact-preview
  worker below 100% zoom.

The writer is at 1106 lines (`write.rs`, hard cap 1200), so the layer-section
writer moves before it grows. `impl_filters.rs` is at 1174 lines (26 lines of
headroom): the `PreviewBaseline` impl moves to `document_session/previews.rs`
if the adjustment work needs more there than a visibility change.

## Goals / Non-Goals

**Goals:**

- One job shape for preview, apply, and save: GUI snapshots, worker computes,
  GUI applies; latest-wins where ticks can arrive faster than work.
- An encode cache whose hit is byte-identical to a fresh encode, with the
  session, not a global, owning lifetime.
- Keep every synchronous entry point working for tests, the self-test, the
  control server, and non-interactive callers.
- Keep the existing filter and adjustment semantics: preview records no
  history, cancel is bit-identical, OK records one state.

**Non-Goals:**

- Background Save with editable-during-save (proposal's explicit exclusion).
- A new dedicated thread pool or async runtime; one std thread per job, and the
  codec reuses rayon's existing global pool.
- Caching 16/32-bit or synthesized Lab/CMYK/Indexed/bitmap channel streams.
- Reworking the filter proxy for section previews or dedicated dialogs.
- HDR Toning, Match Color, Replace Color, or the direct adjustment commands.

## Decisions

### Save: start/running/finish, with the sync API joining

Native save splits into `save_start`, `save_running`, and `save_finish` free
functions (their own bridge, like `open_jobs`). `save_start` computes the
format, embed choice, dirty state, vanishing planes, and a document clone on the
GUI thread, then moves the clone into a worker. The worker runs
`refresh_native_composite`, `save_view`, the vanishing/profile mutations,
`write_psd_cached` or `write_psb_cached`, and `write_atomically`.

`save_start` captures the document's history id, `content_revision`, and
`document_generation`; `save_finish` joins the handle and compares them against
the live document. When the generation is unchanged, it records the path and
calls `mark_saved` only if the history id and content revision still match;
otherwise it records the path and leaves the document modified. When the
generation changed (the document was replaced while the save ran), it leaves the
replacement's path and modified state unchanged. `save_finish` reports whether
the save applied to the current document so the frame records its document path
and title only for the current or stale cases; a replaced document keeps the
frame's path and title unchanged too. A failed write records neither the path
nor the saved state.
The existing `PictureView::save` becomes `save_start` + `save_finish`, so tests
and the self-test keep a synchronous bool.

The C++ shell uses a `runBusyJob` helper (`QEventLoop::ExcludeUserInputEvents`,
wait cursor, dialog after 250 ms) so timers and paints run while user input
waits. Raster and PDF paths keep their current code; the frame checks the
path's format before choosing the job path. The smart-object editor's temp
`contents.psd` save in `frame.cpp` uses the same start/busy/finish path. The
frame drops the recovery snapshot only when the save leaves the document clean.
The control server calls the frame, so its save request gets the same nested
loop with input excluded: control requests are serialized and the request
blocks until the save lands; it does not use a separate synchronous bool.

Alternatives:

- Queued completion via `CxxQtThread` like `open_jobs`: unnecessary because the
  save result is one status and one path, and the frame needs to block input
  anyway. A join makes the handoff trivial.
- Allow edits during save: that is Background Save (#637); it needs a
  generation/rebase protocol and progress reporting, not just a worker.

### Encode cache: per-channel layer keys, one composite block, session-owned

The cache lives in `write_cache.rs` as `EncodeCache` with a `Mutex` map from a
`ChannelKey` to `Arc<[u8]>`. A layer channel's key is the source plane's
`Plane::ident()`, its pixel dimensions, output depth, compression, PSB flag, and
channel role (color or mask); the plane identity distinguishes channel ids.
A hit returns the encoded layer stream; a miss encodes and inserts.

The composite image-data section is one on-disk block: RLE puts every plane's
scanline counts before every row, and ZIP/ZIP-with-prediction deflates the
planes together. It is cached as **one** entry, not per channel, keyed by the
composite buffer's `Plane::ident()`, pixel dimensions, output mode,
compression, depth, PSB flag, and channel count. Synthesized Lab/CMYK/Indexed
composite blocks are not cached. `OutChannel::Encoded` changes from `Vec<u8>` to
`Arc<[u8]>` so a hit is a refcount bump.

`DocumentSession` owns `Arc<EncodeCache>`; `install` replaces it, and both native
saves and `recovery_write` clone the `Arc`. Cacheable layer channels are
deliberately conservative: only when the output depth is 8-bit, the layer has no
retained `source_channels`, and the document is not in a synthesized
Lab/CMYK/Indexed/bitmap/Duotone output mode. That covers the common
RGB/Grayscale case, keeps the key honest, and avoids re-deriving a synthesized
plane's identity from several inputs. After a successful encode the writer
prunes every key the encode did not use, so the cache tracks the live document
rather than growing with edit history. A recovery snapshot's encode prunes like
any other successful encode; it does not clear the cache wholesale. There is no
byte budget in this change: the cache holds at most the latest encode's
cacheable entries. `ponytail: session-pruned cache, no byte budget; add an LRU
budget if profiling shows memory pressure.`

The layer-section writer moves to `write_layers.rs`. It builds a channel plan
per layer (owned for widened/synthesized data, borrowed for working planes),
flattens the plans into tasks, and uses rayon's `par_iter` to encode or fetch
each stream. The plan keeps task order, so records and channel data are written
in the serial order; errors are scanned in task order to keep failures
deterministic. `encode_scanlines` reuses one PackBits row buffer instead of
allocating per row.

Alternatives:

- Per-layer cache entries: fewer lookups, but a changed channel would discard
  all of a layer's encoded streams and the key would have to cover synthesized
  planes. Per-channel is a smaller key and a smaller invalidation unit.
- Cache in `layer_source` (global): that cache is process-wide and bounded;
  PSD streams are large and belong to one open document, so a session is the
  natural owner.
- Parallelize inside flate2 instead of across channels: flate2 is per-stream
  single-threaded, and parallel channels already saturate the corpus.

### Non-RGB reuse: store the working-layer identities

`SourceChannels` gains `working_idents: [u64; 3]` and a manual `PartialEq` that
ignores that cache hint, so document equality still depends only on bytes and
rect. The five existing struct literals (`pictura-core/src/tests.rs`,
`pictura-render` crop/resize/transform tests) are updated with
`working_idents: [0; 3]`; `SourceChannels::new` defaults it to `[0; 3]`.

The reader's normalization step records the idents of the three working RGB
planes after converting the retained CMYK/Lab layer planes. Zero means unknown.
`cmyk_layer_color_planes` and `lab_layer_color_planes` first check whether the
retained store's idents still match the live working layer planes: a match
re-emits the retained planes; a known mismatch converts directly from the
working RGB; only an unknown store keeps the old forward-conversion check. The
rule covers layer source planes; the composite's Lab/CMYK forward checks stay
out of scope.

This is deliberately not byte-identical to the old forward check in one case:
when a plane's identity is a known mismatch but an edit round-trips to the same
working RGB bytes (for example a profile conversion rewrote the working planes
and the retained store still forward-converts equal), the new path converts and
may write different encoded bytes. The file still reopens to the same working
RGB within the documented tolerance. The spec states this divergence instead of
claiming equality.

Alternative: add an ident to `Samples`. `Samples` is a plain `Vec` enum and is
cloned by value throughout the reader; giving it a lazily assigned identity is a
larger core change than one identity array on the store that already knows the
layer rectangle and depth.

### Adjustment dialogs: worker returns a layer and its changed region

This covers the dialogs backed by the shared `AdjustmentDialog` (the generic
page, Levels, Curves, Brightness/Contrast, Hue/Saturation, Color Balance,
Black & White, Gradient Map, Photo Filter, Channel Mixer, Exposure, Vibrance).
HDR Toning, Match Color, and Replace Color keep their existing dialogs and are
out of scope.

`image_adjust/jobs.rs` keeps a latest-wins preview job. On start the GUI clones
the document, restores any open preview's baseline into the clone, clones the
selection, and spawns a worker. The worker builds the selection mask from the
selection, applies the adjustment over the whole layer with
`apply_adjustment_region`, and returns the adjusted layer plus the composited
changed region (the same `Refresh::Region` shape as
`filter_tools::jobs::commit`). The live document is untouched until the GUI poll
takes the result for the latest request.

On apply, the GUI takes the session's whole-layer baseline when one exists, or
saves a whole-layer baseline from the live layer on the first result; it swaps
the worker's layer into the document, stores the baseline in the session, and
installs the worker's composited region. It does not call `preview` with the full
layer rect: on a 24 MP layer that would composite the whole document on the GUI
thread. A later result replaces the layer and keeps the original baseline.
Cancel drops the job and calls `cancel_filter_preview`, which restores the
baseline and returns the region to refresh. A layer-mask target keeps the
mask-sized synchronous path.

OK cancels the preview (restoring the pre-dialog layer), then starts the shared
commit job with `ActiveOp::Adjustment`; the frame helper spins the busy loop and
finishes it. The commit worker restores the pre-preview doc, applies the
adjustment to the whole layer, composites the changed region, and captures one
history state — the same code path as a filter commit.

Alternatives:

- Render adjustments on the canvas proxy: cheaper per tick, but introduces a
  second preview representation and diverges from the committed pixels. The
  pointwise pass is fast enough on a worker and the result is exact.
- Return only the changed section pixels: avoids copying the whole layer, but
  the GUI then has to merge sections and maintain a union baseline. Swapping the
  whole layer plus the worker's composited region is the smaller, safer
  bookkeeping.

### Large whole-layer filter previews: reuse the exact preview, skip the draft

`FilterPreviewDialog::runPreview` keeps its current paths below 100% zoom. At
100% or above, when `positional_` (whole-layer) and the document is over 4 MP,
it calls `prepareExactPreview`, which computes the visible section clamped to
the document, sets `proxyArea_`, `proxyScale_`, and `proxyShown_` without
drawing a synchronous draft, caps the region and scale to the existing
`kCanvasProxyPixels` 4 MP budget, starts `filter_exact_preview_start`, and polls
as it already does. `pollExactPreview` then installs the exact picture over
`proxyArea_` because `proxyShown_` is already set; the busy bar shows meanwhile.
A newer parameter change supersedes the older request, as today.

`filter_exact_preview_start` returns false for a layer-mask target, so the new
branch must fall back to the existing synchronous mask preview. The commit path
already reuses a finished exact preview for the same filters.

Small documents keep the synchronous path, which preserves immediate preview
for the existing lens-blur, shear, and small-document tests and avoids a worker
round trip where the GUI work is already under the threshold.

Alternative: run every preview through the worker. That changes the
live-document preview contract for small documents, breaks tests that assert
`view->image()` changes immediately, and buys nothing on a 64×64 layer.

### Shared busy loop

`busy_job.{h,cpp}` owns the wait cursor, the 250 ms delayed dialog, the 16 ms
poll timer, and `QEventLoop::ExcludeUserInputEvents`. `runFilterCommit`
becomes a wrapper, and the save and adjustment apply paths use the same helper.
The helper takes start/running/finish callbacks, so it does not know about
filters, saves, or adjustments.

### Test seams and observability

Each job gets a `set_delay` seam in the style of `open_set_delay`: the worker
holds its result for the requested milliseconds after doing the work. The save
and adjustment jobs expose their delay seams; the exact-preview worker gets
`filter_exact_preview_set_delay`. A Qt test starts the job, runs a 10 ms timer probe, and
asserts the timer kept firing with no gap over 50 ms. Rust tests cover the
encode cache directly: hit bytes equal a fresh encode, an edited plane
re-encodes, `layers_encoded` is one after a one-layer edit, parallel and serial
outputs match, and a snapshot after a save hits the unchanged channels.

A test-only CMYK conversion counter (or a poisoning-profile seam) proves the
unchanged layer reuses its retained planes without an RGB round trip. The
non-RGB identity path is exercised for both `write_cmyk` and
`lab_layer_color_planes`.

The 24 MP profile is an ignored `pictura-codec` benchmark, not app metrics: it
builds a 20-layer 6000×4000 document, edits one layer, encodes cold and warm,
and prints both timings and `layers_encoded`. `tst_image_adjustments` gains a
small `waitForPreview(view)` helper; every existing synchronous preview
assertion is wrapped with it.

## Risks / Trade-offs

- **Cache memory** → session-pruned and dropped on install; no budget yet. The
  upgrade path is an LRU byte budget in `EncodeCache` without a spec change.
- **16/32-bit and synthesized modes bypass the cache** → the acceptance case is
  8-bit RGB; CMYK/Lab layer changes still get the identity fast path, but their
  synthesized channel streams and composite blocks are re-encoded. A follow-up
  can key synthesized groups by their input idents.
- **Parallel writer correctness** → outputs are assembled in task order and
  tested against the serial writer; if `Document` is not `Sync` for some field,
  the fallback is a serial channel loop with the same cache.
- **Adjustment baseline bookkeeping** → the first result saves a whole-layer
  baseline, later results keep it; cancel restores it. The worker's composited
  region keeps the GUI from compositing the whole layer. Tests cover cancel,
  rapid ticks, and the mask-target fallback.
- **Save stale-snapshot race** → input is blocked, but a timer or the control
  server can still edit. `save_finish` compares the history id,
  `content_revision`, and `document_generation`; a changed document keeps its
  path and dirty state, and a replaced document keeps its own path and dirty
  state. The frame only drops the recovery snapshot when the save leaves the
  document clean.
- **File-size cap** → the layer writer moves to `write_layers.rs`; the cache
  lives in `write_cache.rs`; `impl_filters.rs` is at 1174/1200, so
  `PreviewBaseline` moves to `document_session/previews.rs` if more than the
  visibility change is needed there. New files stay under the 800-line target.
- **Raster and PDF stay synchronous** → the issue's acceptance names native
  save; moving the Qt-bound raster encoder needs a separate design.
- **No new dedicated thread pool** → one std thread per job; the codec's
  `par_iter` uses rayon's existing global pool. Preview coalescing and the
  shared commit slot keep at most one preview and one commit per view.

## Migration Plan

No file-format or persisted-state migration. The cache is in memory only and
is dropped with the document session. Synchronous APIs remain, so a caller that
does not opt into the job path behaves as before. Reverting the change removes
the cache and returns each path to its synchronous implementation; files written
before, during, or after the change are byte-compatible.
