# Design

## D1. One set of taps

Image Size's resample is separable and point-sampled: destination pixel
`(x, y)` reads source coordinate `((x + ½)·sw/dw − ½, (y + ½)·sh/dh − ½)`, and
each axis contributes taps clamped to `0 … n − 1` (one for Nearest, two for
Bilinear, four for Bicubic, Keys `a = −0.75`). The weights are combined as
`acc += wx·wy·v; ws += wx·wy` with `x` taps outer and `y` taps inner, and the
result is `acc / ws`.

`pictura_ops::resize` gets a public `Taps` (indices, weights, count) and
`axis_taps(s, n, resample)`, and its own `sample` is rewritten over them with
the same loop order and the same expressions, so its output does not change.
The tiled resampler uses the same taps. Nearest copies its single tap's sample
unchanged, as both dense paths do.

Two finishing rules stay as they are:

- **8-bit planes** (layer channels, masks, alpha channels) weigh `v as f64`
  and finish with `round().clamp(0, 255)`, as `pictura_ops::resize` does.
- **Retained stores** weigh `Sample::to_unit` and finish with
  `Sample::from_unit`, as `native_store::resize_samples` does.

The resampler takes these as a small `Kernel<T>` value (two function pointers
and the `Resample`), not a trait.

## D2. Strips, blocks and windows

`resize_tiled::resize(src, (sw, sh), channels, (dw, dh), kernel)` returns a
tiled plane built with `Plane::build_tiled_strips`: one strip of up to 256
destination rows of one channel at a time, filled in parallel tasks of 16 rows.
A task walks its rows in blocks of 256 columns.

The axis taps are computed once per plane, one per destination column and one
per destination row. A block's window is the sorted, deduplicated set of source
rows its row taps name, by the column span from its first column's lowest tap
to its last column's highest tap, taken as the minimum and maximum over the area's taps. The
window is read row by row with `Plane::read_rect_into` at global row
`c · sh + row`, so a stacked store's channels read their own rows. Each
destination row maps its taps to window rows by a binary search in the set,
and each column subtracts the span's start. Only the index changes, never a
coordinate or a weight, so the result is byte-identical.

When a window would exceed `WINDOW_SAMPLES` (1 Mpx; 4096 under `cfg(test)`,
so every test also splits), the block's area splits along its longer side,
recursively, down to one pixel, as `transform_tiled` does. A one-pixel window
is at most 4 × 4.

A source plane shorter than `sw · sh · channels` (the dense path zero-pads it)
is copied once into a padded flat plane first. That only happens for a
malformed plane.

Progress advances by each strip's rows. A cancelled job skips the remaining
strips. `resize_document` still returns `Ok`, and the job wrapper discards the
document, as before.

## D3. Every plane

`resize_document` sends layer channels, layer masks and document alpha channels
through the 8-bit kernel, and each retained layer store plane (the `-2` plane
at the mask's old and new rects) and the document-level `source_planes`
(`channels = len / (sw · sh)`) through the unit kernel. `resampled_rows` now
also counts the stores' rows, so the bar still ends at its total. The
strip buffer is allocated with `memory::try_buffer`, and a refusal comes back as
`OpsError::Memory`, a new variant. `document_ops::resample_plane` and
`native_store::resize_samples` are used only as test oracles from now on, so
they become `#[cfg(test)]`.

A retained `-2` plane used to be resampled from the mask's rect after the
mask itself had been rescaled, so it was scaled twice from the wrong source
size. `resize_layer` now keeps the old mask rect for it, as the transform's
`resample_store` does.

`materialize_merged_composite` (a document with no layers) is unchanged: it
copies the stored composite into a layer before the resize.

## D4. Gate

`memory_gate::image_size_cost(doc, width, height) -> CommitCost`:

- `working` = `threads · 4 MiB` (one window of up to 1 Mpx of 32-bit samples
  per worker thread) + `4 · 256 · max_plane_width` (one strip of the widest
  plane at 32-bit) + `refresh_bytes(new canvas)` when the composite is
  authoritative, because `Edit::Full` then renders it densely.
- `spillable` = for each plane above, its new size times its sample bytes:
  1 for the 8-bit planes, and the store's own sample size for each retained
  store.

`image_size_admits(view, width, height)` refuses a side over
`ImageBudget::max_dimension`, and otherwise calls `memory_gate::clear()` and then
`admit_spilling(working, spillable)`. `image_size_apply`, `resize_image`, and a
new bridge `image_size_fits_budget(view, width, height)` call it. The C++
`imageSizeFitsBudget` and the control server's `image.image.size` check use that
bridge. `image.canvas.size` keeps `document_resize_budget_allows`.

The per-allocation cap of `document_fits` is dropped for Image Size, because no
plane is allocated whole any more. A 17000² document at 150 % would exceed the
2 GiB cap as one dense RGBA buffer.

`imageSizeBudgetMessage` returns the scratch-disk text when
`edit_refused_for_scratch_full()` is set, and the memory text otherwise.

## D5. M15

`image_size(c)` runs `run_command image.image.size` with `width` and `height`
at 150 % of the document and `resample` `bicubic`. Its setup is M14's: a new
document, Layer 1, and a corner-to-corner gradient on it, so no tile is
uniform.
