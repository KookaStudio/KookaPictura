# Tasks

## 1. Test plan and test scaffold

- [x] 1.1 Write `test-plan.md` in this change folder mapping every spec scenario to an automated test or a justified manual check, and verify `python3 scripts/check-test-plan.py` reports a plan row for every scenario — the remaining `no such test` failures are the tests tasks 1.2 to 1.8 add, and the check passes once they exist
- [x] 1.2 Write `filter_reports_its_own_facts` in `crates/pictura-filters/tests/` and verify it fails before the facts exist
- [x] 1.3 Write `generated_variant_list_names_every_variant_once` in `crates/pictura-filters/tests/` and verify it fails before the generated list exists
- [x] 1.4 Write `each_variant_is_applied_by_the_engine_its_property_names` in `crates/pictura-filters/tests/` and verify it fails before the engine property exists
- [x] 1.5 Write `mapping_table_holds_one_row_per_variant` and `every_mapping_row_states_an_oracle_or_states_none` in `crates/pictura-filters/tests/oracle/mapping.rs` and verify they fail before the rows exist
- [x] 1.6 Write `ported_cases_cover_every_ported_variant` in `crates/pictura-filters/tests/ported.rs` and verify it fails before the check exists
- [x] 1.7 Write `every_variant_has_a_digest_decision` in `crates/pictura-filters/tests/kernel_digests.rs` and verify it fails before the decision exists
- [x] 1.8 Write `moving_kernel_fixture_matches_the_moving_set`, `offset_in_transparent_mode_exposes_clear_pixels` and `picture_frame_adds_opacity_over_a_clear_pixel` in `crates/pictura-render/src/filter/tests.rs`; verify the fixture test fails before the facts exist and the other two fail only if the existing behaviour is broken
- [x] 1.9 Assemble one default instance per `Filter` variant in the shared `crates/pictura-filters/tests/common/` module (`instances.rs`) — reuse the values `ported.rs`, `kernel_digests.rs` and `profile.rs` already build (104 of the 112) and add the eight the crate has none of (`Displace`, `Liquify`, `Glass`, `DiffuseGlow`, `DifferenceClouds`, `Lighting`, `SmartSharpen`, `HdrToning`) — and verify the list holds one instance of every variant in the generated list (the generated list arrives in task 2.1; until then the check is expected to fail)

## 2. The generated variant list

- [x] 2.1 Add `crates/pictura-filters/src/filter/variants.rs` with a `filter_variants!` macro that lists the 112 variant names once — each with its pattern shape (`Filter::GaussianBlur { .. }`, `Filter::ReduceNoise(..)`, `Filter::Average`) and the engine that applies it — and expands to a `FilterKind` enum, `FilterKind::ALL`, `Filter::kind()` and `FilterKind::name()`; register the module in `crates/pictura-filters/src/filter.rs`, export `FilterKind`, `FilterKind::ALL`, `Filter::kind()` and `FilterKind::name()` from `pictura-filters`'s public surface (`crates/pictura-filters/src/filter.rs` and `crates/pictura-filters/src/lib.rs`), and verify the crate compiles and `generated_variant_list_names_every_variant_once` passes
- [x] 2.2 Generate `FilterEngine` (`Native`, `Ported`), `FilterKind::engine()` and `Filter::engine()` from the macro table — the table is the statement of the partition — export them from `pictura-filters`'s public surface, and verify the crate compiles; `each_variant_is_applied_by_the_engine_its_property_names` passes once task 4.1 routes `apply`
- [x] 2.3 Verify the integration tests in group 1 can name `FilterKind`, `FilterKind::ALL`, `FilterEngine`, `Filter::kind()`, `FilterKind::name()`, `FilterKind::engine()` and `Filter::engine()` from `pictura-filters`'s public surface

## 3. The per-variant facts

- [x] 3.1 Add `crates/pictura-filters/src/filter/props.rs` with `FilterKind::moves_alpha()` as an exhaustive match over the kind and `Filter::moves_alpha()` delegating to it except for Smart Blur's mode; register the module in `crates/pictura-filters/src/filter.rs` and verify the moving set is still the same 44 variants (the fixture comparison is task 6.4)
- [x] 3.2 Move the preview apron and its private `path_reach` helper into `props.rs` as `Filter::preview_apron()`, an exhaustive match keeping every value it has today (an `i32` margin, the default 128); verify the render crate's apron tests still pass unchanged and `filter_reports_its_own_facts` passes
- [x] 3.3 Move the CPU-faster decision into `props.rs` as `FilterKind::cpu_outruns_gpu()`, an exhaustive match over the kind keeping the same 11 variants and the `ponytail:` note about the one-machine crossover, with `Filter::cpu_outruns_gpu()` delegating to it; verify `cpu_preferred_kernels_skip_the_gpu` still passes
- [x] 3.4 Verify the engine property and the two implementation matches agree on every variant — each variant named by exactly one of them — and that a variant named by neither fails to compile

