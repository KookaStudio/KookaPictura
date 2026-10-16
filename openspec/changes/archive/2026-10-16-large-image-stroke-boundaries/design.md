# Design: large-image-stroke-boundaries

## Measured on the world map

16507×16196 RGB PNG (267 MP), one Background layer, 16 cores, RelWithDebInfo,
offscreen. Engine numbers from `large_document_stroke_profile`
(`cargo test -p pictura_app --release --lib large_document_stroke_profile --
--ignored --nocapture`); GUI numbers from 120-move brush drags driven through
the control socket with `PICTURA_PAINT_TIMING=1`.

| step | before | after |
| --- | ---: | ---: |
| first dab of a stroke | 0.47–1.1 s | 4–6 ms (≈60 ms on the very first stroke, first touch of the coverage pages) |
| each later dab, with present | ~0.8 ms | ~0.6 ms |
| commit refresh | 0.64 s | 20–25 ms |
| history capture | 0.38–0.66 s | ~82 ms |
| end of stroke, in the GUI | ~1.3 s | 100–115 ms |
| 120-move drag, release included | 1.8 s | 0.13–0.15 s |
| undo of a stroke, in the GUI | 6 s | 25–33 ms |
| a full canvas image build | 1.4–1.5 s | 0.18–0.32 s |
| `refresh()` while opening (Move warm) | 7.7–9.9 s | ~1 ms |
| history's private copy at open | — | ~0.95 s, once |
| GUI RSS after open / after strokes | 5.4 GB / — | 6.3 GB / 6.3 GB |

### Layer properties (second pass)

Map converted to a normal layer, changed through the Layers panel.

| step | before | after |
| --- | ---: | ---: |
| lock / unlock | 2–3 s (a full recomposite) | ~5 ms |
| full-size region composite, CPU | 8.7 s | 1.3–1.6 s |
| — of which the canvas → planes conversion | 6.4 s | folded into the bands |
| full-size region composite, GPU | 2.8–4 s | unchanged |
| view-pyramid update over the whole document | 1.39 s | 0.2 s |
| blend change on the full-size layer, CPU | — | ~2.4 s end to end |

The CPU compositor's `into_pixel_buffer` wrote every byte through the plane's
`DerefMut` (a refcount check per write) on one thread, and the region kept a
4.3 GB `f32` canvas for the map. With banding the CPU now beats the GPU
compositor at every size measured (4000²: 85 vs 166 ms; the map: 1.3 vs 3.3 s),
because the GPU path is bound by moving the planes to and from the device.

### GPU transfers (third pass; the GPU path is kept)

The device is an integrated AMD 780M (RADV): its memory is system RAM, so the
cost was host-side copying and page faults, not the bus. The compute itself is
0.05–0.13 s on the map.

| full-document composite | CPU | GPU before | GPU after |
| --- | ---: | ---: | ---: |
| 4000², 8 layers | 392 ms | 545 ms | 145 ms |
| map, 1 layer | 1.25 s | 3.3 s | 0.93 s |
| map, 2 layers | 1.93 s | 4.2 s | 1.5 s |

What went: assembling the source into a host `Vec` and copying it again into
staging (0.6 + 0.4–2.2 s), the constant-255 mask built and copied the same way
(0.3 s), `PixelBuffer::new` zero-filling and copying the readback buffer (1.0–
1.6 s), and the readback mapping faulted in afresh each time (~0.8 s). Parallel
reads of the mapped readback were measured slower (1.6 s against 0.4 s
sequential), so that copy stays on one thread after the output is faulted in
on every core.

