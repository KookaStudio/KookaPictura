# Design

## Context

See proposal.md for why this change exists. These are the parts of the current
code that shape the approach.

- **The CPU decides in four places, not one.**
  - `composite_layer_inner` (composite.rs:314) always composites effects
    around the content: below-content effects first, above-content effects
    after.
  - Between those, it picks the content with a chain of try-and-fall-back
    helpers:
    1. a group (pass-through or isolated);
    2. `decode_layer_fill`;
    3. an undecodable adjustment, which is a no-op;
    4. `composite_smart_filtered_source`;
    5. `composite_smart_source`;
    6. `composite_type_source`;
    7. `composite_pixels`.
  - Clipping is decided in `composite_clipping::composite_siblings` and
    knockout in `composite_knockout::composite_layer`.
  - Blend If, Dissolve, and text gamma are applied inside the blend.
  - Native depth comes from per-sample reads gated on `doc.source_depth`
    (composite_native.rs:52, 121, and `authoritative_plane`).
- **`check_supported`** (gpu/mod.rs:421) re-derives all of that by hand. It
  runs before the availability gate and walks group children.
- **`decode_smart_filter`** models only the Camera Raw filter.
  `composite_smart_filtered_source` takes the route when at least one filter
  is individually enabled and every enabled filter decodes. It then re-renders
  the embedded source and applies the chain under the group's master switch.
- **GPU tests skip silently in four ways:**
  - `is_gpu_gone` in blend.rs, layers.rs, and `common::check_scene_parity`;
  - `if !gpu_available() { return; }` in fresh_white.rs, large.rs, region.rs,
    and `resident_uploads_are_reused_and_never_stale`;
  - `filter_gpu_available()` in gpu_filter_parity.rs;
  - `shared_device()` / `GpuStroke::available()` in gpu_filter/tests.rs and
    gpu/stroke.rs.

  nextest's JUnit output writes ignored tests as `<skipped>`, and
  `report_tests.py` counts them as SKIP.

## Goals / Non-Goals

**Goals:**
- One classification of a layer's content that both compositors match on with
  no wildcard arm.
- GPU parity runs on every PR, and a GPU test that did not run is never green.

