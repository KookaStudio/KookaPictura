# Design

## Context

See `proposal.md` for motivation. `layer_effects/bevel.rs` has a private
`blur_f32` with horizontal and vertical serial loops. It computes Gaussian
weights and sums in `f64`, stores the intermediate horizontal plane in `f32`,
then narrows the vertical result to `f32`. It is called for technique rounding
and separately for softened highlights and shadows.

`pictura-filters::kernel::blur_plane` already parallelizes rows, but uses `f32`
weights, sigma, and accumulation. Reduce Noise, Glass, and Diffuse Glow depend
on that arithmetic. `gaussian_blur_planes` offers another row-parallel pattern
but takes an 8-bit buffer. Neither is a compatible replacement for bevel.

The existing `every_effect_parallel_matches_serial` check switches the render
crate's mask/composite execution; it does not select a serial implementation
of a new blur inside the filters crate.

## Goals / Non-Goals

**Goals:** Preserve field bits and composite bytes while distributing each
blur pass across independent rows; place reusable convolution code in the
filters crate; isolate cold-composite time from decoding and cache hits.

**Non-Goals:** Change the bevel approximation or its distance/lighting models,
schedule whole effect builds, change cache reuse, or migrate existing
`f32`-accumulating filter callers. These are separate concerns.

## Decisions

### Add a precision-preserving shared helper

Add public `gaussian_blur_f32(src: &[f32], w: usize, h: usize, sigma: f64)` in
`pictura-filters::kernel`, beside the existing helpers. It returns `Vec<f32>`.
The three bevel call sites pass `sigma_from_radius(radius)` exactly as the
private implementation does today. Delete the private production helper.

Keep the existing private `blur_plane` unchanged: exposing it alone would
change precision, while upgrading it would change unrelated filters. Avoid a
generic configurable convolution abstraction; the two arithmetic contracts
justify separate helpers in the same shared kernel module. Reuse Rayon.

### Parallelize rows, preserving each pixel's computation

For each separable pass, allocate the output plane and use disjoint mutable
rows. Within each pixel, start a double-precision accumulator at zero and visit
kernel taps in their original order. Narrow after each pass, retaining the
horizontal intermediate plane. Complete the horizontal pass before reading it
for the vertical pass. Handle empty dimensions before creating row chunks.
Inputs are finite, correctly sized fields; this change does not add a broader
untrusted-input API.

The user authorized further optimization after the row-scheduled version still
took several seconds. Process a row of independent pixels one kernel tap at
a time, allowing SIMD across pixels while retaining each pixel's tap order.
Use reusable per-worker double-precision row accumulators and clamp-padded
horizontal input so boundary checks do not remain inside the multiply loop.
Retain the single-precision horizontal plane. Tap reordering, `f32` weights,
fused multiply-add substitutions, approximate blurs, and tap-parallel reduction
remain excluded because they risk changing bits. Use the existing serial
field and full-composite references to verify the optimized loop structure.
Profile the remaining bevel stages before deciding whether further parallel
execution there is useful; keep their numerical algorithms unchanged.

Stage timings after the vectorizable blur showed roughly 0.33 seconds per
layer in matte/distance/height and 0.15 seconds in shading. Parallelize the
independent columns of the first Euclidean distance pass, then independent
rows of the second, retaining their original order and the same one-dimensional
transform. Store first-pass columns contiguously to avoid strided writes.
Compute shading rows and signed-distance/height conversions independently in
parallel. The test-only serial selector also selects the former distance
transform and serial shading, so the full-composite oracle covers these
changes as well as blur. Add direct bitwise distance-field comparisons.

The shared Gaussian row sums select AVX2 at runtime on supported x86 CPUs,
otherwise retaining the portable loop. The target-feature functions contain
the same ordered multiply and add expressions; they enable wider SIMD across
pixels without FMA or reordered tap reductions. Apply this to both floating
field and existing 8-bit Gaussian row sums: profiling identifies Inner Glow,
which uses the 8-bit helper, as another substantial part of cold composition.
Direct row tests compare all three sample/accumulator type combinations with
scalar expressions, including vector tails; existing filter digest tests
guard the unchanged bytes of the 8-bit filter callers.

