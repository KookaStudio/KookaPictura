# Design

## Decisions (owner, 2026-11-14)

- **One change, one pull request** for all five reader groups, with one commit
  per group (as #1214 and #1216 did).
- **A whole output keeps its one buffer.** Some readers produce a whole-document
  result by nature: the Qt and WebP encoders take one packed RGBA image, the
  clipboard holds one image, a selection is document-sized, and the Vanishing
  Point editor holds a document-sized backdrop. These keep that one output
  buffer. What goes is the composite they were built from, plus the planar,
  sRGB and flat intermediate copies. The encoders are not made streaming.
- **Previews may read a reduced level.** A reader that only feeds a preview reads
  the composite at the coarsest pyramid level that still covers what it shows.
  The preview is approximate, like the #152 filter preview proxy. Every
  committed result stays byte-identical to today's.
- **Conversions stream their source only.** The converted document keeps its own
  authoritative stored composite. Extending the #1216 "no stored composite" model
  to layered Grayscale, CMYK and Lab documents is out of scope.

## The three read shapes

Every migrated reader uses one of three existing primitives. No new compositor
entry point is added.

1. **Bands.** `pictura_render::stream_composite(doc, rect, rows, f)` hands over
   full-width bands in row order. Readers that are per pixel or accumulate in
   pixel order use it: export encoders, Trim, Duplicate (merged), Load Channel
   Selection, Color Range commit, Match Color statistics, conversions and the
   Vanishing Point backdrop. A band arrives in the same row-major order as the
   whole buffer. So an accumulation in f64 (Match Color) sums in the same order
   and stays bit-identical.
2. **Tiles on demand.** The Sample All fetch from #1216
   (`picture::sample_tiles_from`, a per-tile composite with a 64-tile memo) is
   wrapped as a `TileSource` read by `pictura-select`. The readers that touch
   pixels sparsely or in neighbourhoods use it: the wand flood, Grow, Similar,
   Refine Edge's edge band and decontaminate, and the Magnetic Lasso. The
   selection algorithms already work tile by tile over a `PixelBuffer`
   (#1214). The buffer argument becomes a small enum: `Buffer(&PixelBuffer)`
   for existing callers and tests, and `Tiles(fetch, w, h)` for the app. A
   tile it reads is the same bytes as that crop of the whole composite, so the
   results are identical. One stored-composite case keeps the `Buffer` form,
   because borrowing it costs nothing: a layerless or other authoritative
   document.
3. **Reduced level.** `region_level` over the document composite at level `k`,
   where `k` is the largest level whose size still covers the preview's
   `max_side`. It streams level 0 in chunks, so its peak is one chunk plus the
   level. Readers that use it: the Color Range, Refine Edge and Replace Color
   previews, and `PictureView::preview_image(max_side)` (control-server capture
   and the Image Size preview).

**Move and transform backdrops.** The canvas scales the base and the moving
image by one factor, and the reduced-level transform path excludes groups,
effects and rendered layers on purpose. A reduced base alone is therefore not
an option. Instead, `backdrop_patch` stops refusing effects, adjustments and
groups. It patches the union of `pictura_render::damage::influence` over the
hidden paths, which is the set of pixels whose value can depend on them, and
which is already proven for history damage. This is exact at level 0 and needs
no C++ change. A whole backdrop remains only where the influence is unbounded
(a knockout in the document) or the canvas does not show the document as it
stands (`bounded` false). An adjustment layer's influence is its whole canvas,
so moving one keeps a canvas-sized backdrop. That case stays `ponytail:`.

## Per reader

| Reader | Today | After |
|---|---|---|
| Raster export, Save for Web, save formats | `picture()` → `buffer_to_srgb` → packed RGBA | bands → sRGB per band → one packed RGBA |
| Copy Merged | whole composite → `copy_merged` crops to selection bounds | `picture_region(selection bounds)` passed to a region form of `copy_merged` |
| Duplicate (merged) | whole composite → RGBA → `Document::from_rgba` | bands → the one RGBA → `from_rgba` |
| Trim | whole composite → RGBA → alpha scan | bands → alpha bounds; nothing kept |
| Load Channel Selection | whole composite → gray mask | bands → mask rows |
| Grow, Similar, wand action | `&picture()` | `TileSource::Tiles` |
| Color Range commit | whole composite → per-pixel mask | bands → mask rows |
| Color Range / Refine Edge preview | `shrink(&picture(), max_side)` | `shrink(&level_k, max_side)` (approximate) |
| Refine Edge commit, decontaminate | `&picture()` | `TileSource::Tiles` (reads only the edge band) |
| Replace Color preview | nearest from whole composite | nearest from level `k` (approximate) |
| Match Color source stats | whole composite → `RgbaImage` | bands → a running `MatchColorStats` accumulator |
| Magnetic Lasso | `EdgeMap::from_buffer(&picture())` | peak over bands; tile costs through `TileSource::Tiles` on a document clone |
| `PictureView::image()` callers | whole level-0 image | `preview_image(max_side)` at level `k`; `image()` stays for tests and small documents |
| Grayscale / CMYK / Lab / depth up-conversion | `materialize_composite` then flat `Vec` copies | bands written straight into the gray plane, the encoded store and the shown composite |
| Liquify backdrop | whole hidden-layer composite, cropped to the frame | the frame composited with the layer hidden |
| Vanishing Point backdrop | whole hidden-layer composite → sRGB → RGBA | hidden-layer bands → sRGB → the one RGBA |
| Move / transform whole-backdrop fallback | whole hidden-layer composite at level 0 | a patch over `damage::influence` of the hidden paths (exact); whole only when unbounded or not `bounded` |

**Magnetic Lasso.** The edge map lives across the whole gesture, so it cannot
borrow the document. It keeps a `Document` clone, which is cheap because the
planes are refcounted tiles (`impl_selection` already does this for Alt). A
tile's costs read that tile plus a one-pixel border through the fetch. The peak
pass streams bands with a one-row overlap and visits rows in the same order, so
`f32::max` gives the same value.

**Conversions.** For an 8-bit layered RGB document, the composite is read once,
in bands of 256 rows, before any store is reset, which is where
`materialize_composite` used to run. Grayscale writes a one-channel tiled plane.
CMYK and Lab encode each band into the flat encoded store and write the RGBA they
display as into a tiled plane. Depth up-conversion writes the RGBA composite and
its 8-bit RGB store. A profile conversion's ordered dither indexes the whole
plane with a period of 8 samples (`dither_plane(plane, n)`). Every band starts
at a multiple of 256 rows, so a band's samples keep their index modulo 8 and
banded dithering is exact too. `materialize_composite` remains only for a
conversion that does not stream.

## Measurement

The acceptance rule is "no canvas-sized composite allocation in the migrated
paths". The heap guards in `tests_profiles/memory.rs` already measure a
scenario's peak over its setup at 4250², where one RGBA composite is 68.9 MiB.
Each migrated reader gets a guard of the same shape. The assertion is
`peak < output bytes + one RGBA composite / 2`. The output is zero for scans,
statistics, the edge map and previews, the packed RGBA for export, and the mask
for selections and the gray plane. A whole composite cannot fit in that half,
while a band does, including the compositor's f32 band canvas (16 bytes a
pixel). To keep a band small, the readers stream one tile row (256 rows) per band
instead of `band_rows`' roughly 8 M pixels. The Grayscale guard uses a document
with two small layers, so the out-of-scope layer-plane conversion does not
dominate the peak. The
guards run under `alone_in_process`, as the existing ones do. A 17000² profile
run is recorded in the PR (manual row).

## Risks

- **Byte identity of tile-sourced selections.** A selection read through tiles
  must equal the same selection read through the whole buffer. Each migrated
  `pictura-select` function gets a property test that runs both sources over
  seeded documents at odd sizes.
- **Memo thrash.** A Similar or Color Range pass over a 17000² document reads
  each tile once in grid order. The 64-tile memo is enough, because the passes
  are tile-ordered. Grow's flood revisits rows, but #1216 measured that pattern
  for the bucket.
- **Time.** A tile-sourced reader composites each tile it touches. When the tile
  cache holds the level-0 tiles, `picture_region` takes them from the cache, as
  today. The fetch also tries the cache first.
- **Previews change.** The Color Range, Refine Edge and Replace Color previews
  at large sizes are nearest-sampled from a reduced level instead of from level
  0. Small documents (level 0) are unchanged.

## Out of scope (stays `ponytail:`)

- Refine Edge's own document-sized f64 work arrays.
- The clipboard's selection coverage copy.
- The C++ `ImageView::image()` crop used by the crop tool's content pan.
- Streaming encoders.
