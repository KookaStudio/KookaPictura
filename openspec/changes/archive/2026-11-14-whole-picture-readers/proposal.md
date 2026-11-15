# Proposal

## Why

Issue #1274 follows up on #1216. Since #1216, a layered RGB document keeps no
stored full-resolution composite, and the viewport, save, flatten, Sample All
Layers, the eyedropper, Info and the histogram read regions or tiles. The
remaining readers are marked `ponytail:`. Each still builds a document-sized
composite while it runs, which is 1.16 GB at 17000²:

- **The whole `picture()` accessor**, used by raster export, Save for Web, raster
  save formats, Copy Merged, Duplicate (merged), Trim, Grow, Similar, the wand
  action, Load Channel Selection, Color Range, Refine Edge, Match Color, Replace
  Color and the Magnetic Lasso. Export holds three canvas-sized copies: the
  planar composite, its sRGB conversion and the packed RGBA.
- **`PictureView::image()`**, used by the control server's document capture and
  the Image Size dialog preview. Both scale the result down.
- **Mode and depth conversions**, which materialize the RGB composite and then
  copy it into flat whole-document vectors.
- **The Move and transform whole-backdrop fallbacks**, and the Liquify and
  Vanishing Point backdrops (`hidden_layer_buffer`). These test `doc.composite`,
  which a layered document no longer stores. Since #1216 they therefore always
  composite the whole document.

## What Changes

- **Exact streamed readers.** Raster export, Save for Web, raster save formats
  and Duplicate (merged) composite band by band straight into their one output
  buffer. Trim scans coverage band by band. Copy Merged reads only the selection
  bounds. Load Channel Selection writes the mask band by band.
- **Tile-sourced selection readers.** Grow, Similar, the wand action, Color Range
  (commit), Refine Edge (commit and decontaminate), Match Color statistics and
  the Magnetic Lasso read the composite one tile at a time through the existing
  Sample All tile fetch. Their results stay byte-identical.
- **Reduced-level previews.** The Color Range, Refine Edge and Replace Color
  dialog previews, the control-server document capture and the Image Size
  preview read a reduced pyramid level no larger than they show. These previews
  are approximate. Committed results are not.
- **Streamed conversions.** Grayscale, CMYK and Lab conversion and depth
  up-conversion read their RGB source band by band into the converted document's
  stores. The converted document keeps its authoritative composite.
- **Region backdrops.** The Liquify backdrop composites its frame only. The Move
  and transform backdrops patch the hidden layers' proven influence, for
  groups, effects and adjustments too, instead of falling back to the whole
  document. Vanishing Point streams the backdrop into its one sRGB output.
- Heap guards at 4250² assert that no migrated path allocates a canvas-sized
  composite.

## Capabilities

### New Capabilities

- `compositing/whole-picture-readers`: how a reader that needs the whole picture
  reads it without a canvas-sized composite. It covers streamed exact readers,
  tile-sourced selection readers, reduced-level previews, streamed conversions
  and region backdrops.

### Modified Capabilities

None. `compositing/current-picture` already requires read-only consumers to
read regions or streamed bands where they can. This change closes its remaining
exceptions without changing the requirement.

## Impact

- `pictura-select`: wand, Grow, Similar, Color Range, Refine Edge, decontaminate
  and the Magnetic Lasso edge map read through a tile source as well as a
  buffer.
- `pictura-adjust`: Match Color statistics accumulate band by band.
- `pictura-render`: mode and depth conversions stream their source.
- `pictura-app`: the `picture()` callers, `image()`, `hidden_layer_buffer`, the
  Move and transform fallbacks, and the Image Size and control-server previews.
  New heap guards.
- No new dependency and no file-format change.
