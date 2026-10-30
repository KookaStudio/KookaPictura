# Design

## Context

See `proposal.md` for motivation. The relevant current state:

- `composite_pixels` (`crates/pictura-render/src/composite_rows.rs`) walks each
  layer's rect row by row, but calls `native_unit` up to four times per pixel.
  `native_unit` (`composite_native.rs`) wraps `Document::authoritative_plane`
  (`crates/pictura-core/src/lib.rs`), a non-inlined cross-crate call with no
  `[profile.release]` LTO. `sample_unit` reads `Plane<T>` through `Deref`, whose
  `as_slice` materializes a diverged banded plane.
- `blend_into_row` (`composite_rows.rs`) recomputes opacity × fill, calls
  `mask_alpha_unit` (`composite_native.rs`), evaluates Blend If, applies the
  text-gamma curve, and only then reaches `blend_parts_row`, which returns
  immediately when the resulting alpha is zero.
- `mask_alpha_unit` resolves the mask kind per pixel. The feathered path enters
  `mask_refine::sample`, which borrows a thread-local `RefCell` and linearly
  searches up to `CAPACITY` keys; the unfeathered vector path calls
  `VectorMask::coverage`, a `OnceLock` get plus a row-table lookup per pixel.
  The 8-bit combine is integer `(raster × vector + 127) / 255`; the native path
  is an `f32` product. Both must be preserved.
- `blend_into_row` is shared by `composite_pixels`, `composite_canvas`
  (isolated groups), `composite_adjustment`/`gate_adjusted`, `fill.rs`,
  `smart_source.rs`, `smart_filter.rs`, and `text_render.rs`. The latter call
  `blend_into`, which wraps `blend_into_row`, once per pixel from their own rect
  loops.
- `clip` is fixed for a canvas, so a feathered plane's cache key and a vector
  row table are stable for a whole layer pass. A banded composite builds one
  canvas per `BAND` (64) rows, so its layer pass is per band.

## Goals / Non-Goals

**Goals:**

- Remove per-pixel per-layer constant work, authoritative-plane lookups, and
  mask-cache lookups from the CPU composite inner loop.
- Skip the mask, Blend If, curve, and colour-plane work for zero-alpha source
  pixels.
- Keep one blend formula for every route, with a test-only legacy evaluation for
  a bitwise oracle.
- Keep every composite and saved byte identical.

**Non-Goals:**

- Changing the blend equation, its operation order, or the division by output
  alpha.
- Changing the `Px` accumulator layout or adding SIMD.
- Changing GPU eligibility, the banded threshold, or `FORCE_SERIAL`.
- Rewriting the feathered-plane or vector-row cache semantics.

## Decisions

### D1. A per-layer `BlendCtx`, built once per layer pass

Add `crates/pictura-render/src/composite_blend.rs` with:

```text
BlendCtx<'a> {
    opacity: f32,               // opacity / 255
    fill: f32,                  // 1.0 for a group, else fill / 255
    curve: Option<&'static TextCurve>,  // Some only for a type tool with gamma on
    blend_if: Option<&'a BlendIf>,      // None when absent or default
    mode: BlendMode,
    mask: MaskGate<'a>,
}
```

The kernel computes `(src_a * opacity) * fill` per pixel, preserving the
pre-change association. Precomputing `opacity * fill` would change `f32`
rounding.

`MaskGate<'a>` carries the resolved mask shape and the exact combine mode:

```text
MaskGate::None                                  // 1.0
MaskGate::EightBit { raster, vector }           // (r*v + 127)/255, then /255
MaskGate::Native   { raster, vector }           // raster_f32 * vector_f32
```

- `raster` resolves to `None`, a plain 8-bit plane with its rect/default, a
  `RasterGate<'_>` (a resolved feathered plane), or the native `-2` `&Samples`.
- `vector` resolves to `None`, a `VectorGate<'_>`, or a resolved feathered plane
  with invert/density. `VectorGate<'_>` wraps the core `VectorCoverage<'_>`
  sampler; the names are the render-side gate and the core sampler.
- Density is applied inside the gate, as `mask_refine` does today. When
  density is 255 the integer lift is skipped (it is the identity).

The source side is resolved separately for `composite_pixels`:

```text
SourcePlanes<'a> { native: [Option<&'a Samples>; 4] }   // ids 0,1,2,-1
```

`composite_pixels` keeps its 8-bit channel planes and resolves the native plane
once per layer, then takes `.row(ly, lw)` per row.