Without a usable GPU every composite runs on the CPU: `composite_active` /
`composite_region_active` try the GPU only when it is enabled and available,
and any `GpuError` (too large, unsupported mode or adjustment, readback
failure) falls back to the CPU for that call. A software Vulkan adapter
(Mesa's lavapipe/llvmpipe, present on many machines without a GPU) is now
rejected as no GPU: it took 159–446 ms against the CPU's 93 ms at 4000².

A full-size blend change in the GUI is now ~2 s steady with the GPU on: the
composite ~1.05 s, then the whole-region copies — composite patch 0.08 s, display
image 0.2 s, level 0 0.17 s, pyramid 0.19 s, canvas blit 0.21 s.

### Follow-ups (fourth pass)

- **No canvas image.** A document canvas holds its size, not a 1 GB image;
  region blits only invalidate the crops (0.2 s → 0.02 ms on a full-size
  region) and memory after opening the map fell from 6.3 GB to 5.2 GB. The
  self-test's level-0 identity check draws the fallback from a visible-area
  crop of level 0.
- **GPU residency.** Layer sources and coverages are kept between composites,
  keyed by plane stamps (`gpu/resident.rs`, 2 GB budget). A full-size blend
  change's GPU composite: ~1.05 s → ~0.53 s; end to end ~1.6–2.0 s → ~1.2–1.3 s
  with the GPU on.

## Why it was slow

Every cost was a whole-plane copy or scan at a stroke boundary. Layer planes are
`Arc<[u8]>` copy-on-write and the live document shared them with the history's
snapshot, so the first write anywhere copied the plane: the stroke's first dab
forked the target layer (3 × 267 MB), the commit's composite patch forked the
1 GB composite, and level 0 (an `Arc` clone of the composite for an sRGB
document) forked on its first patch. History then compared whole planes (an
equality pass, then a single-threaded tile scan), and undo forked its own planes
and rebuilt the whole canvas.

## What changed

- **Plane stamps** (`pictura-core::Plane`): two planes with the same nonzero
  stamp hold the same bytes; any mutable access clears it, a clone keeps it.
  Ported from photorust's `Pixmap` stamp.
- **History** keeps `current`, the state at the cursor, in planes the live
  document never shares (`planes::private_copy`). Capture skips stamped-equal
  planes, diffs the rest by 64×64 tiles on every core, and writes only the
  changed tiles into `current`. `undo_live` / `redo_live` / `jump_live` apply
  the same tiles to `current` and the live document in place when their stamps
  agree, and report `Restored::Region` (the composite tiles, folded to document
  rows) or `Restored::Everywhere` (an anchor crossed, a channel or mask view
  plane changed, a display-relevant document field changed, or the planes did
  not agree). The anchor of the cursor's segment is hollow (metadata only) so
  history holds the document once; other anchors stay full. The model test
  `live_restores_match_a_full_snapshot_history` holds all of it to a naive
  full-snapshot history.
- **Stroke** paints the document handed to each call in place and keeps the
  pre-stroke pixels of every 64×64 tile it is about to write (`Saved`, in
  zero-filled layer-sized planes so only saved tiles take memory). The stencil
  reads those saved pixels where it read the old `base` copy;
  `painting_in_place_matches_a_stroke_that_saved_the_whole_layer` proves the
  result byte-identical across modes, jitter, rotated and square tips. `cancel`
  restores the tiles; the Background Eraser, which must layer a Background
  first, does so on the live document and keeps the original in `stroke_base`.
- **Shell**: undo/redo no longer force `refresh()`; a bounded restore blits its
  region and emits `changed`, and `refresh()` skips `image()` while the canvas
  holds the current `frame_revision`.

## Ceilings and follow-ups

- ponytail: history capture still compares every written plane whole (~82 ms on
  the map, memory-bandwidth bound). Tile generation counters on the planes —
  photocraft's tiled copy-on-write storage — remove it and the hollow-anchor
  bookkeeping.
- Opening the map still takes ~25 s, most of it outside these paths (decode and
  import); not profiled here.
- The owner kept the GPU path; it now beats the banded CPU compositor at every
  size measured, and unchanged layers stay resident between composites.
- A full-size layer's property change still copies the region through the
  composite, level 0, the display image and the canvas (~0.9 s on the map).
- The Move tool still warms a whole-document preview for a movable layer on
  activation; on a large multi-layer document that is a multi-second stall.
