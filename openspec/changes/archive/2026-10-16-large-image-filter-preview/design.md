# Design: large-image-filter-preview

## Measured on the world map

The test document is a 16507×16196 single-layer RGB document (267 MP) on 16
cores and an integrated AMD 780M, in a release build. Numbers come from
`large_document_filter_profile` (`cargo test -p pictura_app --release --lib
large_document_filter_profile -- --ignored --nocapture`; set
`PICTURA_PROFILE_GPU=1` for the app's default GPU setting).

| step | before | after |
| --- | ---: | ---: |
| dialog open / pane update (full-resolution `QImage`) | 0.3–0.6 s | a level-0 crop |
| first preview at 100% (copy-on-write fork of the layer) | 0.46 s | 5 ms |
| re-preview at 100% | 0.7–1.1 s + 1.5 s refresh | 4–20 ms + 5–10 ms |
| preview at fit, Gaussian radius 4 | 20 s + 1.5 s | 13–35 ms per change (copy built once in ~0.12 s) |
| preview at fit, Median radius 2 | 110 s + 1.5 s | as above |
| commit, Gaussian radius 4 | 20 s | 2.4 s |
| commit, Unsharp Mask | 23 s | 3.5 s |
| commit, Median radius 2 | 110 s | 5.1 s |

The exact fit-zoom preview is still available with no canvas, or at 100% and
above. With the new kernels it costs Gaussian 2.4 s, Unsharp Mask 3.6 s and
Median 5.2 s, each plus a 1.5 s refresh.

### Where the old time went

- `apply_op_region` cropped the layer serially, then wrote every result sample
  back through an f64 blend with a coverage lookup, on one thread. For the
  Gaussian Blur that was about 13 of the 20 s; the kernel itself took 3.2 s.
- An unlocked layer reran the kernel over a grey copy of its alpha. On an
  opaque layer that pass is a no-op that costs as much as the colour pass.
- Median, Motion Blur, Box Blur, Maximum, Minimum and Custom ran on one thread.
  Median alone took 35 s at this size.
- Every re-preview reset the layer to an `Arc` clone of the baseline. The
  filter's first write then forked each plane, copying 1 GB, and the refresh
  covered the whole layer rect (1.5 s) even for a 1280×800 section.

### GPU against CPU at 8192² (64 MP)

| kernel | GPU | parallel CPU |
| --- | ---: | ---: |
| Gaussian radius 4 | 0.91 s | 0.46 s |
| Gaussian radius 60 | 2.1 s | 2.0 s |
| Unsharp Mask | 0.98 s | 0.63 s |
| High Pass radius 10 | 1.05 s | 0.72 s |
| Maximum radius 3 | 0.87 s | 0.37 s |
| Median radius 2 / 10 | 1.3 s / 11.2 s | 1.2 s / 3.4 s |
| Custom | 0.98 s | 0.96 s |
| Box Blur radius 10 | 1.05 s | 2.9 s |
| Motion Blur 25 | 0.99 s | 1.2 s |
| Surface Blur radius 5 | 1.3 s | 2.0 s |
| Oil Paint | 1.2 s | 68 s |

On this integrated GPU the device shares system memory, so the round trip is
host copying: about 0.9 s at 64 MP whatever the kernel. Layer filters
(`pictura-render::filter`) now skip the GPU for the first group.
`gpu_filter::apply_filter_active` still uses the GPU whenever a kernel exists,
so the parity tests keep testing every GPU kernel.

## Decisions

- **Exact where it is cheap, a proxy where it is not.** The owner chose
  photocraft's proxy for zoomed-out previews. A preview over more than 4 MP of
  document below 100% zoom is drawn on the canvas from a `FilterProxy`: the
  visible region, box-averaged to the canvas's device pixels and capped at
  4 MP. Positional filters (Diffuse, Shear, Lens Flare, Lighting Effects, HDR
  Toning) use the whole document as the region. The copy is built from the
  pre-preview pixels and cached by region, scale and `content_revision`, so a
  parameter change filters only the copy. `ImageView` draws it over the
  document rect; the document is untouched until OK.
- **Scaled pixel parameters.** A slider whose unit is "pixels" (or "px") is
  multiplied by the copy's scale, rounded on an integer slider and held to its
  range. Other size-like parameters (artistic stroke lengths, cell sizes) act
  at the copy's scale, as the Filter Gallery's do.
- **The pane stays exact.** In proxy mode the pane's own section (200 document
  pixels at 100% pane zoom) is previewed on the document with the existing
  section path, so its 1:1 view is the commit's pixels. A positional filter's
  pane crops the copy instead.
- **Section baselines.** A section preview saves a deep copy of its section
  (`PreviewBaseline::Section`) instead of an `Arc` clone of the layer. The live
  planes are not shared, so the filter writes them in place. Restoring copies
  the section back. A whole-layer preview still keeps the layer clone.
- **Coalescing.** A value change starts a zero-delay single-shot timer, so
  changes queued behind a slow preview run as one. Two Qt tests now wait for the
  preview (`QTest::qWaitFor`) instead of reading it in the same call stack.

## Feedback round: background commits and the exact preview

