# Proposal

## Why

The CPU compositor is the fallback for every document the GPU declines: any
clipping, Blend If, knockout, smart filter, high-depth source, or unsupported
effect and adjustment (and #1017 makes that decision per layer, not per
document). Its inner loop still recomputes per-layer work on every pixel:
`native_unit` calls the non-inlined cross-crate `Document::authoritative_plane`
up to four times per pixel, `mask_alpha_unit` resolves the mask kind and samples
the feathered-plane or vector-row cache per pixel, and `blend_into_row`
recomputes opacity × fill, the text-gamma curve, and the Blend If default check
per pixel.
The measured reference is 213-223 ms for a 4000² backdrop plus one full-canvas
text layer on 16 threads, with no per-megapixel figure to target.

The cheapest large win is not in the blend arithmetic: a pixel whose raw source
alpha is zero still pays for colour-plane reads, mask sampling, and Blend If
before `blend_parts_row` throws the result away, and a pixel with nonzero alpha
but zero opacity or fill still pays for mask sampling, Blend If, and the curve.
Sparse layers (text, smart objects, masked content, fill layers with transparent
padding) have full-canvas rects and mostly-empty coverage.

## What Changes

- Add an ignored 4000² profile protocol that prints per-scenario timings for
  Normal without a mask, Multiply, a feathered raster mask, a vector mask, and a
  full-canvas type layer, so the target is measured before and after the
  rewrite.
- Skip a raw zero source alpha before reading colour planes, sampling the mask,
  evaluating Blend If, applying the text curve, or blending. A nonzero alpha
  with zero opacity or fill skips the mask, gate, curve, and blend but still
  reads the colour planes. The composite output is unchanged; zero-alpha
  blending was already a no-op.
- Resolve per-layer composite state once per layer pass (once per band on the
  banded path): opacity and fill, the text-gamma curve, whether Blend If applies,
  the mask gate kind (none, 8-bit raster, native raster, vector, feathered), and
  the layer's authoritative native sample planes.
- Sample native and mask planes by row instead of by per-pixel plane lookup, so
  a diverged banded plane is not materialized by `Plane`'s `Deref` and a
  feathered plane or vector row table is resolved once instead of per pixel.
- Keep one blend formula for every caller: `blend_into_row` becomes the
  context-based kernel and the pixel-layer, group, adjustment, fill, smart
  source, smart filter, and type-source paths all use it. The per-pixel API
  stays for the test-only legacy evaluation.
- Add test-only legacy blend and source switches, alongside the existing
  `FORCE_SERIAL` switch, so a test can compare the new kernel and native
  sampling against the pre-change evaluation bit for bit.
- Specialise the kernel per document depth and, if the profile shows the blend
  arithmetic dominating, for Normal mode. This is gated on the measurement; it
  is not attempted when the hoist alone meets the target.

No observable behaviour changes: every composite and saved file must stay
byte-identical, and the formula, its operation order, and the division by `ao`
are preserved exactly.

## Capabilities

### New Capabilities

- `compositing/cpu-composite-kernel`: how the CPU composite inner loop resolves
  per-layer state, skips fully transparent source pixels, and keeps one blend
  formula while producing byte-identical output.

This is a new capability rather than added requirements on
`compositing/cpu-kernel-parallelism`: that capability covers parallel execution,
not per-layer state resolution or transparent-pixel skipping.

### Modified Capabilities

None. Existing requirements in `compositing/layer-compositing`,
`compositing/cpu-kernel-parallelism`, `compositing/native-depth-composite`,
`compositing/mask-refinement`, and `compositing/vector-mask-sampling` stay as
they are and are re-verified by their oracle suites.

## Impact

- `crates/pictura-render/src/composite_rows.rs` — row kernels and the new
  context-based blend entry point.
- New `crates/pictura-render/src/composite_blend.rs` — per-layer context, mask
  gate, native plane resolution.
- `crates/pictura-render/src/composite.rs` — `blend_into` builds the context
  per call; `blend_parts` keeps the raw blend math.
- `crates/pictura-render/src/composite_native.rs` — adjustment gate paths use
  the context.
- `crates/pictura-render/src/fill.rs`, `smart_source.rs`, `smart_filter.rs`,
  `text_render.rs` — rect loops hoist the context.
- `crates/pictura-render/src/mask_refine.rs` — expose a resolved feathered
  plane instead of the per-pixel cache lookup.
- `crates/pictura-core/src/samples.rs` — a row accessor over `Samples`.
- `crates/pictura-core/src/vector.rs` — an opaque resolved coverage sampler.
- New `crates/pictura-render/tests/composite_profile.rs` (ignored profile) and
  unit tests in `crates/pictura-render/src/tests/`.
- No new dependencies. No `docs/` change.
