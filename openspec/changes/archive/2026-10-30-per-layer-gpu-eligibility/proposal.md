# Proposal

## Why

GPU compute is on by default, but GPU eligibility is all-or-nothing per document:
one visible layer with a layer effect, a clipping mask, a fill, a non-LUT
adjustment, or a proxy-less smart/text source forces the whole stack onto the
CPU. A typical CS6 document (drop shadow, clipping mask, Curves or Levels, Solid
Color fill) therefore never reaches the GPU, even though the GPU path measures
3–7× faster on composites and 2.5 ms vs 17.5 ms on a 4000² paint dab.

The eligibility gate and the dispatcher are also two independent implementations
of the same routing decision (`check_supported` vs `Gpu::composite_layer`), so
widening one without the other can silently composite the wrong source.

## What Changes

- Introduce one per-layer GPU plan that both eligibility and dispatch consume,
  replacing the duplicated route logic in `Gpu::composite_layer`.
- Accept cached CPU-rasterized sources as ordinary pixel sources: type without a
  raster proxy, smart-filtered smart objects (including the smart-filter mask
  mix), channel-less smart-object sources, and Solid, Gradient, and Pattern
  fills rasterized on the CPU.
- Accept Levels, Curves, Exposure, and Gradient Map adjustment layers through
  256-entry lookup tables built from the same `pictura-adjust` code that the CPU
  applies. The five already-supported kinds stay.
- Render enabled layer-effect passes as GPU dispatches around the layer content,
  using the cached `EffectPass` groups the CPU already builds.
- Composite clipping groups through the GPU backend with the CPU's sibling-group
  algorithm: the group buffer is built on the CPU and blended on the GPU,
  including the hidden-base and no-base cases.
- Keep 16/32-bit documents, knockout, non-default Blend If, and Dissolve on the
  CPU. A document still falls back as a whole when any visible layer declines;
  the decision becomes per layer, the fallback stays per document. A mixed
  GPU/CPU canvas is out of scope because the packed 8-bit GPU canvas cannot hand
  off to the `f32` CPU accumulator without breaking the ±1 LSB contract.
- Add a per-layer decline counter so the existing phase-0 measurement ranks
  reasons across a corpus instead of stopping at the first declining layer.

No breaking public API changes; the compositor signatures and `GpuError` variants
stay as they are, and the metrics `Counters` struct gains one additive field. No
new dependency.

## Capabilities

### New Capabilities

None. The behavior belongs to capabilities the project already has.

### Modified Capabilities

- `compositing/gpu-compositing`: the per-layer plan, the accepted content routes
  (cached raster sources, fills, LUT adjustments, effect passes, clipping
  groups), and the parity contract for each.
- `compositing/gpu-transfer`: resident uploads for content-keyed cached raster
  sources, so a proxy-less or smart-filtered layer is not re-uploaded every
  present.
- `compositing/layer-source-cache`: a cached composited raster source (the
  smart-filter mask mix and the placed smart-object render) so the GPU uploads
  the exact pixels the CPU would blend.

## Impact

- `crates/pictura-render/src/gpu/plan.rs` — per-layer plan and stack walk.
- `crates/pictura-render/src/gpu/raster.rs` — cached raster and fill uploads.
- `crates/pictura-render/src/gpu/effect.rs` — effect-pass uploads.
- `crates/pictura-render/src/gpu/clip.rs` — clipped-group build and upload.
- `crates/pictura-render/src/gpu/lut.rs` — adjustment lookup tables.
- `crates/pictura-render/src/gpu/backend.rs` — dispatch from the plan.
- `crates/pictura-render/src/gpu/shader.rs` — LUT and effect-pass branches.
- `crates/pictura-render/src/gpu/resident.rs` — content-keyed raster uploads.
- `crates/pictura-render/src/gpu/mod.rs` — public surface and error type.
- `crates/pictura-render/src/layer_source/` — cached composited raster source
  and its key, including the smart-filter mask.
- `crates/pictura-render/src/composite_clipping.rs` — a pure-refactor raw-group
  seam.
- `crates/pictura-render/src/fill.rs` — fill rasterization helpers for upload.
- `crates/pictura-render/src/layer_effects/` — export `EffectColor` for the
  effect-pass upload.
- `crates/pictura-render/src/metrics.rs` — per-layer decline counter.
- `crates/pictura-adjust/src/tonal.rs`, `lib.rs` — public lookup-table builder
  for the accepted adjustments.
- `crates/pictura-render/tests/gpu_parity/` — one parity scene per new route;
  `resident_upload_profile.rs` — reuse and region benchmarks.
