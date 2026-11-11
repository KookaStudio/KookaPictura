# Design

## Context

Since #1213 every layer, mask and selection plane is tiled. Since #1215 history
shares those tiles. The display, though, still holds whole-document buffers:

| Buffer | Owner | At 17000² |
|---|---|---|
| `doc.composite` (raw working-space RGBA, planar) | `Document`, kept current by the edit transaction, tracked by history | 1.16 GB |
| `level0` (straight sRGB RGBA, planar) | `PictureViewRust` | shares the composite's tiles in sRGB; forks on non-sRGB documents and live strokes |
| `ViewPyramid` levels 1…n (premultiplied RGBA, interleaved) | `PictureViewRust` | ≈ 385 MB (level 1 = 289 MB) |
| `mask_level0` + `mask_pyramid` (solo mask) | `PictureViewRust` | another level-0 frame and pyramid |

The canvas already reads pixels through a narrow seam, `ImageView::LevelProvider`:
it crops a rectangle at a level, reports level sizes and a revision, and takes a
zero-copy view of a whole stored level. The app's composite readers go through
one accessor, `picture()` (`cxxqt_object/picture.rs`). The render side writes
the composite through one facade, `DocumentComposite`. Those three seams are
where this change does its work.

`docs/01-architecture/gpu-rendering-pipeline.md` ("Tile-based rendering",
"Data-model impact") already specifies 256² tiles in an LRU keyed by
generation, evicted under the memory cap, with no undo record storing pixels the
pipeline produced. This change ships that for the CPU path.

## Owner decisions (2026-11-09)

- **Scope.** As in #1214, one PR covers the viewport, the solo-mask restore (R7)
  and the R4 readers: save/export, flatten, Sample All Layers and histogram.
  `doc.composite` becomes a lazily built cache the governor can drop. Every
  other reader stays on it, marked `ponytail:`, with a follow-up issue.
