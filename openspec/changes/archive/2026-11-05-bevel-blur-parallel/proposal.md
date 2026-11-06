# Proposal

## Why

Issue #1178 identifies bevel's private serial Gaussian convolution as the main
cost of a cold CPU composite with large bevel sizes. The reported prototype
reduces the cold composite from about 10 seconds to about 3 seconds while
preserving pixels; this change makes that optimization shared and testable.

## What Changes

- Provide a shared floating-point plane blur in `pictura-filters::kernel`
  that preserves bevel's `f64` weights and accumulation, with rows evaluated
  in parallel.
- Route bevel's height rounding and separate highlight/shadow softening
  through that helper and remove its private `blur_f32` implementation.
- Prove bitwise field equivalence against the former serial algorithm and
  unchanged composite bytes on generated bevel cases.
- Following profiling and the user's request for further speed, vectorize
  ordered blur sums across pixels and parallelize independent distance and
  shading passes without changing their numerical models.
- Measure first-composite latency separately from decoding and warm-cache
  compositing, with an opt-in external-input profiling test.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `compositing/cpu-kernel-parallelism`: Extend parallel execution guarantees
  to bevel's floating-point Gaussian passes, including their precision and
  a hardware-specific cold-composite acceptance measurement.

## Impact

- `crates/pictura-filters/src/kernel.rs`: public helper and serial-reference
  equivalence tests; reuse existing Rayon dependency.
- `crates/pictura-render/src/layer_effects/bevel.rs`: replace three blur calls
  and remove the duplicate production implementation.
- Render tests: generated composite regression coverage and an ignored
  external-input profiling test. No new dependency, UI, file-format, or
  approximation change; existing `f32`-accumulating filter callers retain
  their arithmetic.
- Changes to the distance/lighting models, effect scheduling, and caches remain
  outside this issue's scope. No `docs/` edits are planned.