Rationale: the context is the smallest object that removes all per-pixel
constant work while preserving the two mask-combine formulas. Building it per
band is O(1) per 64 rows; the feathered plane is shared through the existing
cache, not rebuilt.

Alternatives considered:

- *Only `#[inline]` the authoritative-plane lookup.* One line, but leaves the
  mask-cache lookups, the per-pixel Option chain, and the zero-alpha work.
  Kept as a complementary one-line change, not the solution.
- *A global thread-local context.* Rejected: layers interleave, bands run in
  parallel, and it would not be reentrant.

### D2. Early-out cascade, in this order

`blend_into_row` becomes:

```text
content = (src_a * opacity) * fill
if content <= 0.0            -> return            // no mask, no curve, no gate, no cover
content = curve(content)
if content <= 0.0            -> return
mask    = ctx.mask.alpha(x, y)
if content * mask <= 0.0     -> return            // skip Blend If
gate    = blend_if_factor(...)
blend_parts_row(..., content * mask * gate, mode)
```

`composite_pixels` reads alpha first and skips the three colour reads when it is
zero:

```text
a = alpha(lx)
if a <= 0.0 { continue; }
(r, g, b) = colour(lx)
blend_into_row(..., [r,g,b], a)
```

Byte-identity argument:

- The old path reaches `blend_parts_row` with `alpha == 0`, which returns before
  writing `cover[i]` or `row[i]`. The new early returns happen at the same
  observable point. `curve(0) == 0`, and `content * 0.0 == 0.0` for finite
  content, so the second and third returns only skip work whose product was
  already zero.
- The only skipped reads for a raw zero alpha are colour native planes and
  mask/gate state; none of them can affect the output when the source alpha is
  zero. A nonzero raw alpha with zero opacity or fill reaches the cascade: it
  skips the mask, Blend If, curve, and blend, but `composite_pixels` has already
  read the colour planes.
- `-0.0 <= 0.0` holds, so a negative-zero alpha takes the same branch the old
  `blend_parts_row` took.

### D3. Native sampling by row, one depth dispatch per row

Add to `pictura-core`:

```text
Samples::row(y, width) -> Option<SampleRow<'_>>
SampleRow::{U8(&[u8]), U16(&[u16]), F32(&[f32])}
SampleRow::unit(i) -> Option<f32>          // same conversion as sample_unit
```

`Samples::row` returns the available slice; it returns `None` only when the row
start is out of range, so a truncated plane keeps a short row. `SampleRow::unit`
bounds-checks per pixel and returns `None` past the row, matching `sample_unit`'s
per-index `get` and the 8-bit fallback.

`composite_pixels` resolves `SourcePlanes` once per layer, then per row:

1. If no native row for any channel, run the pure 8-bit loop: index the
   `&[u8]` rows directly, no enum, no `Option`.
2. Otherwise resolve each channel's `SampleRow` (or `None`) and match the store
   depth once per row. All native planes in a store share
   `SourceChannels::depth`, so the row loop is monomorphized for that depth; a
   channel whose plane is absent or whose variant differs uses the per-channel
   8-bit fallback.

Rationale: channels can mix native and 8-bit (a store may omit a channel), so
the depth branch belongs at row granularity, not layer granularity; the row
slice is resolved once. `.row()` also avoids the `Deref` materialization that
`sample_unit` triggers on a diverged banded plane.

Alternatives considered: keep the `Option<f32>` per channel per pixel and rely
on inlining. Rejected: it is the cost being removed.

### D4. Resolve the mask, keep both combine formulas

In `mask_refine.rs`, add:

```text
resolve_raster(mask, clip) -> RasterGate<'_>
resolve_vector(mask, clip) -> VectorGate<'_>
```

`resolve_raster` resolves the feathered plane eagerly: it gets the cell from
`shared_cell` and calls `get_or_init` once, then returns a `RasterGate<'_>` that
owns the `Arc` and exposes `at(x, y)` plus the density. The kernel never calls
`get_or_init` per pixel. `resolve_vector` returns a `VectorGate<'_>`: either a
core `VectorCoverage<'_>` (unfeathered) or a resolved feathered plane, with
invert and density. Both reuse the existing `Key`/`shared_cell` machinery.
`sample()` stays for the remaining per-pixel callers.

In `pictura-core::vector`, add an opaque `VectorCoverage<'_>` returned by
`VectorMask::coverage_sampler(clip)`, wrapping the `SmoothRows` table and
answering `level(x, y)`; `coverage()` becomes a thin wrapper. This exposes the
resolved row table without exposing `SmoothRows`. The render crate's
`VectorGate` wraps it; the two names refer to the core sampler and the
render-side gate.

