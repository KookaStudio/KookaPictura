# Tasks

## 1. Test plan

- [x] 1.1 Write `test-plan.md` with one row per spec scenario, naming the tests this list adds, and verify `python3 scripts/check-test-plan.py` reports only the not-yet-written tests

## 2. Adjustment kernels (`pictura-adjust`)

- [x] 2.1 Add `rayon = "1"` to `crates/pictura-adjust/Cargo.toml` (already in the dependency tree via filters/render/codec) and verify `cargo nextest run -p pictura-adjust` still passes
- [x] 2.2 Add the `#[cfg(test)] FORCE_SERIAL` switch, a test-only parallel-branch counter, and the chunked helpers to `common.rs` (`map_lut`/`map_float` over `par_chunks_mut`, plus `map_rgb` zipping the three planes); add `parallel_matches_serial` in `src/tests.rs` and verify it passes, including that the parallel branch runs when the switch is off
- [x] 2.3 Route the `map_lut`/`map_float` call sites through the helpers and parallelise `lut.rs`'s Color Lookup with `map_rgb`; verify `levels_identity_and_clamps`, `curves_identity_and_control_point`, `bc_identity_both_modes_and_legacy_clip`, `exposure_identity_and_ev_gain`, `posterize_levels_and_idempotence`, `invert_known_and_involution` and the Color Lookup tests pass unchanged
- [x] 2.4 Convert the `color.rs` kernels (hue/saturation, vibrance, color balance, black & white, photo filter, channel mixer, selective color) to `map_rgb`; verify `tests/oracle.rs::hue_saturation_properties` and the unit tests in `src/tests.rs` pass unchanged
- [x] 2.5 Convert the `tonal.rs` per-pixel kernels (shadows/highlights, desaturate, threshold, gradient map) to `map_rgb`, leaving the histogram reductions serial; verify `tests/shadows_highlights.rs` and `src/tests/gradient_map.rs` pass unchanged
- [x] 2.6 Parallelise the `auto.rs` apply loops while keeping the histogram and mean reductions serial; verify the auto tests pass unchanged
- [x] 2.7 Parallelise `replace_color` by rows (it derives `x`/`y` from the pixel index) and `match_color` through `map_rgb` (pointwise); verify `crates/pictura-adjust/tests/replace_color.rs` and `match_color.rs` pass unchanged
- [x] 2.8 Verify `cargo nextest run -p pictura-adjust` passes, including the `oracle` differential tests

## 3. Layer-effect mattes (`pictura-render`)

- [x] 3.1 Blur the matte as one plane in `blur_matte` via `pictura_filters::kernel::gaussian_blur_planes`; add `one_plane_matte_blur_matches_three_planes` over random mattes and sizes and verify it passes
- [x] 3.2 Extract the duplicated mask multiply into one row-parallel helper in `layer_effects/mod.rs` (taking the matte, width, origin, clip, layer and document, reading `composite_rows::FORCE_SERIAL`), replace the eight call sites in `satin`, `glows`, `shadows`, `bevel`, `overlays` and `strokes`, and verify `cargo nextest run -p pictura-render` passes
- [x] 3.3 Add `every_effect_parallel_matches_serial` covering drop shadow, inner shadow, outer glow, inner glow, bevel, satin, overlay and stroke, and verify it passes
- [x] 3.4 Verify `cargo nextest run -p pictura-render` and `cargo test -p pictura-render --test document_oracle resize_document_preserves_structure_and_composite` pass

## 4. Clipping and the adjustment-layer conversion (`pictura-render`)

- [x] 4.1 Parallelise the clipping lerp with `par_iter_mut().zip(...)` and the coverage collect with `par_iter().map(...).collect()`, both honouring `FORCE_SERIAL`; add `clipping_parallel_matches_serial` and verify it passes alongside `a_region_composite_matches_the_full_one`
- [x] 4.2 Parallelise the canvas-to-`u8` fill in `composite_native.rs` behind `FORCE_SERIAL`, add `short_region_composite_matches_serial` for a region of 128 rows or fewer, and verify it passes
- [x] 4.3 Verify `cargo test -p pictura-render --test clipping_oracle` and `cargo nextest run -p pictura-render` pass

## 5. Display colour transform (`pictura-color`, `pictura-codec`)

- [x] 5.1 Build the display transform with `Transform::new_flags_context(..., Flags::NO_CACHE)` and change `ByteTransform`'s field to `Transform<u8, u8, GlobalContext, DisallowCache>`; add `byte_transform_cache_flag_does_not_change_output` and verify `buffer_to_srgb_converts_a_profiled_document` still passes
- [x] 5.2 Rewrite `convert_planes_with` to interleave, transform (whole-pixel chunks) and de-interleave in parallel chunks, and to build the result with `Plane::build`, copying the alpha plane only when `channels == 4`; add `parallel_display_transform_matches_serial` for 3- and 4-channel buffers and verify it passes
- [x] 5.3 Verify `cargo nextest run -p pictura-codec` and `cargo test -p pictura-codec --test icc_oracle` pass

## 6. Integration and repository contracts

- [x] 6.1 Run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run --workspace` and `cargo test --workspace --doc`, and verify all pass
- [x] 6.2 Update `ROADMAP.md` under "Engine and performance" with the shipped parallel CPU kernel entry linking #1011, and verify no remaining `[ ]` item lacks an issue link
- [x] 6.3 Archive the change with `openspec archive parallelise-cpu-kernels`, then run `bash scripts/verify-fast.sh` and `openspec validate --all --strict`, and verify both pass