**Non-Goals:**
- Implementing smart filters or native depth on the GPU.
- Catching hardware driver bugs (#418).
- Changing `PICTURA_GPU_BENCH` gating, which `gpu-transfer` and
  `gpu-filter-acceleration` specify.

## Decisions

### D1: A content-only `ContentRoute`, and attributes stay attributes

`composite_route.rs`:

```
pub(crate) enum ContentRoute {
    PassThroughGroup, IsolatedGroup,
    Fill(Adjustment),          // decode_layer_fill succeeded
    UndecodedAdjustment,       // adjustment block that does not decode (CPU no-op)
    UndecodedVectorFill,       // vscg present, no adjustment, did not decode
    SmartFiltered,             // smart_filters_apply(layer)
    SmartSource,               // smart object without channel 0
    TypeSource,                // type layer without channel 0
    Pixels,
}
pub(crate) fn content_route(layer: &Layer) -> ContentRoute
```

`composite_layer_inner` keeps the effects bracketing and replaces the
if/else-if chain with an exhaustive `match`. Byte identity rests on two points:

- The predicates are the same expressions as today's guards.
- The render steps keep falling through. `SmartFiltered` runs
  `smart_filtered || smart_source || type_source || pixels`, `SmartSource` runs
  `smart_source || type_source || pixels`, and so on. The earlier helpers would
  have returned false on those layers anyway.

`check_supported` matches the same enum, also with no `_` arm. Before the walk
it rejects a document whose `source_depth.is_some()`. For each visible layer it
then checks, in order:

1. effects;
2. the content route: groups recurse; `Fill` passes only for an adjustment
   block whose kind `adjustment_params` accepts; everything else declines with
   its reason;
3. Dissolve;
4. Blend If, knockout, or clipping;
5. text gamma on a layer with `type_tool`.

`UndecodedVectorFill` declines as `UnsupportedAdjustment`, which keeps today's
"vscg present" decline exact. `Pixels` passes.

*Alternative:* one route per layer that also covers effects and clipping
(rejected). The CPU composites effects around the content and decides clipping
in another function, so such an enum would need dead arms and would not give
the compile-time guarantee.

### D2: Native depth is gated on `source_depth.is_some()`

That mirrors the CPU's adjustment and mask gates exactly. A per-layer
`authoritative_plane` test would miss masks and adjustments, and a "16 or 32"
test would miss documents converted from a non-8-bit source.

### D3: Count declines where they happen

`check_supported` returns the error and `composite_gpu` / `composite_gpu_region`
count it. Counting in `composite_active` would be wrong, because that path
returns early when no device is available. This is shared with the
instrumentation change (its `pictura_render::metrics`).

### D4: `software-vulkan` is a cargo feature

- `pub const SOFTWARE_VULKAN: bool = cfg!(feature = "software-vulkan")`.
- `accelerates()` accepts `DeviceType::Cpu` only when that constant is true.
- No manifest enables the feature. The CI job passes `--features software-vulkan`
  with `-p pictura-render` only.
- `a_software_adapter_is_no_gpu` is `cfg(not(feature))`, and a sibling test
  covers the feature-on case.
- The gpu job also runs clippy with the feature, because the default clippy
  never compiles the `cfg` arm.

### D5: Ignore the device-dependent tests and require the device

`require_gpu()` is a test helper (`tests/gpu_parity/common.rs`, plus a
`#[cfg(test)]` copy in the crate). It panics with `"no usable Vulkan device;
this test is #[ignore]d and needs one (software-vulkan in CI)"` unless
`gpu_available()`. The device-dependent tests call it and are marked
`#[ignore = "requires a Vulkan device; run with --profile gpu --run-ignored all"]`:

| File | Tests |
|---|---|
| gpu_parity/blend.rs | `gpu_matches_cpu_within_one_lsb`, `non_separable_modes_match_cpu` |
| gpu_parity/layers.rs | `groups_and_masks_match_cpu`, `fill_scene_matches_cpu`, `adjustment_layers_match_cpu` |
| gpu_parity/fresh_white.rs | `fresh_white_document_composites_uniform_white_twice` (its CPU arm is moved into an unignored `fresh_white_document_composites_uniform_white_on_cpu`) |
| gpu_parity/large.rs | `document_past_2d_product_limit_returns_too_large` |
| gpu_parity/region.rs | `region_matches_full_slice`, `full_region_equals_full_composite` |
| gpu_filter_parity.rs | `accelerated_filters_match_cpu_within_one_lsb` (with a `require_filter_gpu()` helper) |
| src/gpu/mod.rs | `resident_uploads_are_reused_and_never_stale` |
| src/gpu_filter/tests.rs | `chunked_dispatch_matches_one_submission` |
| src/gpu/stroke.rs | `gpu_stroke_matches_the_cpu_oracle`, `gpu_mask_stroke_matches_the_cpu_oracle` (already ignored; the early return becomes `require`) |

The other tests in those files are device-free and stay as they are, including
`enabled_gpu_matches_availability`, which asserts in both arms. The bench tests
(`*_timing_*`, `large_document_over_1d_limit_composites_on_gpu`,
`full_4000_document_composites_on_gpu`, `large_buffer_over_1d_limit_runs_on_gpu`,
`surface_blur_1024_gpu_beats_cpu`, `median_1024_parity_and_timing`,
`oil_paint_1024_parity_and_timing`) and `composite_profile_{1024,4000}` are
listed by name in the gpu profile's exclusion filter.

### D6: The fixture scenes run from `tests/oracle.rs`

The new test goes in the same binary, which already owns `scene()` and the
fixture loaders. That avoids a module shared between test binaries, which would
trip `dead_code`. It is ignored and device-dependent, and it is selected by
name in the gpu profile.

### D7: The `gpu` nextest profile and CI job

`.config/nextest.toml`:

```toml
[profile.gpu]
default-filter = "package(pictura-render) & (binary(gpu_parity) | binary(gpu_filter_parity) | test(/^gpu(_filter)?::/) | test(=fixture_scenes_match_oracle_on_active_backend) | test(=declined_fixture_scene_equals_oracle)) & !test(/timing|_1024_|over_1d_limit|full_4000|composite_profile/)"
[profile.gpu.junit]
path = "junit.xml"
report-skipped = "ignored"
```

`ci.yml` job `gpu-parity`:

- `needs: changes`, with the same `cpp_only` skip condition as `test`;
- the sccache and rust-cache anchors;
- installs `mesa-vulkan-drivers libvulkan1`;
- `cargo clippy -p pictura-render --all-targets --features software-vulkan -- -D warnings`;
- `cargo nextest run -p pictura-render --features software-vulkan --profile gpu --run-ignored all`;
- uploads `target/nextest/gpu/junit.xml`.

It is not made a required check, because `paths-ignore` skips docs-only PRs.

## Risks / Trade-offs

- [The refactor changes CPU output] → `content_route_matches_the_legacy_chain`
  checks the classification against a verbatim copy of the old if/else chain
  over a corpus covering every route. The render steps themselves do not
  change, and the existing CPU oracle suites run unchanged.
- [`composite.rs` or `gpu/mod.rs` exceed the 1200-line cap] → The enum lives
  in `composite_route.rs` and the eligibility tests in `gpu/eligibility_tests.rs`.
- [Lavapipe rounding fails ±1 LSB] → Triage the shader first. Never widen the
  tolerance per scene without a note in the task result.
- [Developers with a GPU lose the default parity run] → `DEVELOPING.md`
  documents the `gpu` profile command.
- [Global decline counters race in the shared `cargo test` process] → Tests
  assert `>=` deltas.
