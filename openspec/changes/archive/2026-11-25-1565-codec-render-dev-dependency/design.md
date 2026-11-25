# Design

## Context

See `proposal.md` for motivation. The codec manifest dev-depends on render, while the render manifest depends on codec. Current `origin/master` has ten codec test files with render references, more than the eight listed in the issue. The references fall into three groups: compositor output, codec encoders re-exported by render, and renderer-owned document mutations.

## Goals / Non-Goals

**Goals:**
- Remove the codec-to-render development edge and its lockfile entry.
- Keep each behavioral assertion in the crate that owns the behavior.
- Preserve the exact test inventory and all existing assertions.
- Keep new and moved files below repository size caps.

**Non-Goals:**
- Change production APIs or rendering behavior.
- Move document operations into `pictura-core`.
- Add dependencies or run verification under this task's constraints.

## Decisions

### Use classification

| Current codec file | Render use | Disposition |
|---|---|---|
| `tests/content_constructors.rs` | Output: `composite_rgba` | Move two tests to render. |
| `tests/streamed_save.rs` | Output fixture: `composite_tiled` | Build the same tiled composite through core. |
| `tests/layer_content.rs` | Encoding: `encode_*`; output: `composite_rgba` | Call codec encoders directly; move the opaque-text output test. |
| `tests/mask_params_oracle.rs` | Mutation: `add_layer`, `add_shape_layer` | Keep codec oracle tests with local `LayerParts` fixtures. |
| `tests/vector_mask_oracle.rs` | Mutation: `add_shape_layer`, `resolve_path_mut`, `set_shape_fill`, `set_shape_stroke`, `add_vector_mask` | Move four render-authored tests to render. |
| `tests/layer_record_flags.rs` | Mutation/output: `add_solid_fill`, `set_layer_shape_paths`, `rasterize_shape`, `rasterize_fill_content`, `add_shape_layer` | Move two render-authored tests to render. |
| `tests/mask_link_state.rs` | Query/mutation: `layer_mask_linked`, `set_layer_mask_linked` | Keep codec tests with direct PSD flag fixtures. |
| `src/tests/color_modes/profile.rs` | Mutation: `add_layer`, `flatten` | Move the flatten-then-convert test to render. |
| `src/tests/depth.rs` | Mutation: `convert_depth_exposure_gamma` | Move the conversion round-trip test to render. |
| `src/layer_content_budget/tests.rs` | Output: `composite_rgba` | Move the admitted-content test to render. |

### Classify encoders as codec APIs

The `encode_*` functions used by `layer_content.rs` are defined in `pictura-codec` and re-exported by render. Those calls will name `pictura_codec` directly. Moving the encoders would add churn without removing a dependency.

Alternative: move the encoder tests to render. Rejected because the tests assert codec decode contracts, not renderer behavior.

### Move output and render-mutation tests as units

Tests whose subject is compositor output, rasterization, shape authoring, flattening, or native-depth conversion will move to render integration tests. Moving the whole test keeps its codec round-trip assertions and avoids splitting one test into several, which would change the test count.

The streamed raster-save heap test is different. It uses `composite_tiled` only to create a tiled composite fixture before measuring the codec save. It will clone the existing composite and tile its plane through `pictura_core::Plane::tile`, preserving the tiled input without linking render.

Alternative: move the heap test to render. Rejected because the measured behavior is the codec's streamed raster writer.

### Keep codec round trips in codec with local fixtures

Mask parameter, vector-mask, layer-record, and mask-link tests that assert PSD/PSB bytes will stay in codec. Simple layer and shape setup will use test-only builders over `LayerParts`, `encode_fill_content`, `encode_vector_mask`, and `edit_layer`. Mask-link toggles will set the PSD flag directly. These fixtures do not become production APIs.

Alternative: copy render mutation helpers into codec. Rejected because it would duplicate document-operation ownership and make the dependency removal misleading.

### Add a manifest assertion without adding a test

The existing render feature-gate manifest test will also read the codec manifest and assert that its dev-dependency table has no render edge. This keeps the test count unchanged while making the removed edge mechanically visible.

Alternative: add a standalone test. Rejected because the task requires preserving the test count.

## Risks / Trade-offs

- Moved tests can import the wrong fixture path → use render-relative paths to `../pictura-codec/tests/fixtures` and review every moved include.
- Local fixture builders can drift from render authoring → restrict them to the exact fields each codec assertion needs and keep render-owned authoring tests in render.
- A source-only move can hide a missing import → the full build and both crate suites compile and pass every moved test.
- Manual lockfile editing can diverge from Cargo → remove only the codec package's render entry; `cargo tree -p pictura-codec --edges dev` confirms the graph.

## Migration Plan

1. Move renderer-output and render-mutation tests to render integration suites.
2. Rebuild codec-only fixtures with codec/core APIs and point encoder calls at codec.
3. Remove the codec dev-dependency and lockfile edge.
4. Extend the existing manifest test and update the test plan.
5. Review the diff for test-count and assertion preservation.
6. Format, build, lint, and run the codec and render suites; record the results in `test-plan.md`.