### Test the old algorithm directly, then test render integration

Keep the former serial convolution as a test-only reference in the filters
crate. Compare `to_bits()` for deterministic fields at zero, fractional, small,
and maximum caller radii, including negative finite samples, clamped borders,
large support, empty fields, and single-axis images. Exercise the shared helper
in local Rayon pools with one and multiple workers against this reference.
This provides serial execution for tests without a production mode flag.

Add a generated render regression against the former blur, in addition to the
worker-count comparison. Retain a test-only copy of the former render blur
with its original radius argument and internal radius-to-sigma mapping. A
`cfg(test)` scoped selector at the three bevel blur sites chooses that reference
before the replacement path converts radius to sigma. The reference must not
reuse the new helper or its call-site conversion: otherwise a wrong or duplicate
conversion could affect both sides and escape the test. Keep this a local test
hook with no production configuration or generalized rendering interface.

Use a thread-local selector restored by a scope guard, set inside the Rayon
pool's `install` closure. Generate small documents of at most 128 rows: the
current compositor builds their effect passes synchronously on that thread,
while the replacement blur still schedules rows on the chosen pool. This
avoids a process-global test switch affecting unrelated concurrent tests.

For generated asymmetric masked layers, cover every supported style and
technique with softening off and on. Compare complete composite buffers from
the former-blur path with the replacement path in one-worker and multi-worker
pools; retain the direct worker-count comparison. Include rounding and
softening radii large enough to expose an omitted or duplicate sigma conversion.
Use fresh documents and clear effect caches between runs so a cached result
cannot bypass either blur path. Assert that the reference and replacement
paths actually execute on cases with enabled rounding or softening.

This checks both helper arithmetic and its integration with all three call
sites. It needs no pre-change build or newly recorded golden baseline, so tests
and implementation can be finished before the single final compilation. Retain
the existing bevel, region, layer-effect, and filter checks without changing
baselines.

### Use an opt-in external-input timing test

Add ignored `bevel_cold_composite_profile` under the render integration tests.
Read the input via `PICTURA_BENCH_PSD`, fail clearly when explicitly run without
an input, decode once, then time exactly the first `composite_rgba` call. Use
`black_box` to retain the output. Do not prewarm the compositor or its effect
cache. Report read/decode separately from composite time. Record optimized
profile, actual Rayon worker count, and elapsed times without printing the
input identifier. The acceptance invocation uses a fixed `RAYON_NUM_THREADS`
and asserts composite time below four seconds; do not register this as a
default CI performance gate.

Run baseline and changed revisions in fresh processes with the same profile,
worker count, and input, after implementation is complete. Use three runs per
revision and report each duration; do not present a warm-cache run as cold.
The issue's approximately 10-second baseline and 3-second prototype are
reported evidence until independently reproduced. Committed artifacts and
later PR text must omit the local input's filename and path. Correctness
fixtures are generated and need no external input.

## Risks / Trade-offs

- Floating-point drift -> compare sample bits against the former serial loop;
  preserve intermediate narrowing and tap order, with no accepted golden delta.
- Scheduling overhead on small planes -> keep work partitioned by rows, using
  the existing pool; measure the targeted large-radius case.
- Other serial bevel stages limit speedup -> measure blur, distance, and
  shading separately before further optimization; retain their arithmetic
  and compare complete composites against serial references.
- Timing depends on CPU availability and build profile -> use the same fixed
  settings, isolated fresh processes, and opt-in hardware-specific acceptance.
- External workload is unavailable in CI -> generated inputs enforce
  correctness; explicit local profiling supplies performance evidence.
- The existing main layer-effects spec contradicts itself about Chisel
  techniques (it both describes distinct roundings and says they render as
  Smooth) -> preserve current code and its distinct-technique test. This
  performance change does not resolve that pre-existing contract conflict.

## Migration Plan

No persisted data or UI migration. Write regression checks first, replace the
private blur, finish all code, then compile with at most 80% of available CPUs
and run the focused checks. Rebuild only after a code fix. Roll back by
reverting the helper/call-site replacement together. Validate the change and
archive it in the shipping PR after implementation and verification finish.
