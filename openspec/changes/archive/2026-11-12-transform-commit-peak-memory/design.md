# Design

## D1. The fast path lives in `transform_layer_with_map`

The fast path runs only when `pure_translate` is set, which means scale 1,
angle 0 and integer `dx`/`dy`. At such an offset every destination pixel centre
maps onto a source pixel centre, and the exactness depends on the kernel:

- **Bilinear:** `fx = fy = 0`.
- **Catmull-Rom and Sharper (b = 0):** the weights are `0, 1, 0, 0`.
- **Nearest:** it rounds to the same pixel.

Each of these returns the source sample, and `bounding_box` returns the source
rect shifted by the offset, so reusing the source planes is byte-identical.
The B-spline used by Bicubic Smoother (b = 1) has weights `⅙, ⅔, ⅙` at t = 0
and blurs, so the fast path excludes it (`interpolation.resolve(n, n)`) to keep
its output unchanged.

The source planes move out of `Prepared`; for tiled planes this moves the `Arc`
handles to the tiles, not the pixels. The mask follows the same refusal rules
as `transform_mask`. A mask whose data length does not match its rect gives
`None`, which keeps the old mask, just as the resample path does.

The fast path opens a one-unit job stage and advances it, so a multi-layer job's
stage accounting is unchanged.

## D2. `try_plane` for the 8-bit buffers

`try_plane` asks the governor for a buffer of 16 MiB or more and allocates it
with `try_reserve_exact`. The commit has already passed `memory_gate::admit`
for its whole estimate, so this reservation only adds a refusal when the host
lost room since the gate. A `None` from any channel makes
`transform_layer_with_map` return `false` before `write_layer`.

**Ceiling (`ponytail:`):** a refusal on the deferred job's worker thread is
recorded in that thread's refusal slot, so the GUI reports a generic failure,
not the memory message. The out-of-core work in phase 4 replaces this path.

## D3. `commit_bytes` for a move

The quad is a whole-pixel move when every corner equals the matching source
corner plus one shared integer offset. Such a move holds no new layer planes
(D1), so the estimate drops the `4·area(out)` term and keeps `12·area(union)`
for the composite refresh.
