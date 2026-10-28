# Design

## Context

See `proposal.md` and
`specs/compositing/gpu-transfer/spec.md` (the modified resident-upload
requirement and the added store requirement).

Resident uploads live in one process-wide `Mutex<Resident>` per device
(`gpu/backend.rs:171`), shared by every `Gpu` instance. `Gpu` is constructed for
a region (`Gpu::new(x0, y0, w, h)`), and every upload today is sized to that
region. `composite_region_active` uses an arbitrary dirty rect; `composite_gpu`
uses the whole document.

Two facts shape the design:

- The shader samples a source by **document** pixel: `SrcLayout` carries
  document-space `x0/y0/w/h` and `sample_src` bounds-tests in document
  coordinates. A whole-document source upload therefore works for a region
  dispatch with no shader change; only the layout changes.
- The shader samples coverage by **region-local** index (`mask_byte(i)` where `i`
  is the canvas invocation index). A whole-document mask cannot serve a region
  dispatch without a new mask layout in the shader and params.

`source_geom(document_region, layer, doc)` clips to the layer rect, not the
canvas, so a document-scoped upload of a small layer stays small; only a
full-canvas layer costs a full-document upload (64 MB at 4000²).

## Goals / Non-Goals

**Goals:**

- A stream of region presents leaves the store flat for unchanged layers.
- A following full composite hits every unchanged layer's source and sampled
  mask, and re-uploads only content that changed.
- The store is bounded by bytes and entry count, with a constant-time lookup and
  no new dependency.
- Resident usage is observable through the existing metrics counters.

**Non-Goals:**

- A document-space mask layout. That would let region composites cache masks
  too, but it changes the shader and its params for an upload that is at most
  one byte per pixel.
- A constant-coverage mask mode in the shader (no mask buffer at all). It is a
  larger kernel change and does not affect the growth this issue reports.
- Lowering the 2 GB budget or making it adapter-specific.
- Hybrid CPU/GPU source sharing (the `layer_source` cache from #969 is a
  separate, content-keyed CPU cache).
- Zero-copy present.

## Decisions

### Look up a document-scoped key, build a region-sized upload on a miss

`build_source` computes the region-clipped geometry (to skip a layer outside the
region) and the document-clipped geometry. The key contains the plane stamps and
lengths, the raw layer rect (which determines the row mapping), the document
width/height (which determines the clip), and the grayscale flag. No region.

On a lookup hit the returned buffer is the whole-document upload and the dispatch
layout is the document-clipped geometry. On a miss the compositor assembles only
the region-clipped geometry, at the cost it pays today, and does not retain it.

Alternatives:

- *Build the whole document on every region miss.* Correct and simpler, but a
  full-canvas layer would upload 64 MB per present instead of 256 KB during a
  stroke. Rejected.
- *Keep region keys and only stop retaining them.* Prevents growth but never
  hits the whole-document uploads during a stroke, and fails the acceptance that
  a following full composite hits every unchanged layer. Rejected.
- *Key by `Plane::ident()` instead of stamps.* The GPU store already relies on
  the caller's stamp discipline; the content-keyed CPU cache uses idents because
  it survives across documents differently. Mixing the two identities here buys
  nothing.

### Retain uploads only for whole-document composites

`kept_or_upload` takes a `keep` flag, set when the `Gpu` region equals the
document. Region-sized source and mask uploads are built and dropped. The store's
key space then becomes one entry per layer content state (source) plus one per
sampled mask, instead of one per present.

### Do not retain constant-coverage masks

A layer with no enabled data-carrying mask has coverage 255 inside its influence
rect and 0 outside, rebuilt row-wise by `mask_writer` without per-pixel work.
Retaining one byte per pixel per distinct influence rectangle would occupy the
shared budget that the sources need; layers with the same rect shared the old
entry (one 16 MB mask served the eight full-canvas layers in the benchmark),
while the rebuilt path uploads 8 × 16 MB per composite. `mask_key` returns no
key for that case, which also lets `UploadKey::Mask.sampled` stop being
optional: the key exists only for a sampled mask. Coverage for a maskless layer
is uploaded per composite (16 MB at 4000²), a cost the issue accepts in
exchange for the memory.

