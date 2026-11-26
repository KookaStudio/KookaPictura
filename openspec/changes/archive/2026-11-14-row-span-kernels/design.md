# Design

## Context

See `proposal.md` for motivation. The relevant current state on master:

- `CompositePlan` resolves each visible layer once per composite
  (`crates/pictura-render/src/composite_plan.rs:70-267`); `PlanNode` carries the
  content body, effect passes, `BlendCtx`, and `SourcePlanes`.
- `BlendCtx` already hoists opacity, fill, the text curve, the Blend If view, the
  blend mode, and the resolved mask gates
  (`crates/pictura-render/src/composite_blend.rs:19-155`).
- `composite_pixels` already reads source rows as spans and samples native
  planes by row (`crates/pictura-render/src/composite_rows.rs:186-280`).
- The per-pixel context `blend_into_row_ctx`
  (`crates/pictura-render/src/composite_blend.rs:187-227`) still dispatches the
  mask gate and checks the curve and Blend If options per pixel; `source_unit`
  (`crates/pictura-render/src/composite_rows.rs:166-183`) checks the native and
  byte-plane options on every channel sample; and `blend_parts_row`
  (`crates/pictura-render/src/composite_rows.rs:113-160`) still runs a mode
  `match` and three divisions by `ao` per pixel.

The CPU compositor is chosen for a whole document when any layer is outside the
GPU plan, when the GPU is disabled, or when the document carries a source depth
(`crates/pictura-render/src/gpu/plan.rs:57-66`, `:331-341`). Plain Normal layers
therefore remain on this path inside otherwise advanced documents.

## Goals / Non-Goals

**Goals:**

- A row-span kernel for the common plain pixel content that produces the same
  bytes as the general per-pixel path.
- Per-row mask coverage for every mask kind, including the native `f32` mask,
  sampled only where the general path would sample it.
- Remove the per-pixel `u8` to `f32` divisions from both paths with a shared
  table for channel-plane and mask-gate samples.
- A forced-general switch and a parity matrix that make "same bytes" mechanical.
- 4000² before/after evidence in the existing ignored profile.

**Non-Goals:**

- Blend modes other than Normal; Blend If; text-gamma curves; native source
  samples; knockout cover; isolated-group and adjustment content routes stay on
  the general kernel in this change.
- Changing the native `SampleRow` conversions; the fast set excludes native
  stores.
- Changing the blend equation, the `Px` layout, the banding threshold, the
  `to_u8` quantisation, or GPU eligibility.
- Storing the kernel in `PlanNode` or caching the plan between composites.
- New dependencies.

## Decisions

### D1. Select the kernel per layer pass from the effective `BlendCtx`

`BlendCtx` is built in the plan and then patched by `with_override` for clipping
proxies and adjustment gating (`crates/pictura-render/src/composite_blend.rs:86-105`,
`crates/pictura-render/src/composite_clipping.rs:83-93`). Knockout does not use
a `BlendOverride`: it is excluded from the fast kernel by the cover canvas
(`Canvas::with_cover_from`, `crates/pictura-render/src/composite_knockout.rs:117-125`).
A kernel stored on `PlanNode` would describe the unmodified layer and go stale.

`composite_pixels` therefore asks the effective `BlendCtx`, plus
`SourcePlanes::any_native()`, for a kernel once per layer pass. The selection is
a handful of branches per layer, not per pixel, and automatically reflects every
override. Alternative rejected: a `kernel` field on `PlanNode` (stale under
overrides) or recomputing it inside `with_override` (duplicate state, same
result).

### D2. Fast set: Normal, no Blend If, no curve, no native, no cover, any mask

```
fast iff
    ctx.mode() == Normal
 && ctx.blend_if().is_none()
 && ctx.curve().is_none()
 && !sources.any_native()
 && cover.is_none()            // knockout canvas
```

The fast path applies to every content route that reaches `composite_pixels`:
plain pixel content, and a smart-object or type layer composited from its
raster proxy. A type proxy reaches the fast set only while the text-gamma curve
is off; the application default keeps the curve on, so the parity matrix's
type-proxy document is a general-path parity case. The type proxy's row-span
route is covered by the selection predicate
(`type_proxy_with_text_gamma_takes_the_general_kernel`), not by an in-process
gamma toggle. Masks stay in the fast set; the mask gate is resolved to one
`f32` coverage value per active pixel for the row (D4). The issue names masks
in the common case, and the mask-gate dispatch is one of the largest per-pixel
savings.

`legacy_blend()` and `legacy_source()` (test-only) disable selection, so the
pre-change oracles still see the legacy body.

### D3. Three kernel shapes, one formula

All kernels evaluate the same expression as `blend_parts_row` and preserve its
operation order (`((1-ab)*as_)*cs`, `(as_*ab)*b`, `((1-as_)*ab)*cb`, the sum
order, and the division by `ao`). They differ only in what is known constant:

| Shape | Conditions | Per-pixel difference |
|---|---|---|
| `normal_masked` | mask present | `as_ = ((src_a*opacity)*fill) * mask_row[i]` |
| `normal_alpha` | no mask, alpha row present | `as_ = (src_a*opacity)*fill`; keep `if src_a <= 0` skip |
| `normal_opaque` | `passes_alpha()`, no mask, alpha plane absent, within the D2 fast set | `as_ = 1`; `ao = 1`; `co = (1-ab)*cs + ab*cs`; no skip branch |

The opaque shape is derived by evaluating the shared formula with `as_ = 1.0`
and letting the constant fold: `1.0-ab` stays, `as_*ab*b` becomes `ab*cs`, and
`(1.0-as_)*ab*cb` becomes `+0`. The remaining terms are non-negative, so the
dropped `+0.0` cannot change a `-0.0` result; the division by `ao = 1.0` is
exact and removed. This is the only divide-free shape, and it is fully
straight-line.

