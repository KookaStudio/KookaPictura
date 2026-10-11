# Proposal: large-image-filter-preview

## Why

Issue #152: editing a filter's parameters and applying it on a very large image
is extremely slow. Profiled on a document the size of the 16507×16196 world map
(267 MP, one layer, CPU path, `large_document_filter_profile`):

| step | before |
| --- | ---: |
| each dialog thumbnail update or dialog open (a full-resolution `QImage`) | 0.3–0.6 s |
| preview at 100% zoom (1280×800 visible) | 0.7–1.1 s filter + 1.5 s refresh |
| preview at fit zoom, Gaussian Blur radius 4 | 20 s + 1.5 s refresh |
| preview at fit zoom, Median radius 2 | 110 s + 1.5 s refresh |
| commit, Gaussian Blur radius 4 | 20 s + 1.5 s refresh + 1.5 s history |

Four things made it slow. The layer-filter wrapper wrote results back one f64
sample at a time on a single thread, and reran the whole kernel over an opaque
layer's alpha. Several CPU kernels (Median, Motion Blur, Box Blur, Maximum and
Minimum, Custom, the Unsharp Mask and High Pass combines) ran on one thread.
Every preview copied whole planes and refreshed the whole layer, however small
the visible section was. And at fit zoom the visible section is the whole
document.

photocraft previews large documents on a reduced proxy and filters at full
resolution only on commit (`ui-egui/src/filter_dialog.rs`, `proxy.rs`); the
Filter Gallery already does this (#267). This change applies it to the filter
dialog below 100% zoom, and removes the whole-plane costs everywhere else.

## What Changes

- The layer-filter wrapper (`pictura-render::filter`) crops and writes back
  row-parallel. Without a mask or a transparency lock it copies the filtered
  rows straight in. It skips the alpha pass when the alpha crop is fully
  opaque, which is exact because every moving kernel maps a flat opaque plane
  to itself.
- Gaussian Blur accumulates whole rows per tap, so it vectorizes. Median,
  Motion Blur, Box Blur, Maximum, Minimum, Custom, High Pass and the Unsharp
  Mask combine run row-parallel. All of them stay byte-identical to their
  serial versions.
- Layer filters skip the GPU for kernels whose parallel CPU path now beats the
  GPU round trip. Gaussian Blur, Blur, Blur More, Sharpen, Sharpen More, Unsharp
  Mask, High Pass, Maximum, Minimum, Median and Custom run on the CPU. Box,
  Motion and Surface Blur and Oil Paint keep the GPU.
- A section preview saves and restores only its section, writes the live
  planes in place, and refreshes only what it and the last preview touched.
- Below 100% zoom, a preview covering more than 4 MP of document is drawn as an
  overlay on the canvas. A draft appears at once: a reduced copy of the
  visible region, filtered with its pixel-sized parameters scaled to match.
  A worker then filters the full-resolution layer, and its reduced result
  replaces the draft, so what the canvas settles on is what OK will produce.
  The dialog's pane renders its own section exactly, off the document. The
  document is not touched until OK.
- OK, Last Filter and the Filter Gallery commit on a worker thread. The
  worker filters the layer (or reuses the exact preview's), composites the
  changed region and captures history. Input waits, and an animated busy window
  appears if the commit takes longer than a quarter second.
- History names the filter: "Filter: Crosshatch", or "Filter Gallery: Grain,
  Texturizer" for a gallery stack.
- History keeps each changed tile once and swaps it on undo/redo, halving what
  a full-layer filter costs in memory.
- Radial Blur samples in tiles with shared taps (Spin 30–45 s → 5.6 s on an
  8000² image), and its Zoom runs on the GPU.
- The filter dialogs never build a full-resolution image. The pane crops level
  0, and the Placement pad, Lens Flare and Lighting Effects read a reduced
  pyramid level.
- Parameter changes that arrive while a preview runs coalesce into one preview.
- Maximum and Minimum show CS6's "pixels" unit, and Offset shows "pixels right"
  and "pixels down".

## Capabilities

### New Capabilities

- `document/single-sided-history`: deltas that keep one side of each changed
  tile.
- `imaging/filter-dialog-preview`: section previews that touch only their
  section, the zoomed-out draft and exact preview, no full-resolution reads,
  coalesced changes, background commits, and named history states.
- `imaging/parallel-filter-kernels`: row-parallel kernels and write-back that
  stay byte-identical.

### Modified Capabilities

- `compositing/gpu-filter-acceleration`: default-on GPU filters, except the
  kernels the CPU now runs faster.

## Impact

- `pictura-filters` (`kernel.rs`, `blur.rs`, `noise.rs`, `other.rs`,
  `sharpen.rs`), `pictura-render` (`filter.rs`, `filter_proxy.rs`),
  `pictura-app` (`impl_filters.rs`, `filter_tools.rs`,
  `filter_tools/canvas_proxy.rs`, the new `filter_tools/jobs.rs` bridge,
  `state.rs`, `build.rs`), and the shell (new `filter_commit.*`,
  `filter_preview_dialog.*`, `filter_gallery_dialog.cpp`, `frame_session.cpp`,
  `filter_param_controls.*`, `filter_commands.cpp`,
  `image_view*`, `dialogs.*`, `lens_flare_dialog.cpp`,
  `lighting_effects_dialog.cpp`, `frame_menus_filter.cpp`).
- No new dependencies; `rayon` is already in all three crates.
- Commits stay byte-exact. The zoomed-out canvas shows an approximate draft
  only until the exact picture arrives.

## Provenance

The reduced-copy preview follows photocraft (same author); no code is copied.
