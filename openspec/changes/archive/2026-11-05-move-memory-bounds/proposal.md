# Proposal

## Why

Issue #1207, second report: on a 17000×17000 document, moving, resizing and
transforming a painted black box gets pictura OOM-killed. A replay at 4250²
(1/16 of the area) with the RSS sampled every 10 ms, and a valgrind massif run
at 2000², found everything sized by the layer's canvas-sized rect rather than
by the box (D = one document of RGBA, 1.16 GB at 17000²):

- **Move press: ~3D transient, 1D kept.** `layer_image` copied the whole layer
  into a planar buffer, interleaved it to RGBA bytes and built a `QImage`,
  which the canvas copied again. Reading a banded, diverged plane through
  `Plane::as_slice` also materialized and **kept** a contiguous mirror of each
  plane until its next write.
- **Move commit: ~1.5–2.5D transient.** The commit's damage was the layer's
  rect before and after, so it recomposited the whole canvas, and the full
  frame replaced (and copied) the display's level 0.
- **History: no byte limit.** Twenty states of edits to large layers could
  hold several gigabytes; repeating the edits climbed until the kill.

## What Changes

- **A Move's moving image covers the layer's visible pixels**, read row by row
  so no plane is materialized whole; the cached preview is placed at the moved
  rect plus the mark's offset.
- **A Move's damage is a plain pixel layer's visible pixels** before and after,
  so the commit composites the mark's reach. A layer with effects, a group or
  an adjustment keeps its rect.
- **The memory gate covers the Move press and commit and the Free Transform
  commit.** A refused press starts no drag; a refused commit leaves the layer
  where it was; a refused transform is dropped and the layer restored. Each
  reports "not enough memory" on the status bar.
- **History has a byte cap**: after each capture, the oldest states are dropped
  while the bytes history holds beyond the live document exceed a quarter of
  the host's physical memory (owner decision), keeping one undo.
- Layers stay canvas-sized (owner decision): trimming a layer to its content
  on Move would clip later painting, since no engine grows a layer's rect.
  Growing layers on paint is a follow-up (#1208).

## Capabilities

### New Capabilities

- `tools/move-memory-bounds`

### Modified Capabilities

None.

## Impact

`pictura-core` (`Plane::is_mirrored`), `pictura-codec` (`total_memory`),
`pictura-app` (`helpers_composite::{layer_buffer_within, layer_image_within}`,
the Move preview and `move_reach`, the shared `memory_gate` and its
`edit_refused_for_memory` bridge, the Move and transform commit gates, the
history byte cap, the Move tool and Free Transform refusal messages).