The 8-bit gate reproduces `mask_alpha` exactly: raster sample (with density and
feather), vector sample (with density, feather, invert), integer
`(r*v + 127)/255`, then `/255.0`. The native gate reproduces `mask_alpha_unit`:
native `-2` sample (only when density 255 and no feather, as
`native_mask_unit` decides) times the vector sample as `f32`. The two paths are
not unified, because that would change bits.

### D5. One formula, per-pixel entry point kept for the legacy test

`blend_into_row_ctx` is the shared kernel; it takes a prebuilt `BlendCtx`.
`blend_into_ctx` in `composite.rs` takes the same context and one sample, so a
rect loop builds the context once. `blend_parts` has no layer, document, or clip
and cannot build one, so it keeps calling `blend_parts_row` with
`alpha * effect_scale`. The old per-pixel `blend_into` wrapper was removed
because every caller now hoists; `blend_into_row` is `#[cfg(test)]` for the
legacy oracle, so `-D warnings` does not flag dead code. Every rect loop that
can hoist does so:

- `composite_pixels`, `composite_canvas`
- `composite_adjustment`, `gate_adjusted`
- `composite_solid_fill`, `composite_gradient_fill`, `composite_pattern_fill`
- `composite_smart_source`, `composite_smart_filtered_source`,
  `composite_type_source`

This keeps one implementation of the blend math while letting the hot callers
pay for the context once.

### D6. Test-only legacy switches and mask-sample counter

Add, following the `FORCE_SERIAL` pattern, in `composite_blend.rs`:

- `FORCE_LEGACY_BLEND: AtomicBool` — when set, the kernel's test branch calls
  `#[cfg(test)] blend_into_row_legacy(row, cover, ox, layer, doc, clip, x, y,
  cs, src_a)`, the pre-change body. The switch cannot live inside `BlendCtx`:
  the context has no layer, document, or clip, so the kernel receives them as
  arguments for this branch.
- `FORCE_LEGACY_SOURCE: AtomicBool` — when set, `composite_pixels` samples the
  old per-pixel `native_unit` path instead of `SourcePlanes`, so the high-depth
  oracle compares the source migration, not only the blend body.
- `MASK_SAMPLES: AtomicUsize` — incremented once per mask sample in the context
  path, so a test can assert a zero-alpha layer sampled no mask.

All three are `cfg(test)` and absent from production builds, following the
existing `PARALLEL_RUNS` pattern. Tests that toggle either switch or reset the
counter take `serial_test_lock()`; the counter is reset immediately before the
measured composite.

### D7. Specialisation is gated on the profile

After the hoist, run the 4000² profile, including a full-canvas type layer (the
sparse case the early-out targets). Add a monomorphized Normal kernel and
per-depth row loops only if `blend_parts_row` still accounts for a dominant
share of a no-mask Normal pass. Otherwise stop; the extra code is not earned.

## Risks / Trade-offs

- **[Reordering changes bits]** → The legacy switch test compares every blend
  mode over random inputs; the oracle suites compare full composites. The
  cascade returns only where the old code already returned.
- **[A band-local clip would rebuild the feathered plane]** → Each band inherits
  the whole requested clip, so the cache key is stable: the first band builds
  the plane and later bands hit the same cell; the `Arc` keeps it alive even if
  the shared cache evicts it.
- **[Banded-plane materialization remains elsewhere]** → The kernel never calls
  `as_slice()`; it uses `.row()`. Other callers are out of scope.
- **[A native row shorter than the layer width]** → Keep the per-pixel 8-bit
  fallback exactly as `sample_unit` has it.
- **[Formula drift across six call sites]** → One context kernel; the old
  per-pixel body exists only behind the test switch.
- **[File-size caps]** → New code goes in `composite_blend.rs`; `composite.rs`
  is at 997 lines against the 1200 cap and only loses per-pixel bodies.
- **[`blend_into_row` is called with `cover` for knockouts]** → The cascade must
  not write `cover` on a zero-alpha pixel; the test `ck_skip_cover` asserts it.

## Migration Plan

1. Land the ignored profile and record the baseline.
2. Land the early-out cascade; it is independent and byte-identical.
3. Add the core row/coverage accessors and the `mask_refine` resolvers.
4. Add `BlendCtx` and migrate the pixel, group, and adjustment paths.
5. Migrate the fill, smart-source, smart-filter, and type-source loops.
6. Re-profile. Add specialisation only if the profile justifies it.
7. Run the full oracle and gate suite, archive the change, and run
   `verify-fast`; the branch ships as one PR with one commit per step.