The alpha and masked shapes keep the `src_a <= 0` early skip because the
existing requirement `ck_skip` forbids reading colour planes for a zero raw
alpha; that keeps them scalar, and their win is the removed mode/mask/`Option`
dispatch rather than vectorisation. The mask multiply itself cannot be hoisted:
`mask = 0` must skip the blend as well, so the `as_ <= 0` test stays. The
masked shape also relies on the row builder's active-pixel rule: a pixel whose
post-opacity/fill content is zero keeps `0.0` in the slice, so `as_ <= 0`
leaves it unchanged without the builder sampling the mask (D4).

### D4. Mask coverage is an `f32` row

`MaskGate::alpha(x, y)` (`crates/pictura-render/src/composite_blend.rs:136-147`)
already returns the exact per-pixel value for every gate kind defined in
`crates/pictura-render/src/mask_refine.rs:119-205`: `u8 as f32 / 255.0` for the
8-bit raster and vector gates, and the `f32` product for the native raster. The
masked kernel consumes an active-pixel row slice of those same values.

- A thread-local `Vec<f32>` scratch is resized to the visible span. For each
  row the builder computes `content = (src_a * opacity) * fill` for every
  pixel and calls the existing gate only where `content > 0.0`; skipped entries
  stay `0.0` and the kernel's `as_ <= 0` test leaves them unchanged. This
  preserves `ck_skip` and the mask-sample counter: a zero-alpha, zero-opacity,
  or zero-fill pixel never samples the mask.
- A native mask is never quantised to eight bits; it stays `f32`, which the
  thread-local scratch accommodates for every kind.
- An absent mask skips materialisation entirely (`None` slice, `as_` unscaled).
- Alternative rejected: a `u8` row plus a conversion in the kernel. It would
  change the native-mask product's rounding, and the plain-mask savings are
  small.
- Follow-up (not this change): borrow a flat plain-mask plane row when density
  is 255 instead of copying it.

### D5. The `u8` unit table

Add one shared 256-entry table: `UNIT[i] == (i as f32) / 255.0`, built as a
`const` array with a `const fn` (the pinned toolchain supports const float
arithmetic; a `OnceLock` fallback is not planned). `source_unit`'s byte branch
and `MaskGate::alpha` replace the division with `UNIT[b as usize]`. Native
`SampleRow` conversions are unchanged; the fast set excludes authoritative
native stores.

The compile-time and runtime divisions produce the same `IEEE-754` bits, so this
changes no output. It removes up to five true divides per pixel from every layer,
not only the fast ones, which is the largest single saving and the reason it is
kept out of the kernel gate.

### D6. A forced-general switch composes with the existing test switches

Add `FORCE_GENERAL: AtomicBool` beside `FORCE_SERIAL` in `composite_rows.rs`;
the legacy switches remain in `composite_blend.rs`. When set, selection returns
`None` and `composite_pixels` takes the current per-row general body. The parity
matrix toggles it on the same document; serial and legacy switches stay
independent, and a test may hold the serial lock while forcing either.

A test-only `KERNEL_RUNS` counter per shape lets a test prove the intended shape
actually ran, matching the existing `PARALLEL_RUNS` pattern. Tests use
`serial_test_lock` and reset guards for `FORCE_GENERAL` and the counters, so a
failed assertion cannot leave a process-global switch set.

### D7. Measurement

`crates/pictura-render/examples/composite_profile.rs` already measures 4000²
Normal, Multiply, feathered-mask, vector-mask, type, and mixed cases with an
`#[ignore]`. Add an alpha-less Normal case and a plain-raster-mask case that are
kernel-eligible: Normal mode, opacity 255, fill 255, no Blend If, no text-gamma
curve, no native stores, on a full-canvas 8-bit layer. Record the timings with
`cargo run -p pictura-render --release --example composite_profile` on the branch and on `origin/master`, and put the table in the PR.
Because the ignored profile is an example and the shape counters are
crate-private, assert the opaque and masked shapes in the unit-level matrix
(`profile_cases_use_the_intended_shapes`) as well. The profile is evidence, not
a pass/fail gate; the parity tests are the gate.

## Risks / Trade-offs

- **Auto-vectorisation may not fire for the alpha and masked shapes** (the
  zero-alpha skip and mask multiply are branches). Mitigation: their measured
  win is dispatch and table conversion; the opaque shape is the vector target,
  and the profile reports each shape separately. If the opaque shape regresses,
  it is a single `if` to disable.
- **Mask row materialisation costs one extra store and load per active pixel**
  compared with the inline gate. Mitigation: the gate work itself is unchanged,
  skipped pixels only store `0.0`, and the blend loop reads a flat slice;
  measure the feathered and vector cases. Borrow the plain plane row later if
  the copy shows.
- **Thread-local scratch re-entry.** The kernel never calls `MaskGate::alpha`
  while the scratch is borrowed; materialisation happens before the blend loop
  and uses a separate thread-local slot from `mask_refine`'s feather cache.
- **Kernel selection can drift from an override or a proxy route.** Mitigation:
  selection reads the effective `BlendCtx`, and the parity matrix includes
  clipping, knockout, smart-proxy, and type-proxy documents where overrides or
  routes change mode, fill, mask presence, or the content route.
- **A `const fn` float table could be rejected by a future pinned toolchain.**
  Mitigation: the table values are asserted against the division in a unit test;
  falling back to a `OnceLock` keeps the same bits.

## Migration Plan

No data, format, or API migration. The change is internal to the CPU
compositor; rollback is reverting the commit. GPU parity is unchanged because
the CPU oracle's bytes are unchanged.
