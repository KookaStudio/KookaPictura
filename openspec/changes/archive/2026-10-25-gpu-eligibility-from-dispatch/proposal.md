# Proposal

## Why

The GPU compositor decides what it can composite from a hand-maintained
denylist (`check_supported`), and that list has drifted from the CPU dispatch it
mirrors. Two cases now get through:

- A pixel-carrying smart object with enabled smart filters is composited on the
  GPU without its filters.
- A 16- or 32-bit document is composited on the GPU at 8-bit, while the CPU
  oracle uses its native samples.

In both cases the GPU silently replaces the CPU oracle, and its result feeds
Magic Wand, Color Range, Refine Edge, and export (#745). No CI run would catch
this or the next drift: every GPU parity test returns early on hosted runners
and is reported as passed (#746).

## What Changes

- Classify each layer's content into a single content route. The CPU compositor
  selects its content path from that route, and GPU eligibility is decided by
  matching on the same route. A content path added to the CPU without a GPU
  decision becomes a compile error instead of a silent divergence. Effects,
  Dissolve, Blend If, knockout, clipping, and text gamma stay separate attribute
  checks, applied in a fixed order.
- Decline the GPU for documents read at 16 or 32 bits and for layers whose
  enabled smart filters the CPU would apply. Each gets its own `GpuError`
  reason.
- Add a test-only build feature that lets the GPU probe accept a software
  (CPU) Vulkan device. Production builds keep rejecting software devices.
- Add a Linux CI job that installs a software Vulkan driver and runs every
  GPU-dependent test on it, including the ImageMagick fixture scenes run through
  the active backend.
- GPU-dependent tests no longer return early and pass without a device. They
  are ignored by default, so the unified report counts them as skipped, and they
  fail when run without a device.

## Capabilities

### New Capabilities

- `verification/gpu-parity-ci`: the software-device test feature, the CI job
  that runs GPU parity on it, fixture-scene parity through the active backend,
  and honest skip reporting for GPU-dependent tests.

### Modified Capabilities

- `compositing/gpu-compositing`: adds eligibility from the CPU compositing
  route, CPU-only high-bit-depth documents, and CPU-only smart-filtered layers.
  Changes "The GPU compositor is validated against the CPU oracle before use" so
  that `GpuError` may gain decline reasons.

## Impact

- `crates/pictura-render/src/composite.rs` (`composite_layer_inner`),
  `smart_filter.rs`, `smart_source.rs`, and `text_render.rs`: each try-and-fall-back
  helper is split into a pure classification step and a render step.
  `composite.rs` is close to its file-size cap, so the route type goes in its
  own module.
- `crates/pictura-render/src/gpu/mod.rs`: `check_supported` matches on the
  route, and `GpuError` gains `UnsupportedDepth` and `UnsupportedSmartFilter`.
- `crates/pictura-render/src/gpu/backend.rs`: `accelerates()` honours the
  `software-vulkan` cargo feature. The feature is off by default, and no
  workspace manifest enables it.
- `crates/pictura-render/tests/gpu_parity/*`, `gpu_filter_parity.rs`, and the
  four in-crate `#[ignore]` GPU tests: the early-return branch is removed and
  device-dependent tests are marked ignored.
- `.github/workflows/ci.yml`: a new `gpu-parity` job (`mesa-vulkan-drivers`);
  `.config/nextest.toml`: a `gpu` profile.
- No new crate dependency. The CI job adds one apt package.
