# Proposal

## Why

The facts the engine holds about each filter kind — which engine applies it, whether its kernel moves the alpha plane, how wide a preview apron it needs, whether its row-parallel CPU path beats the GPU round trip — are spread across seven variant-keyed matches in two crates, and several of those matches end in a wildcard arm. A new `Filter` variant therefore compiles with no dispatch, no apron and no alpha decision, and the gap surfaces only at runtime. The coverage tests are already wrong about it: the oracle mapping test asserts "one mapping row per Filter variant … full enum coverage" over a hand-typed list of 56 names while the enum has 112 variants.

## What Changes

- `pictura-filters` gains a single home for the per-variant facts: `crates/pictura-filters/src/filter/props.rs` exposes them as methods on the type — `Filter::moves_alpha()`, `Filter::preview_apron()` and `Filter::cpu_outruns_gpu()`. The kind-level decisions are `match`es over the filter kind that name every variant and have no wildcard arm; `Filter::preview_apron()` is a value-level match that also names every variant. The value-level methods delegate to the kind-level match (Smart Blur's mode is the one value-dependent exception), so the compiler rejects a new variant until the fact is decided. (`matches!` is not usable here: it expands to a match with a `_` arm and therefore enforces nothing.)
- `pictura-filters` gains a generated variant list. A `filter_variants!` macro lists the variants once and generates a `FilterKind` enum, its `ALL` list, `Filter::kind()`, `FilterKind::name()` and the engine fact — a `FilterEngine` enum (`Native`, `Ported`) with `FilterKind::engine()` and `Filter::engine()`. The table lists each variant with the engine that applies it and with its pattern shape, because `Filter` has three shapes (99 named-field variants, the tuple variants `ReduceNoise` and `HdrToning`, and eleven unit variants) and one pattern does not fit all of them. The generated match is exhaustive over `Filter`, so the compiler rejects the list in both directions when it drifts from the type: a variant missing from the list is a non-exhaustive-pattern error, a name in the list that is not a variant is a no-such-variant error. No new dependency is added.
- The engine that applies a variant becomes one decision with three statements of the partition, all in `pictura-filters`: the macro table, the native implementation match in `filter/apply.rs`, and the ported implementation match in `photorust/dispatch.rs`. Each names every variant, so a variant omitted from both implementations is a compile error rather than a runtime gap, and `apply` routes on the property so a misplacement fails with `FilterError::Unsupported` instead of running the other engine's kernel.
- `pictura-render` stops owning two of the facts. `pictura_render::preview_apron` and the crate-private `cpu_outruns_gpu` are replaced by the `Filter` properties; the render crate and the application ask the filter instead of a render-side table.
- `pictura_filters::filters_alpha` becomes `Filter::moves_alpha()`; the free function is dropped.
- The oracle coverage test becomes honest. The mapping table grows from 56 rows to one row per variant, and the test iterates the generated variant list instead of a hand-typed one, so it can no longer claim full coverage while covering half the enum. The digest and the ported-contract tests gain the same per-variant completeness decision.
- The three capability specs that restate these lists in prose (`imaging/filter-application`, `compositing/gpu-filter-acceleration`, `imaging/parallel-filter-kernels`) bind to the new properties instead, so the prose stops being a fourth hand-maintained copy.

## Capabilities

### New Capabilities

- `imaging/filter-engine-facts`: the per-variant engine facts as exhaustive properties of the `Filter` type — the generated variant list, the compiler-enforced decision for each fact, the engine that applies a variant, and the coverage tests that iterate the generated list.

### Modified Capabilities

- `imaging/filter-application`: the alpha-preservation requirement names the moving-kernel set by the `moves_alpha` property instead of restating the list in prose, and the region-bounded requirement names the apron property now that `preview_apron` lives with the filter rather than in the render crate.
- `compositing/gpu-filter-acceleration`: the default-on GPU requirement names the CPU-faster kernels by the `cpu_outruns_gpu` property instead of restating the list in prose.
- `imaging/parallel-filter-kernels`: the opaque-layer scenario names the current `moves_alpha` property; it still refers to the long-removed `filter_preserves_opacity`.

## Impact

- `pictura-filters` becomes the crate that owns the facts: `crates/pictura-filters/src/filter/props.rs` (new), `crates/pictura-filters/src/filter/variants.rs` (new), `crates/pictura-filters/src/filter/apply.rs`, `crates/pictura-filters/src/filter/types.rs`, `crates/pictura-filters/src/photorust/dispatch.rs`, `crates/pictura-filters/src/filter.rs` (the module list and the re-exports) and `crates/pictura-filters/src/lib.rs` (the re-exports and the crate doc comment that names `filters_alpha`).
- Public API of `pictura-filters`: `filters_alpha` is replaced by `Filter::moves_alpha()`; `Filter::preview_apron()`, `Filter::cpu_outruns_gpu()`, `FilterKind`, `FilterEngine`, `Filter::kind()`, `Filter::engine()`, `FilterKind::engine()`, the kind-level facts and the variant list are added. `pictura-render` keeps its `apply_filter` / `apply_filter_region` / `apply_adjustment_region` signatures.
- `crates/pictura-render/src/filter.rs` and `crates/pictura-render/src/gpu_filter/mod.rs` ask the filter for the facts; `pictura_render::preview_apron` is dropped from the crate's re-exports and the three application call sites (`impl_filters.rs` twice, `canvas_proxy.rs` once) call the property.
- Tests: `crates/pictura-filters/tests/oracle/mapping.rs` grows to one row per variant; `crates/pictura-filters/tests/ported.rs` and `crates/pictura-filters/tests/kernel_digests.rs` gain a per-variant completeness decision; a per-variant instance list is assembled in the shared `crates/pictura-filters/tests/common/` module so the facts can be tested against every variant; `crates/pictura-render/src/filter/tests.rs` follows the rename.
- Docs: `docs/GLOSSARY.md` names `filters_alpha` in its apply-filters entry and cites a line in `filter/apply.rs`; `docs/dev/m6c-filter-integration.md` and `docs/dev/STATE.md` name `preview_apron`. The entries are updated in a separate commit carrying `TASK-ALLOWS-DOCS`.
- No new dependency, no change to any filter's rendered output: the facts keep the values they have today.
