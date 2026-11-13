# Design

## D1. One strip driver

`geometry_tiled::build(dw, dh, channels, fill)` returns a tiled
`Plane<T>` built with `Plane::build_tiled_strips`. It works through one strip of
up to 256 destination rows of one channel at a time. The strip is split into
parallel tasks of `TASK_ROWS` (64) rows, and each task walks its rows in blocks
of 256 columns. For each block the driver calls
`fill(channel, Block { x, y, w, h }, out, stride)`, where `y` is local to the
channel and `out[r * stride + c]` is the block's destination sample. The
kernel reads its own source window through `Plane::read_rect_into`, at global
row `channel · sh + row`, so the channels of a stacked store read their own
rows.

The strip buffer comes from `memory::try_buffer`. A refusal comes back as
`OpsError::Memory`. Each strip is settled as it is written, so uniform tiles
hold no allocation. Progress advances by each strip's rows. A cancelled job
skips the remaining strips. The job wrapper then discards the result, as it
does for Image Size (#1293).

A source plane shorter than its geometry is read as the kernel's fill past its
end. The dense paths do the same: `extend_channel` reads `0`, `extend_background`
keeps the colour, and `remap_plane` reads `PixelBuffer`'s zeros. Only such a
malformed plane is copied whole and padded, as `resize_tiled` does.

## D2. Kernels

- **Shift** `(sw, sh) → (dw, dh)` at offset `(dx, dy)` with a `fill` value.
  Destination `(x, y)` reads source `(x − dx, y − dy)` inside the source, and
  `fill` outside it. Each destination row copies at most one span with
  `read_rect_into`, so no window is needed.
  - Canvas Size uses it with `fill = 0` for document alpha channels and every
    plane of the retained document store.
  - `extend_background` uses it once per channel, with the channel's fill: the
    colour, its luma on Gray, `255` for alpha, and `0` otherwise.
- **Orient** for `Rot90`, `Rot180`, `Rot270`, `FlipH` and `FlipV`. A block's
  source window is the bounding box of its two corners under the source map
  that `native_store::remap_plane` uses. The window holds at most 256 × 256
  samples. It is read once and indexed by that map. The 8-bit layer channels,
  masks and alpha channels use it, and so does every retained plane (the `-2`
  plane at the mask rect).
- **Rotate** with `rotate_in`'s `Rotation` and the plane's source and
  destination rects. A block's window is the bounding box of the inverse map
  of its four corner pixel centres. It is padded by two samples on each side
  and clamped to the source. A 64 × 256 block rotates into at most about
  320 × 320 source samples, so it never needs to split.
  - `sample_plane` becomes `sample_with(sw, sh, x, y, at)`, where `at(px, py)`
    returns the unit value. The dense oracle reads `src[py * sw + px]`. The
    tiled kernel reads `window[(py − y0) * ww + (px − x0)]`. Only the index
    changes, never a coordinate or a weight, so the output is byte-identical.
  - The padding covers rounding differences between the corners' inverse map
    and an interior pixel's. A test rotates by angles whose corners land on
    integers, so that boundary case is checked.

## D3. Commands

- **`resize_canvas_document`** clones the document. The clone shares its
  planes. On the clone it moves the layer, store and mask rects, as before (no
  layer plane is copied), and shifts the alpha channels and the document store.
  It installs the clone only if every plane succeeded.
  - **`extend_background`** now returns `Result<bool, OpsError>`. It shifts the
    Background's channels into canvas-sized planes. Canvas Size's
    `canvas_size_apply` and Crop's grow path call it.
- **`transform_document`** (turns and flips) clones the document and orients
  every plane through the driver. `rotate_document` already returns `Result`.
  `flip_document` now returns `Result<(), OpsError>`.
  - Progress becomes rows instead of layers. `orient_rows` counts every plane's
    output rows up front.
- **`rotate_document_in`** clones the document, rotates every plane through
  the driver, and returns `false` on a memory refusal, leaving the document
  unchanged. Its stage counts output rows, as Image Size's does.
  - The quarter-turn delegation, the vector masks and the frame geometry are
    unchanged.
  - Crop's straighten path calls it too and already handles `false`.

`resize_canvas_document` keeps materializing a layer-less document's merged
composite before it changes the document, and so do the turns and Rotate. Each
now does it on the clone.

## D4. Plane-byte estimators

`resize.rs`'s `walk` and `sample_bytes` move into a small `plane_bytes`
helper. That helper is
`mapped_plane_bytes(doc, canvas, layer_dims) -> (bytes, widest)`:

- `canvas` is the new canvas dimensions. They size the alpha channels and the
  document store.
- `layer_dims(rect)` gives a layer, mask or store plane's new dimensions, or
  `None` when the plane is not rewritten.

Four estimators use it:

- **`resized_plane_bytes`** keeps its signature. Its rects scale.
- **`canvas_plane_bytes(doc, w, h)`** rewrites no layer plane. It adds the
  Background's channels at `w × h` when the Background exists and does not
  already cover the new canvas.
- **`oriented_plane_bytes(doc, quarter_turns)`** keeps each rect's dimensions,
  swapped for a quarter turn. Pass `0` for a flip.
- **`rotated_plane_bytes(doc, angle, pivot)`** gives each rect the size of
  `rotated_rect`, and the canvas comes from `frame_geometry`.

## D5. Gate

`memory_gate::geometry_cost(doc, (planes, widest), canvas)` is the shared form
of `image_size_cost`. It is made of:

- **`working`**: `threads × 4 MiB`, plus one 32-bit strip of the widest plane
  (`4 × 256 × widest`), plus the dense refresh of the new canvas when the
  composite is authoritative.
- **`spillable`**: the plane bytes.

`image_size_cost` becomes a call to `geometry_cost`. `helpers.rs` gains three
admits that pass through `admit_spilling`, after `ImageBudget`'s side limit for
the new canvas:

- `canvas_size_admits(doc, w, h)`
- `orient_admits(doc, quarter_turns)`
- `rotate_admits(doc, angle)`

These callers gate before their `document_op`:

- `canvas_size_apply`, `resize_canvas`, `rotate_doc` and `flip_doc`.
- `rotate_document_arbitrary`, which is now a counted `document_op`.

The control server's `image.canvas.size` check calls a new
`canvas_size_fits_budget` bridge in place of `document_resize_budget_allows`.
That bridge and its `resize_budget_allows` helper stay for New Document's
callers.

## D6. Refusal and jobs in the shell

`memory_refusal` gains `commitRefusalMessage(action)`. It says the scratch disks
are full after a scratch-full refusal, and otherwise gives
`memoryRefusalMessage`. These handlers show it on the status bar when the
command did not apply because of a memory refusal:

- The Canvas Size handler, with the action "resize the canvas".
- The orientation handler, with "rotate the canvas" or "flip the canvas".
- The arbitrary Rotate handler, with "rotate the canvas".

The arbitrary Rotate handler runs through `runDocumentCommand`, as the quarter
turns do.

## D7. Benchmark

`memory_scenarios.py` gains three scenarios. Each sets up a new document, adds a
layer, and fills it with a gradient:

- **M16**, `canvas_size`: `image.canvas.size` to 150 % × 150 %, centred, with a
  white extension.
- **M17**, `rotate_quarter`: `image.rotate90cw`.
- **M18**, `rotate_arbitrary`: `image.rotate.arbitrary` with angle 15.

`run_command`'s table lists the five orientation ids with no keys.

## Ceilings

- `ponytail:` a Canvas Size whose offset is a multiple of 256 could share the
  alpha channels' tiles instead of copying them. A turn could not share them,
  because a turn transposes the samples inside each tile.
- `ponytail:` the extended Background still drops its retained 16/32-bit
  samples, as before.
