# Proposal

## Why

Adjustment previews, large whole-layer filter previews, and native saves still
run on the GUI thread, and a native save re-encodes every layer channel whether
or not it changed. On a 24 MP document a single adjustment tick or whole-layer
filter tick freezes the event loop, and a save can freeze it for seconds; an
autosave pays the same full encode. Refs #972.

## What Changes

- Route destructive adjustment dialog previews and applies for dialogs backed by
  the shared `AdjustmentDialog` through a worker: the worker adjusts a
  copy-on-write document clone, ticks coalesce, the latest result is swapped
  into the live layer on the GUI thread, and OK applies the whole layer as one
  history state. Cancel restores the pre-dialog pixels bit-identically. A
  layer-mask target may keep the mask-sized synchronous path.
- Route large whole-layer filter previews (more than 4 MP of document, at 100%
  zoom or above) through the existing exact-preview worker instead of filtering
  the live layer on the GUI thread. The exact picture is capped to the same
  4 MP budget as the draft, is displayed when it lands, and falls back to the
  synchronous path for a layer-mask target. Below 4 MP the synchronous path
  stays.
- Run native `.psd`/`.psb` saves on a worker: clone the document, refresh the
  native composite, encode, and write atomically off the GUI thread. User input
  waits while it runs; timers and paints continue; a busy indicator appears when
  the write outlasts 250 ms. The synchronous save API keeps its behavior by
  joining the worker. On success the save records the path; it clears the
  modified state only when the same document's snapshot is still current. A
  document changed during the save keeps the path and remains modified; a
  replaced document (generation mismatch) keeps its path and dirty state
  unchanged.
- Add a session-scoped PSD encode cache keyed by the source plane's
  `Plane::ident()` plus the encoding parameters, so a re-save encodes only the
  layers that changed. Cache the whole composite image-data block as one entry
  keyed by the composite identity plus pixel dimensions, output mode,
  compression, depth, PSB, and channel count; synthesized Lab/CMYK/Indexed
  composite blocks are not cached. Encode layer channels in parallel, reuse a
  row scratch buffer, and expose hit/miss/`layers_encoded` counters. A recovery
  snapshot shares the cache, so autosave cost scales with changed layers.
- Detect an unchanged non-RGB **layer** source plane by content identity
  instead of a forward CMYK/Lab color round trip; an edited plane still
  converts. The non-RGB composite forward checks stay out of scope.
- Out of scope: Background Save with editable-during-save (#637), raster and
  PDF encoding off the GUI thread, dedicated filter dialogs (Lens Flare, Picture
  Frame, Reduce Noise), non-whole section previews, HDR Toning, Match Color,
  Replace Color, and direct adjustments (Invert, Desaturate, Equalize, Auto).

## Capabilities

### New Capabilities

- `codec/psd-encode-cache`: content-keyed reuse of encoded PSD channel streams,
  whole-block composite reuse, parallel layer-channel encoding with
  byte-identical output, a session-scoped cache with pruning and counters,
  shared by saves and recovery snapshots, and identity-based reuse for unchanged
  non-RGB layer planes.

### Modified Capabilities

- `document/document-lifecycle`: native Save and Save As run their composite
  refresh, encode, and atomic write off the GUI thread while input waits; the
  path is recorded, and the modified state is cleared only when the same
  document's snapshot is still current. A replaced document keeps its path and
  modified state unchanged.
- `document/crash-recovery`: a recovery snapshot encodes through the document
  session's encode cache, so a snapshot after a save encodes only changed
  channels; writing it does not clear the cache.
- `imaging/filter-dialog-preview`: a whole-layer filter preview over more than
  4 MP at 100% zoom or above runs on the exact-preview worker, displays the
  exact picture it computes, and falls back to the synchronous path for a
  layer-mask target.
- `imaging/image-adjustments`: destructive adjustment dialog previews and OK
  applies for the shared `AdjustmentDialog` family run on a worker, coalesce
  parameter changes, swap the latest result into the live layer without history,
  and restore bit-identically on cancel; a layer-mask target may keep the
  mask-sized synchronous path.

## Impact

- `crates/pictura-codec`: the layer-section writer moves to `write_layers.rs`;
  a new `write_cache.rs` holds `EncodeCache`, its key, and the whole-composite
  block key; new `write_psd_cached` / `write_psb_cached` entry points;
  `encode_scanlines` reuses one row buffer; an ignored 24 MP benchmark reports
  cold/warm timings and `layers_encoded`. `rayon` joins `pictura-codec` (already
  a workspace dependency).
- `crates/pictura-core`: `SourceChannels` gains the working-plane identities
  used by the non-RGB identity check and a manual `PartialEq` that ignores them;
  every existing struct literal is updated.
- `crates/pictura-app`: new `impl_core/save_jobs.rs` and `image_adjust/jobs.rs`;
  `filter_tools/jobs.rs` generalizes its commit to an `ActiveOp` and the
  adjustment worker returns the composited changed region; `DocumentSession`
  owns an `Arc<EncodeCache>`; `recovery.rs` encodes through it; `state.rs`
  tracks the save and adjustment jobs; `build.rs` registers the new bridge
  modules.
- `crates/pictura-app/cpp`: a shared busy-job helper; the native save path in
  `frame.cpp`/`document_api`, including the smart-object temp `.psd` save; the
  frame drops a recovery snapshot only when the save leaves the document clean;
  preview polling in `adjustment_dialog.cpp` and `filter_preview_dialog.cpp`; a
  new `tst_offthread_jobs` Qt suite; CMake lists the new sources.
- No PSD byte-layout change: a cache hit equals a fresh encode and the
  psd-tools round-trip oracles stay green. No new dedicated thread pool: one
  std thread per job, and the codec reuses rayon's existing global pool.
