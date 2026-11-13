# Design

## D1. `layer_digest`

- **Request:** `{"path": "<layer_tree path>"}`.
- **Answer:** `{"digest": "<16 hex>", "rect": [left, top, right, bottom]}`. An
  unknown path, or no document, is `invalid_param` or `no_document`.
- **What is hashed, in order:**
  1. the rect, as four little-endian `i32`;
  2. then for each channel in the layer's order, its id as a little-endian
     `i16`, followed by its rows read with `Plane::read_row_into`;
  3. then, when the layer has a mask with data, the mask's rect followed by its
     rows.
- A row past the end of its plane hashes as a single `0xFF` marker. This keeps
  an empty channel distinct from a black one.
- The digest is FNV-1a, the same function as `pictura_core::content_digest`,
  folded one row at a time.
- Memory stays bounded: one row buffer, and tiles are borrowed one at a time.
  A spilled tile is read back by the plane itself.

This is a Rust bridge function that takes `&PictureView`, in its own bridge
file. That leaves `cxxqt_object.rs` under its file-size ceiling.

## D2. Scenarios M19–M23

The setup for each is `new_document`, `add_layer`, `gradient_layer`, the same
as M14–M18.

| ID | Measured step |
|---|---|
| M19 | `layer_op free_transform`, scale 50 |
| M20 | scale 100, angle 15 |
| M21 | scale 150 |
| M22 | `dispatch_command edit.transform.perspective`, image-space drag of the top-left corner to (w/4, 0), then `key Return` |
| M23 | `dispatch_command edit.transform.distort`, drag of the top-left corner to (w/8, h/8), then `key Return` |

For M22 and M23, the harness then checks that no transform session is still
open and that the history gained a state. A drag that missed the handle would
otherwise commit an identity transform and pass silently.

## D3. `--digest` and the compare

- With `--digest`, a memory row gains `digest: {layer name: hex}`. The harness
  reads it after the peak is recorded, so it costs time but not memory.
- `memory_compare_lines` adds a `pixels` column: `same` or `differs` when both
  runs carry digests, and empty otherwise.
- The acceptance run is the same scenarios, run uncapped and then capped, with
  `--compare` between them.

## D4. Progress during spills

The render test wraps the homography in a `PlaneMap` that records the job's
permille every time it maps a point. On the first call of each strip it also
runs an eviction pass. Strips run one after another, and `advance` runs
between them, so the recorded values rise in steps.

The test asserts three things:
- at least four distinct values strictly between 0 and 1000, in order;
- tiles spilled while the resample was running;
- the result equals the dense resampler's output.

This is deterministic, and it uses the same `job::current` path that the app
job installs.

## D5. `key` with no active window

Offscreen, a long step (the 17000² gradient, a progress window) can leave no
window active, so `QApplication::focusWidget()` is null and the key went to the
main window, where nothing handles Return. `key` now tries the main window's own
focus child first, which is where the key would go once the window is activated;
only then the main window. A desktop session with an active window is unchanged.

## D6. The GPU region composite and the governor

After a commit lands, its deferred composite of the dirty region tried the GPU
first. `composite_gpu_region_raw` holds, at once, a `u32` canvas, a planar
readback and its staging buffer, the `PixelBuffer` output (4 bytes a pixel
each), and one source upload a layer (4 planes of a byte). On this host's
unified-memory GPU all of that is process memory: more than 5 GB for a 17000²
region, so `MemoryMax=2G` killed the app. The scale commits survived only because
their 25500² result made the GPU decline with `TooLarge`.

The kernel now reserves that estimate with `Governor::reserve_if_room` after
`Gpu::new` has checked the device limits (so an oversized region stays
`TooLarge`) and before any region-sized buffer, and holds it until the readback
is copied out. The estimate walks the plan: a source and a mask for each layer
and effect pass, an inner canvas for each isolated or clipping group, summed
because a dropped buffer may live until the queue drains. It does not evict to
make room and does not record a refusal (a declined accelerator is not a failed
edit). When the bytes do not fit, it returns `GpuError::Memory`, counted as the
`memory` decline, and `composite` falls back to the CPU, which bands a large
region into tiles.

## D7. History and evicted tiles

With the GPU gate in place, M22 and M23 still died under 2G after landing, while
M19–M21 did not. The difference is the layer rect: a scale or rotation changes
it, so history stores a full state; Perspective, Distort and a 90 % drag keep
the 17000² bounding box, so `diff_into` compares the planes tile by tile. That
comparison pinned every unshared tile of both sides, and a pin restores a packed
or spilled tile and leaves it resident: about 35k tiles inflated at once, beside
the banded composite. `changed_or_away` now reads a tile only when it is
resident on both sides; otherwise the whole tile rect is the changed box. The
delta still holds the old tile by `Arc`, so undo is unchanged; the only cost is
that identical tiles that were evicted are no longer deduplicated.