- **Authoritative composites stay stored.** A composite remains a real stored
  (tiled) plane only where it is the source of truth:
  - documents with no layers;
  - non-RGB documents (their composite keeps the document's own colour planes);
  - retained native-depth merged composites.
- **The sRGB `level0` frame is removed too.** Its tiles live in the pyramid
  cache.
- **#942 closes into this issue.** Hit and miss counters are reported through
  the memory census.
- **R7 budget.** At 16507×16196 with a 1920×1080 viewport, visible solo-mask
  presentation takes ≤ 50 ms on the GUI thread (704 ms today). Exact tiles
  finish on workers. Peak memory is recorded, with no limit set.
- **GPU** composites tiles only where measurements show it is faster. CPU stays
  the oracle.
- **Docs.** `gpu-rendering-pipeline.md` is updated in a separate
  `TASK-ALLOWS-DOCS` commit.

## Decisions

### 1. `composite_region(doc, rect, level)` (`pictura-render`)

- **Level 0** returns raw straight-alpha working-space RGBA for `rect`. It is
  `composite_rgba_region` (already byte-identical to a crop of
  `composite_rgba`, #1180), clamped to the canvas, with transparent padding
  outside it.
- **Level k > 0** returns premultiplied RGBA for a rectangle in level-k
  coordinates. It composites the level-0 rectangle `rect × 2^k`, clamped, and
  halves it k times with the existing `ViewPyramid` kernels
  (`shrink_from_planes`, `shrink_into`).

  The halving needs care at odd edges. Today level k+1 is built from the whole
  of level k, and a level with an odd size folds its last row and column with
  `half()` rounding. A sub-rectangle is therefore aligned to `2^k` on its
  origin. Its far edge is either `2^k`-aligned or the level-0 edge, so the
  kernels see exactly the samples a whole rebuild sees. The
  `composite_region_matches_full_crop` test pins this.
- **Large rectangles stream.** When `rect × 2^k` exceeds one band budget, the
  level-0 composite runs in bands of `2^k`-aligned rows. Each band is halved and
  appended to the output, so a fit-view tile of a 17000² document never holds
  more than a band of level 0.
- **Display conversion** (working space → sRGB) is applied per region by the
  caller, before premultiplication. For sRGB documents it is the identity and
  copies nothing. This matches today's `level0_from_buffer` order: convert,
  then build levels.

### 2. The tile pyramid cache (`pictura-render::tile_pyramid`)

```text
TilePyramid
  tiles: HashMap<Key, Entry>       Key  = (source, level, tx, ty), 256² in level pixels
  epoch: u64                       bumped by whole-document invalidation
  damage: Vec<u64> per level-0 tile, the damage counter at its last change
  lru:   ordered access stamps;  bytes: u64;  hits/misses: u64
Entry = Arc<[u8]> (256²×4 or the clipped edge size), made_at: u64 (damage counter)
source = Composite | Mask(layer id)
```

- **Tile formats.** A level-0 tile is straight sRGB, the source for halving and
  the Info readout. A tile at level k > 0 is premultiplied, as stored levels are
  today. A crop premultiplies level 0 on the way out, as now.
- **Validity.** A tile is valid when no level-0 tile under its footprint was
  damaged after `made_at`, within the current epoch. A worker result carries
  the epoch and the damage counter from when it was requested. A stale result
  is dropped, not installed: this is the existing
  `pyramid_generation`/`composite_generation` pattern, applied per tile.

  Layer-tile generations from #1213 are deliberately not used as the cache key.
  Effects, clipping and adjustment layers make a composite tile depend on
  neighbouring layer tiles. The damage rectangles every edit path already
  reports (including #1180's effect spread) are the correct and complete
  invalidation, and they are what the stored composite relies on today.
- **Damage updates in place.** `refresh_region` already composites the damaged
  rectangle. That rectangle is aligned out to 64, the `TILE` constant today,
  which covers six levels. The new path folds the region into the cached tiles
  at every level from that one buffer: it writes level-0 tiles and halves
  upward, patching only tiles already cached. Uncached tiles are not built,
  only marked damaged. This is `ViewPyramid::update` with the region buffer as
  level 0 instead of a stored frame, and it stays byte-identical to a rebuild
  (`vp_tile_update`).
- **Budget and eviction (R2).** The cache is a governor consumer
  (`Governor::display_budget`): `min(192 MiB, ceiling / 4)`, and never less
  than the pinned visible set. A put over budget evicts in this order:
  1. unpinned level-0 tiles, LRU (the most expensive to keep, cheapest to skip);
  2. unpinned higher levels, LRU.

  Under `Pressure::High` every unpinned tile is dropped before any other
  consumer is asked. The pinned set is the tiles of the last presented view and
  its one-tile margin.
- **Counters.** Hits, misses, evictions and resident bytes are reported with the
  memory census. This is #942's accounting.

### 3. Viewport presentation

- `canvas_display_image(level, rect)` assembles the requested crop from tiles.
  On a miss, the GUI thread fills the hole from the nearest cached coarser tile
  of the same source, upscaled by nearest-neighbour as the coarse stand-in does
  today (`ViewPyramid::coarse`), and queues the exact tiles on a worker. Each
  arriving batch bumps `canvas_revision` and emits `region_damaged` for its
  rectangle.
- When nothing coarser is cached (first paint after a large open), the first
  paint shows transparency and starts the worker for the visible tiles; their
  build is the work the open used to do on its own thread (one pass over the
  whole composite for a fit view), now without the full-resolution buffer.
  Pre-building them on the open's worker was planned and dropped: the canvas's
  own request starts the same build at the same moment, and M13 measures
  whether the first exact paint keeps to the baseline.
- **Small documents** (≤ 16 M pixels, the existing `DEFERRED_PIXELS`) build
  missing visible tiles synchronously, so tests and small files see no stand-in
  frames. That keeps `tst_canvas_*` deterministic.
- **`levelImage`** (whole stored level, zero copy) is removed. `ImageView`
  already falls back to `crop` when it is empty, and its present cache keys on
  `canvas_revision`. `display_level_count` and `display_level_size` keep their
  meaning (the halving rule is unchanged).
- **Live strokes and previews** keep their current paths: `refresh_region` of
  the stroke document, the display preview's reduced-level patch, and the
  stroke tile present. They now write into tiles instead of `level0` and the
  levels. The display preview's stored-level patch (`patch_level`) becomes a
  per-tile patch into an overlay map that is dropped when the preview ends, as
  today.

### 4. The current picture and `doc.composite`

- `Document::composite_is_authoritative()` is true for a layerless document, a
  non-RGB mode, or `source_planes` holding a merged composite. Only those
  documents keep `doc.composite` stored, and history tracks it only for them.
- For every other document, the edit transaction no longer stores or patches
  `doc.composite`. Its "refresh" goes to the tile cache, and the field is left
  as an empty `0×0` buffer, so `is_shape_valid` is false.
- `picture()` becomes a region API:
  - `picture_region(doc, view, rect) -> PixelBuffer` reads raw level-0 tiles
    from the cache, compositing misses.
  - `picture_bands(doc, view, rows, f)` streams the whole picture.

  The current-picture rules (live stroke, lifted transform, display preview,
  deferred Move composite) still pick between the cache and a fresh composite
  of the layer stack. The fresh branch now composites the region, not the
  whole document.
- `materialize_composite(doc)` fills `doc.composite` for the `ponytail:`
  readers. It is counted like `as_slice` and dropped by the next edit or by the
  governor.

### 5. Reader inventory (`.composite` outside tests: 325 references)

| Group | Files (refs) | Treatment |
|---|---|---|
| Composite facade | `pictura-render/document_composite.rs` (62) | Rewritten: `store`/`patch`/`refresh` feed the cache for layered docs and the stored plane for authoritative ones |
| Viewport and display | `impl_core.rs` (9), `impl_core/display.rs`, `helpers_composite.rs` (5), `display_preview.rs` (6), `impl_history.rs` (3), `deferred_pyramid.rs`, `quick_mask` | **Migrated** onto tiles |
| Current-picture accessor | `picture.rs` (17) | **Migrated** to the region API |
| Sample All Layers | `impl_selection.rs` (19: wand, Grow, Similar, channel load), `paint_tools/imp.rs` (bucket, clone, smudge, mixer, focus), eyedropper `impl_core.rs` | **Migrated**: tiled region sources |
| Histogram and Info | `histogram_panel.cpp` (coarsest level), Info `sample_planar_argb` | **Migrated**: cached tiles |
| Save | `pictura-codec/write.rs` (11), `write_cache.rs` (8) | **Migrated** for layered RGB: merged image from a band source, cache keyed by the cache epoch and damage counter. Authoritative composites (indexed, duotone, bitmap, CMYK, Lab, native depth) keep writing the stored plane |
| Flatten | `document_ops/layer_ops` | **Migrated**: band composite into the new layer's tiles |
| Raster export | `export/imp.rs` | `ponytail:`: the Qt encoders take one `QImage`. One packed copy for the export's lifetime, nothing retained |
| History | `history.rs`, `history/planes.rs` (5), `history/budget.rs` (6) | Composite plane tracked only when authoritative |
| Whole-document ops | `document_ops/{crop,mode,depth,orient,resize,canvas,rotate_in,lens_correction,smart_object}` (≈ 80) | `ponytail:`: these transform or rebuild the document, then invalidate the cache whole. They operate on a stored composite only when authoritative. Layered documents skip the composite and recomposite through the cache afterward |
| Colour conversion | `pictura-codec/{icc,icc_convert,color_mode,read,crs_xmp}` | Unchanged: they act on authoritative composites (open, convert) or on a stored composite they then invalidate |
| Filters and transforms | `filter_tools/imp.rs` (5), `displace_maps.rs`, `impl_transform/mod.rs` (4), `filter_proxy.rs`, `smart_filter.rs`, `vanishing_point` | `ponytail:`: `materialize_composite` where they need the whole picture as a base |
| Memory estimates | `memory_gate.rs`, `annotations/imp.rs` | Updated to the new display model |
| Tests and fixtures | `edit.rs` tests, `canvas_view_test.rs`, examples | Assert against `composite_rgba`/`composite_region` instead of the field |

### 5a. Decisions made while implementing

- **Stale tiles are stand-ins.** Damage and whole-document invalidation mark
  tiles stale instead of dropping them. A stale tile is never served as
  current (crops report it missing, `picture_region` ignores it), but the
  canvas shows it until its rebuild lands, and eviction takes stale tiles
  first. Without this, any full display reset (cancelling a preview,
  toggling Quick Mask, undoing a whole-canvas edit) would blank a large
  document's view, because no stored composite is left to rebuild from
  cheaply.
- **Region refresh policy.** A refresh that would composite more than
  4 M level-0 pixels on a document above 16 M pixels marks the tiles stale,
  and only the visible ones are rebuilt on a worker. Smaller refreshes and
  every refresh on a small document composite the region aligned to the
  repair grid and repair the cached tiles at every level in place.
  Regions produced elsewhere (filter and adjustment workers, history)
  are widened to that grid by compositing only the border strips they lack.
- **Metadata-only restores.** History bounded an opacity, blend, visibility
  or position restore by the stored composite's changed tiles. A document
  without a stored composite has none, so a restore that changes layer
  metadata reports `Everywhere` (`unbounded_meta_change`). This is
  `ponytail:`: the changed layers' influence rectangles would bound it.
- **Open skips the merged image.** `OpenOptions::skip_layered_merged`, set
  by every application open, skips decoding the colour planes of an 8-bit
  RGB file with layers and RLE or raw image data. Its extra channels are
  still decoded. The codec's own readers keep the full decode.
- **Conversions materialize.** Mode conversions (Grayscale, CMYK, Lab) and
  depth up-conversion read the RGB composite as their source, so they
  materialize it band by band on entry (`materialize_composite`,
  `ponytail:`). The converted document keeps it, because its composite is
  then authoritative. The Indexed Exact-palette check streams the composite.
- **`Document::with_uniform_composite`.** New Document starts its composite
  as uniform tiles. `Document::new` allocated a flat canvas-sized buffer
  that was replaced at once (72 MiB at 4250²).
- **`image()` stays whole** (`ponytail:`). It is composited on demand, not
  through the cache. Its callers are the capture command, small-document
  dialogs and tests.

### 6. History

Nothing changes in history's plane tracking: a layered document's composite
is an empty plane, so history holds no pipeline pixels for it, and an
authoritative composite is tracked as before. Undo, redo and jumps already
report per-tile damage (#1215), and that damage now repairs or invalidates
cache tiles (see 5a for metadata-only restores). A named-snapshot restore invalidates the epoch:
the visible tiles are recomposited on workers. This replaces #1215's "restore
presents the snapshot's exact composite", whose stored composite is what M8's
remaining 359 MiB was. M8's ≤ 100 ms GUI-thread bound still holds, because
the GUI thread installs the stand-in and returns.

### 7. Solo mask (R7)

`Source::Mask(layer)` tiles are built from the mask plane per tile: grayscale
to opaque RGBA, then halved for levels > 0. No document-sized
`mask_level0` is built and no exact pyramid is built synchronously. A restore
while soloed shows the coarse stand-in, which is point-sampled from the mask
plane directly (cheap: a tiled plane read per output pixel), and queues the
exact visible tiles. Leaving solo drops the mask tiles. The composite tiles are
unaffected.

### 7a. Tile builds run one at a time

A worker builds the missing tiles of a request one after another. Each
tile's composite already runs on every core; building them side by side held
one level-0 chunk per tile at once, over a gigabyte at a 17000² fit view
(M1 peaked at 1218 MiB before this, 93 after).

### 8. GPU (R6)

The CPU tile path ships first and is the oracle. The GPU backend already
composites regions (`CompositeRequest`). After the CPU path is measured, viewport
tile misses go to the GPU when `gpu_compute` is on, **only if** M13's frame time
and first paint improve. The parity tests' existing tolerances apply. The result
is recorded in the measurements table below.

## Budgets and acceptance (17000² unless noted)

Measured 2026-11-10 on this branch: AMD Ryzen 7 255 (16 threads), 29 GiB, Qt 6.10.3, `dev-app`
build, `scripts/profile-tools.py --peak` under the default limit (MemoryMax
7720M); MiB added at peak over the resident size at the reset.

| Scenario | Bound | Measured |
|---|---|---|
| M1 New Document, 17000² | no 1.16 GB composite; open/new ≤ layer tiles + 256 MiB | 93 (resident 242); was ~1218 before the tiles were built one at a time |
| M2 Add Layer, 17000² | as M1 | 0 |
| M5 Bucket, 17000² | re-measured (display refresh) | 89; #1214 left ~1100 |
| M7 Undo ×3 / Redo ×3, 17000² | re-measured | 104; #1215 left 2206 |
| M8 Snapshot restore, 17000² | re-measured | 48; #1215 left 359 |
| M11 Save PSD, 17000² | ≤ layer tiles + 128 MiB | 77 |
| M12 Open 4-layer PSD, 17000² | ≤ layer tiles + 256 MiB | refused: the fixture decodes to ~5.5 GB of planes, over the reader's budget on this host, as in the #1210 baseline |
| M12 Open 4-layer PSD, 12000² | as above | **2800 (resident 1165): not met.** The peak is the PSD decode (the whole file read, layer channels decoded before tiling), not a composite; the merged image is no longer decoded. Follow-up #1273 |
| M13 Pan/zoom, 17000² | frame time within 10 %; first paint ≤ baseline | setup 0 MiB; frame time needs a real display (manual) |
| Solo-mask restore, 16507×16196 (`structural_restore_profile`) | visible presentation ≤ 50 ms GUI thread | **0.02 ms** GUI thread (was 704 ms); 72 visible level-3 tiles on a worker 411 ms |
| Whole-document restore, 16507×16196 | recorded | 3.3 ms GUI thread; 72 visible tiles 330 ms on a worker |
| Solo mask, 8000² (`solo_mask_profile`) | recorded | 0.02 ms GUI thread; 64 visible level-2 tiles 95 ms (the old whole plane + pyramid: 67 + 54 ms) |
| Heap guards, 4250² | at or below baseline | New Document 103.4 → 0.5 MiB, Undo/Redo 137.7 → 0.1, Snapshot restore 22.0 → 0.1, whole-layer Bucket 69.0 → 0.2; Magic Wand 0.3 → 1.8 (tiles composited on demand instead of a retained 72 MiB composite) |

The issue's before-values come from the #1214 and #1215 records and the #1177
audit; a same-machine master build for a side-by-side run was not made. GPU
tile compositing was not compared: the CPU path ships, and the GPU stays on
the whole-frame paths it already accelerated.

## Risks

- **Odd-edge halving.** A region at level k must reproduce the whole rebuild's
  folding of odd rows and columns. `composite_region_matches_full_crop` and a
  property test over odd sizes cover it.
- **Missed damage.** A path that changed pixels without reporting damage used to
  be hidden by a whole-composite `store`. It now shows as a stale tile. The
  `cp_identical` test compares every served state against a full composite, and
  a debug-build check compares each newly built tile with its damage counter.
- **First paint on open.** A fit view still needs one pass over the whole
  composite. It is streamed, so memory falls while time stays at the baseline.
  M13 guards it.
- **Ponytail whole-document ops** keep today's peak (they materialize). None of
  them is an M-scenario. Follow-up #1274 lists them.

## Migration order

1. `composite_region` and its identity tests.
2. `TilePyramid` and its governor consumer, with unit tests.
3. Viewport: present path, damage folding, workers, open pre-build; remove
   `level0`, `ViewPyramid` storage and `levelImage`.
4. Solo mask on `Source::Mask`.
5. `picture()` to regions; Sample All, Info, histogram.
6. Save band source; flatten.
7. Stop maintaining `doc.composite` for layered documents; history; ponytail
   materialize; memory gate.
8. GPU measurement; docs; measurements; follow-up issue.