## 4. The dispatch

- [x] 4.1 Route `crates/pictura-filters/src/filter/apply.rs` through `Filter::engine()` — the native match names every variant (the 39 native arms plus a grouped arm for the 73 the ported engine owns, which returns `FilterError::Unsupported`), and the ported dispatch's `None` becomes `FilterError::Unsupported` when the property routed the variant there, so a misplacement never falls back to the other engine; verify every filter still applies as before and `each_variant_is_applied_by_the_engine_its_property_names` passes
- [x] 4.2 Collapse `crates/pictura-filters/src/photorust/dispatch.rs`'s `plan` / `plan_strokes` / `plan_sketch` chain into one match that names every variant — the 73 ported arms plus a grouped arm for the 39 the native kernel owns, which returns `None` — keeping the per-family bodies in the existing `photorust` modules; verify the ported contract test and the recorded kernel digests are unchanged
- [x] 4.3 Verify `bash scripts/check-file-size.sh` passes with `photorust/dispatch.rs` under the 1200-line cap (1103 today) and `props.rs` and `variants.rs` under it too, splitting the per-family bodies into a sibling module if it is not

## 5. The cross-crate move

- [x] 5.1 Replace `pictura_filters::filters_alpha` with `Filter::moves_alpha()` at its four call sites — the ported engine's alpha write-back (`photorust/dispatch.rs`), the GPU plane count (`pictura-render/src/gpu_filter/mod.rs`), the render crate's alpha test (`pictura-render/src/filter/tests.rs`) and the ported contract test (`crates/pictura-filters/tests/ported.rs`) — drop the free function and the doc comments that name it (`crates/pictura-filters/src/filter/types.rs`, `crates/pictura-filters/src/filter.rs`, `crates/pictura-filters/src/lib.rs`, `crates/pictura-filters/src/filter/apply.rs`, `crates/pictura-filters/src/photorust/dispatch.rs`, `crates/pictura-render/src/gpu_filter/mod.rs`, `crates/pictura-render/src/filter/tests.rs`, `crates/pictura-filters/tests/ported.rs`); verify the engine's tests pass
- [x] 5.2 Delete `pictura_render::preview_apron` and the render crate's `cpu_outruns_gpu` helper and point the three application call sites (`impl_filters.rs` twice, `canvas_proxy.rs` once) at `Filter::preview_apron()`; update the render crate's own call sites (`crates/pictura-render/src/filter.rs` and `crates/pictura-render/src/filter/tests.rs`); verify the application crate builds and the preview tests pass
- [x] 5.3 Drop `preview_apron` from `pictura-render`'s re-exports and verify no caller of the old name is left in the workspace

## 6. The coverage tests

- [x] 6.1 Add the 56 missing rows to the oracle mapping table — one per variant, each naming the ImageMagick operator that is the variant's oracle or stating that it has none with a non-empty note saying why — and replace the hand-typed 56-name `all` list, the `MAPPING.len() == 56` assertion and the `NO_EQUIVALENT` exact-list assertion the existing `mapping_marks_no_equivalent_operators` test carries with the per-row checks; verify `mapping_table_holds_one_row_per_variant` and `every_mapping_row_states_an_oracle_or_states_none` pass
- [x] 6.2 Add the ported-case coverage check to `crates/pictura-filters/tests/ported.rs`, add the two missing ported cases (`Displace` and `Shear`), and verify `ported_cases_cover_every_ported_variant` passes
- [x] 6.3 Add the per-variant digest decision to `crates/pictura-filters/tests/kernel_digests.rs` and verify `every_variant_has_a_digest_decision` passes with the recorded digests unchanged
- [x] 6.4 Add the moving-fixture comparison to `crates/pictura-render/src/filter/tests.rs` and verify `moving_kernel_fixture_matches_the_moving_set` passes

## 7. Docs

- [x] 7.1 Update the `docs/GLOSSARY.md` apply-filters entry to name the filter's own alpha fact instead of the removed free function and re-point its source line, and update the `preview_apron` mentions in `docs/dev/m6c-filter-integration.md` and `docs/dev/STATE.md`, in a separate commit carrying `TASK-ALLOWS-DOCS`; verify `bash scripts/guard.sh` passes on the commit

## 8. Integration verification

- [x] 8.1 Run `openspec validate --all --strict` and verify the change and the corpus validate
- [x] 8.2 Run `python3 scripts/check-test-plan.py` and `bash scripts/check-file-size.sh` and verify both pass
- [x] 8.3 Build once, then run `cargo nextest run --workspace` and `ctest --test-dir build -R '^tst_' --output-on-failure`, verifying the engine and GUI layers are green
- [x] 8.4 Run `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D warnings` and verify both pass
