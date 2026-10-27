# Proposal: shared-mask-ops

## Why

Mask algorithms are implemented four times across the engine. `Select > Similar`
is O(pixels × distinct selected colours), Smooth is O(radius²) per pixel,
Expand/Contract/Border are O(radius) per pixel, and the floods walk one queue
entry per visited pixel. On a 6000×4000 document the slowest copy sets the
user-visible latency (issue #971).

## What Changes

- **One shared `mask_ops` module in `pictura-core`**:
  - separable sliding-window dilate/erode whose cost is independent of the
    radius, with an explicit border policy (`Zero` for selection morphology,
    `Clamp` for selection Smooth, filters, paint and layer effects) and a generic
    sample type so the f32 layer matte uses the same kernel as the u8 selection;
  - a sliding-histogram median that returns the same bytes as sorting each
    clamped window;
  - a scanline four-connected flood used by Magic Wand, Grow, and the Paint
    Bucket, so the frontier is held as row spans instead of one queue entry per
    pixel.
- **A colour matcher for `Select > Similar`**: a 2^24-bit RGB cube built once
  from the selected colours and dilated by the tolerance, so each pixel costs
  one lookup instead of a scan of every selected colour.
- **Migrate every call site**: selection Modify (Expand, Contract, Border,
  Smooth) and Refine Edge's Smooth; Maximum/Minimum and Median filters; the
  Paint Bucket flood and selection stroke band; the layer-effect spread and
  choke matte.
- **`pictura-core` gains `rayon`** for row-parallel kernels. Rayon is already in
  the dependency graph through three other engine crates, so no new crate enters
  the lockfile.
- **No behavioural change.** Every operation keeps the bytes or selection it
  produces today; the existing filter and selection oracles and the recorded
  layer-effect spread/choke digests stay byte-identical.

## Capabilities

### New Capabilities

- `imaging/mask-ops`: the shared exact-morphology, sliding-median, scanline-flood
  and colour-matcher contracts, including their border policies, equivalence to
  the naive forms, determinism, and radius/colour-count-independent cost.

### Modified Capabilities

- None. `tools/selection-model`, `tools/selection-tools`,
  `imaging/other-filters`, `imaging/noise-filters`, `tools/paint-bucket-tool`
  and `compositing/layer-effects` keep their current requirements; this change
  only supplies a new implementation that must reproduce them.

## Impact

- `crates/pictura-core/Cargo.toml`: adds `rayon`; new `src/mask_ops/`
  submodules; `lib.rs` re-exports the module.
- `crates/pictura-select/src/lib.rs`: `Selection::{expand,contract,border,smooth}`
  delegate to `mask_ops`; `median` and `morph` move out; `magic_wand`, `grow` and
  `similar` use the shared flood and matcher (`src/matcher.rs`). Refine Edge's
  Smooth reaches the shared median through `Selection::smooth`.
- `crates/pictura-filters/src/other.rs`: Maximum/Minimum delegate to the shared
  clamp-border morphology.
- `crates/pictura-filters/src/noise.rs`: `window_median` moves to `mask_ops`.
- `crates/pictura-paint/src/bucket.rs`: flood uses the scanline fill; the private
  `dilate`/`erode` pair for the selection stroke delegates to `mask_ops`.
- `crates/pictura-render/src/layer_effects/mod.rs`: `axis_max`/`axis_min` and
  their wrappers delegate to the f32 morphology.
- New `#[ignore]` profile at 6000×4000; randomised core-kernel differential tests
  in `crates/pictura-core/src/mask_ops/tests.rs`; the Similar/matcher differential
  tests in `crates/pictura-select/`.
- Parallelism and memory: `pictura-select` operations become row-parallel;
  `similar` allocates a 2 MiB colour cube; flood frontier memory drops from
  one entry per visited pixel to row spans.