The owner reported three problems on the map. History said only "Filter".
Applying a filter froze the app for seconds with nothing on screen. And the
zoomed-out draft of an artistic filter (Accented Edges, edge width 9) looked
nothing like the result: a 9-pixel stroke drawn on a copy at 1/13 scale cannot
stand for 9-pixel strokes averaged down.

- **History names the filter.** The shell passes "Filter: <menu name>" (and
  "Filter Gallery: <names>") as the label.
- **Commit on a worker.** `filter_tools/jobs.rs` filters a clone of the layer,
  composites the layer's rect, patches the composite and captures history on a
  worker thread. The worker owns the `History` while it runs; the panel
  refresh skips while a commit holds it. The UI thread runs a local event loop
  that excludes user input and shows a busy window after 250 ms, then swaps
  the document and history in and patches level 0 and the pyramid. The blit
  that follows carries a one-pixel image: a document canvas only re-crops.
- **Exact picture after the draft.** Each zoomed-out preview also asks a
  worker for the full-resolution result reduced to the draft's region and
  scale. One worker runs at a time and only the latest request is kept. A
  commit of the same filter installs that worker's layer instead of filtering
  again. The dialog's pane renders its section at scale 1 off the document, so
  the document is never written while the canvas shows a copy.

8000×8000 single layer, GPU on, from the same profile:

| step | worker | UI thread when it lands |
| --- | ---: | ---: |
| commit, Gaussian radius 4 | 1.4 s | 0.12 s |
| commit, Accented Edges 9/38/5 | 4.2 s | 0.11 s |

On the map, measured while the machine was using 30 GB of swap, the same
commits took 323 s and 196 s on the worker and 4.7 s and 3.5 s to land. That
is memory thrash, not compute. A full-layer commit on the map holds about
10 GB at its peak. That is the filtered layer, the composite region and its
sRGB copy, a copy-on-write fork of the 1 GB composite, and the history delta.
The delta keeps both the before- and after-bytes of every tile, about 3.6 GB
for one filter, retained up to 20 deep.

## Second feedback round: single-sided history and Radial Blur

The owner reported that a full Radial Blur on `earth_xtralarge.jpg` (8000×8000)
took 30–45 s to apply.

- **History keeps each changed tile once.** A delta used to hold every
  changed tile twice, before and after. Now it holds the side the
  materialized state it is walked from does not: before-bytes for states at or
  below the cursor, after-bytes above. Capture copies the live bytes into
  `current` while it diffs, keeping only the bytes it overwrote. The walks
  that move a materialized state (undo, redo, jump, filling or crossing an
  anchor) swap the tile bytes with that state's. `materialize`, which rebuilds
  a state for the History Brush or a dropped anchor, never moves one. Every
  delta it crosses already holds the side it needs (down from `current`
  before-bytes; up from `current` or a full anchor after-bytes), so it reads
  them unchanged. The model test now rebuilds every retained state after every
  step. Breaking the swap makes five history tests fail.
- **Radial Blur** ran one serial loop per plane, with `atan2`, `cos` and `sin`
  for every sample. Now:
  - Spin rotates the pixel's offset by precomputed sample angles.
  - Zoom hoists its ray direction out of the sample loop.
  - Each tap's bilinear indices and weights are found once for all planes.
  - The planes are interleaved, so a tap reads one cache line.
  - Work runs over 64×64 tiles, because a row's Spin samples sweep a diagonal
    across thousands of rows and miss the cache.

  The recorded digests are unchanged.
- **Zoom on the GPU.** A new shader mode handles Zoom, whose samples stay near
  the pixel's row. Spin stays on the CPU: on the integrated GPU its arc
  sampling took 12.6 s against the CPU's 5.6 s.

8000×8000, amount 100, Best, commit on a worker, measured while a game held
most cores (load average ~10):

| | before (reported) | after |
| --- | ---: | ---: |
| Spin, total | 30–45 s | 5.6 s (UI blocked 0.11 s) |
| Zoom, total | — | 1.9 s with GPU (6.7 s CPU) |
| history kept per full-layer commit | 768 MB | 384 MB |

## Ceilings and follow-ups

- ponytail: the 4 MP exact-preview budget is one constant for every filter,
  not a per-filter cost model.
- ponytail: the CPU-or-GPU split was measured on one machine, an integrated
  GPU. Re-measure on a discrete GPU before widening it.
- History keeps one side of each changed tile, but still the composite's and
  the layer's tiles (~1.8 GB per full-layer filter on the map). Dropping the
  composite tiles (recompositing on undo) or tiled storage (the
  `large-image-stroke-boundaries` follow-up) would shrink it further.
- ponytail: Radial Blur Spin is 32 bilinear taps per pixel at Best; on the
  map it remains seconds of CPU work.
- ponytail: input that arrives during a background commit is queued, not
  dropped, and runs once the commit lands. A control-server request during a
  commit is not blocked.
- Lens Flare and Lighting Effects still preview the whole layer exactly on the
  canvas; they could take the same proxy.
- Box Blur's CPU path (prefix sums in f64) is still slower than the GPU; it
  stays on the GPU.
- GPU Gaussian at radius 60, Motion Blur and Oil Paint did not match the CPU
  byte-for-byte at 8192². These are existing GPU tolerance cases and are not
  changed here.
