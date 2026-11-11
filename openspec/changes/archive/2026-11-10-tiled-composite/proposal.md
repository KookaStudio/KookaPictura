# Proposal

## Why

Issue #1216 is step 8 of #1209 (bounded memory for large documents). After
#1213–#1215 the layers, masks and history are tiled and shared, but the
**display** still holds whole-document buffers. At 17000², these are the M5,
M7, M8 and M6×20 residuals recorded on #1216:

- **`doc.composite`.** A full-resolution RGBA composite (1.16 GB) is kept
  current after every edit and stored in history as a tracked plane.
- **The sRGB level-0 frame.** `PictureViewRust.level0` shares the composite's
  tiles in sRGB, but forks them on every non-sRGB document and every live
  stroke.
- **Stored pyramid levels.** Each level below 0 is one interleaved buffer: about
  385 MB at 17000², with level 1 alone 289 MB.
- **Solo-mask display.** Soloing a mask builds a document-sized display plane
  and its whole pyramid synchronously. A restore at 16507×16196 takes 704 ms on
  the GUI thread (#1177 audit).

The viewport only ever shows the visible tiles at one level.
`docs/01-architecture/gpu-rendering-pipeline.md` already specifies 256² tiles
in a generation-keyed LRU, and says that no undo record stores pixels the
pipeline produced.

## What Changes

- **`composite_region(rect, level)`** in `pictura-render` returns any rectangle
  of the composite at any pyramid level. A level-0 result is byte-identical to
  the same crop of `composite_rgba`. A level-k result is byte-identical to the
  same crop of today's halved, premultiplied `ViewPyramid` level. Large
  rectangles stream in bands.
- **A tile pyramid cache** (`TilePyramid`) replaces the stored levels and the
  level-0 frame. It holds 256² tiles per (source, level, tile) with damage
  generations, and is registered with the governor as droppable memory. Visible
  tiles are pinned for the current paint. Everything else is evicted LRU under
  the display budget, and dropped first under memory pressure (#942 closes into
  this).
- **The viewport** asks for the visible tiles only. A missing tile is shown from
  the nearest cached coarser tile until a worker delivers the exact one. The
  `LevelProvider` seam stays, and the whole-level zero-copy `levelImage` view is
  replaced by tile handles.
- **`doc.composite` is no longer maintained for layered documents.** It stays a
  stored (tiled) plane only where it is the source of truth: documents with no
  layers, non-RGB documents, and retained native-depth merged composites. Other
  readers either ask for regions or, on the remaining `ponytail:` paths, call a
  counted `materialize_composite` that the governor can drop afterward. History
  no longer tracks it for layered documents.
- **Named readers are migrated (R4):**
  - PSD/PSB save streams the merged image in bands.
  - Flatten composites band by band into the new layer.
  - Sample All Layers (bucket, wand, Grow/Similar, eyedropper, clone, smudge,
    mixer) reads composite tiles on demand.
  - The Info readout and the histogram read cached tiles.
- **Solo mask (R7)** uses the same tile cache with the mask as its source.
  Visible mask tiles are built from the mask plane per tile, with no
  document-sized display plane and no synchronous whole pyramid.
- **GPU (R6)** composites viewport tiles only where measurements show it is
  faster than the CPU tile path. CPU stays the oracle.

## Capabilities

### New Capabilities

- `compositing/region-composite`: `composite_region(rect, level)` and its
  identity with the whole composite and the halved pyramid.
- `compositing/tile-pyramid`: the per-tile display cache, its invalidation,
  budget, eviction order, viewport presentation and solo-mask source.

### Modified Capabilities

- `compositing/current-picture`: the picture is served from cached composite
  tiles and regions, not from a maintained whole composite.
- `compositing/view-pyramid`: the stored-level structure, borrowed whole-level
  views and supplied level 0 are replaced by tile requests.

## Impact

- `pictura-render`: `composite_region`, `TilePyramid`, streaming shrink;
  `ViewPyramid` storage removed (its halving kernels reused);
  `DocumentComposite` gains the "authoritative" rule.
- `pictura-core`: governor consumer for the display cache; `Document`
  composite authority helper.
- `pictura-codec`: PSD/PSB writer takes a merged-image band source.
- `pictura-select` / `pictura-paint`: Sample All sources read regions through a
  closure.
- `pictura-app`: display state (`level0`, `pyramid`, `mask_level0`,
  `mask_pyramid`, deferred pyramid) becomes one tile cache; the `picture()`
  accessor becomes region-based; history drops the composite plane; the memory
  gate's display estimates; the C++ canvas level provider.
- `docs/01-architecture/gpu-rendering-pipeline.md`: record the shipped tile
  pyramid (separate `TASK-ALLOWS-DOCS` commit).
- Closes #942. No new dependency and no file-format change.
