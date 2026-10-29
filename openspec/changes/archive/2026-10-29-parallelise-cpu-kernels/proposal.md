# Proposal

## Why

The CPU compositor evaluates independent output rows in parallel, but the
O(region) passes it calls still run serially: the adjustment kernels, the
effect-matte mask multiply, the clipping lerp, and the non-sRGB display
transform. Interactive stroke frames composite regions of 128 rows or fewer,
which skip the banded path, so every adjustment layer costs a serial
full-region pass per frame; a single drop shadow or glow blurs three identical
matte planes. On a 16-core machine the compute-heavy adjustments
(Hue/Saturation, Selective Color, Channel Mixer) leave most of the machine idle.

## What Changes

- Parallelise every per-pixel kernel in `pictura-adjust` through shared chunked
  helpers, and keep the histogram/mean reductions serial so their reduction
  order is unchanged. This covers the kernels the issue names plus the ones it
  does not: `map_lut` (Levels, Curves, Brightness/Contrast, Exposure, Equalize,
  Posterize) is the hot helper, `map_float` only serves `Invert`, and Color
  Lookup is a per-pixel kernel the issue omits.
- Replace the eight copies of the layer-effect matte mask multiply with one
  shared helper and run it in parallel, so drop shadow, inner shadow, outer and
  inner glow, bevel, satin, overlay and stroke all share the fix.
- Blur one matte plane instead of three in `blur_matte`; the result is
  bit-identical because the blur already treats each plane independently.
- Parallelise the clipping blend lerp and the clipping coverage collect.
- Parallelise the canvas-to-`u8` fill in the 8-bit adjustment-layer path.
- Build the display colour transform's output buffer directly with
  `Plane::build` and transform it in parallel chunks, instead of forking the
  whole four-plane buffer to overwrite three of them. The display transform
  becomes shareable across threads.
- No interface changes and no behavioural change: every touched kernel MUST
  produce byte-identical output, and each crate keeps a serial path its tests
  can compare against.

Out of scope: the native-depth (`f32`) adjustment path, the neighbourhood blur
itself, the band threshold, and the destructive Convert-to-Profile path.

## Capabilities

### New Capabilities

- `compositing/cpu-kernel-parallelism`: the contract that O(region) per-pixel
  CPU kernels run in parallel while staying byte-identical to their serial
  execution, and that a serial oracle remains available to tests.

### Modified Capabilities

None. The existing `compositing/gpu-compositing` row-parallel requirement
already permits parallel per-pixel content loops and keeps neighbourhood effect
kernels sequential; this change does not move that boundary.

## Impact

- `crates/pictura-adjust` — new `rayon = "1"` manifest entry (already in the
  tree via `pictura-filters`, `pictura-render` and `pictura-codec`; no new
  crate), new chunked helpers in `common.rs`, call sites in `color.rs`,
  `tonal.rs`, `lut.rs`, `auto.rs`, `replace_color.rs`, `match_color.rs`.
- `crates/pictura-render` — `layer_effects/mod.rs` (shared matte helper and the
  eight call sites), `composite_clipping.rs`, `composite_native.rs`.
- `crates/pictura-codec` — `icc.rs` display conversion.
- `crates/pictura-color` — `ByteTransform`'s transform type becomes the
  reentrant lcms2 transform, so the display conversion can share it.
- No `docs/` change: execution only, output bytes unchanged.