### Bounded store: `HashMap` keyed, insertion-order queue

`Resident<V = wgpu::Buffer>` holds `HashMap<UploadKey, (V, u64 size)>`, a
`VecDeque<UploadKey>` in insertion order, and a running byte total. A manual
`impl<V> Default for Resident<V>` keeps `wgpu::Buffer: Default` out of the
picture, and `UploadKey` derives `Hash`. `get` is a map lookup that does not
mutate the store, so it is O(1); the resident mutex remains exclusive, but the
lookup takes no extra work.

`put` refuses an upload larger than `BUDGET` (the current composite still uses
the buffer). Otherwise it removes the previous entry for the key from both the
map and the queue, subtracts its bytes, inserts the new entry at the back, and
evicts from the front while the store is over `BUDGET` or `MAX_ENTRIES`
(`1024`). A replace therefore keeps exactly one queue entry per key and cannot
evict the newer upload first. `put` returns the post-eviction `(entries, bytes)`
so the device-side caller publishes the gauges; the generic store used by unit
tests has no global side effects.

Eviction order is insertion order, not access order: a hit does not reorder the
queue, which keeps `get` O(1). Keys for content that has since changed stay until
evicted, so a long editing session fills the store with stale keys up to the byte
budget or entry cap; FIFO then evicts them oldest-first, which can also evict a
hot unchanged layer's older entry. Accepted ceiling: that layer re-uploads once on
the next full composite. Touch-on-hit LRU is the upgrade path if a profile shows
it matters.

Alternatives:

- *`indexmap`.* One dependency for LRU ordering, which the design does not need.
  Rejected.
- *Keep the `Vec` and linear scan.* O(n) `get` with an entry cap of thousands is
  avoidable with std `HashMap`. Rejected.

### Mask keys stay region-scoped

A sampled mask key keeps the region and the influence rect because the upload is
region-sized and region-local. Since only whole-document builds are retained, the
region in a retained key is always the document; the field remains because the
key must still distinguish a sub-region build that is not retained (and because
the same key function serves both).

### Metrics gauges

`kept_or_upload` publishes the `(entries, bytes)` that `Resident::put` returns to
new atomics in `metrics.rs`; `Counters` gains `resident_entries` and
`resident_bytes`, and the app metrics JSON prints them next to the existing
resident hit/miss counters. Publishing at the device call site keeps the generic
store used by the `Resident<u64>` unit tests free of global state, and the
post-eviction snapshot makes the gauges exact on keep and eviction.

## Risks / Trade-offs

- **A full composite after an edit misses the changed layer once** (then retains
  the new upload). This is expected and covered by the acceptance wording; only
  unchanged layers must have zero misses.
- **Constant masks upload per full composite.** ~1 byte/pixel per maskless
  layer. Mitigation: the fill is row-wise and the memory it frees protects the
  source budget. A shader constant-mask flag is the follow-up if this shows up
  in a profile.
- **A single upload larger than the budget is not retained.** It is still used
  for the composite that built it, and the next composite rebuilds it. At 2 GB
  and 4 bytes/pixel this needs a document past ~23k², so the ceiling is
  deliberate.
- **Stale keys across documents.** The key carries document size, the layer rect,
  and stamps, so two uploads compare equal only when their bytes are identical.
  A write clears the stamp, as before. The existing
  `resident_uploads_are_reused_and_never_stale` test stays the contract.
- **FIFO can evict a hot unchanged layer after enough edits.** The entry cap and
  byte budget bound the stale keys; the accepted cost is one re-upload on the
  next full composite. Touch-on-hit LRU is the upgrade path.
- **Global gauges and store across concurrently running device tests.** CI runs
  each test in its own process (nextest); the assertions are exact before/after
  deltas measured inside one test, so a shared process cannot move the baseline.
- **The 2 GB budget still counts system RAM on an integrated GPU.** Unchanged by
  this change; the growth to the budget is what is being fixed.
