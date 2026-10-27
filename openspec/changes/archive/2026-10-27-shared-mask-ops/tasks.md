# Tasks

## 1. Tests first

- [x] 1.1 Add the morphology, median and flood differential tests to the new `pictura-core/src/mask_ops.rs` test module (`morph_radius_zero_is_the_identity`, `morph_zero_border_erodes_from_the_edge`, `morph_clamp_border_keeps_the_edge`, `morph_matches_naive_at_every_radius_and_border`, `median_matches_the_sorted_window`, `median_radius_zero_is_the_identity`, `median_binary_majority_removes_and_fills`, `flood_matches_the_naive_queue`, `flood_multi_seed_matches_the_naive_queue`, `flood_stops_at_the_predicate_boundary`, `flood_frontier_is_bounded_by_spans`, `flood_rejects_an_out_of_plane_seed`, `kernels_agree_under_one_and_many_threads`); verify every name matches `test-plan.md`
- [x] 1.2 Add the matcher and Similar tests to `pictura-select` (`similar_matches_the_per_colour_scan`, `matcher_matches_brute_force_for_every_tolerance`, `similar_zero_tolerance_selects_only_the_exact_colour`, `similar_preserves_soft_selected_coverage`, `similar_with_an_empty_selection_adds_nothing`); map the dimension mismatch to the existing `grow_similar_size_mismatch_errors`; verify every name matches `test-plan.md`
- [x] 1.3 Create `crates/pictura-select/tests/profile.rs` with an `#[ignore]` 6000×4000 profile that prints Similar with 10 and 10 000 reference colours; Smooth at radius 10 and 100; Expand at radius 1 and 100; and the three targets (Similar 10 000 < 100 ms, Smooth r=100 < 1 s, Expand r=100 < 300 ms); verify `cargo nextest run -p pictura-select --run-ignored ignored-only` (or `cargo test -p pictura-select --test profile -- --ignored --nocapture`) prints every line

## 2. Shared engine

- [x] 2.1 Add `rayon = "1"` to `crates/pictura-core/Cargo.toml`, declare `pub mod mask_ops`, and verify `cargo check -p pictura-core` and `cargo deny check licenses` (no new license surface)
- [x] 2.2 Implement `dilate`/`erode` as separable monotonic-deque passes with a `Border::Zero(T)` / `Border::Clamp` policy (callers pass `0u8` / `0.0f32`) and a generic sample type; verify `cargo nextest run -p pictura-core mask_ops`
- [x] 2.3 Move the sliding-histogram median from `pictura-filters/src/noise.rs` into `mask_ops` (clamp border, `u16`-safe histogram, no kernel-side reach cap; each caller keeps its bound) and leave the caller's radius validation in place; verify `cargo nextest run -p pictura-core mask_ops`
- [x] 2.4 Implement the scanline four-connected flood taking an initial selected set `seeds: &[u8]` and a `matches(from, to)` predicate, returning `Option<Vec<u8>>`, with a `#[cfg(test)]` peak-frontier counter and the `flood_multi_seed_matches_the_naive_queue` differential; verify `cargo nextest run -p pictura-core mask_ops`
- [x] 2.5 Verify the kernels are scheduling-independent by running one and several rayon threads over the same random planes; verify `cargo nextest run -p pictura-core kernels_agree_under_one_and_many_threads`

## 3. Selection migration

- [x] 3.1 Route `Selection::{expand,contract,border}` through the zero-border morphology and delete `Selection::morph`; verify `cargo nextest run -p pictura-select` and `cargo test -p pictura-select --test oracle`
- [x] 3.2 Route `Selection::smooth` and `refine.rs`'s Smooth through the shared median and delete `select::median`; verify `cargo nextest run -p pictura-select` and `cargo nextest run -p pictura-select refine`
- [x] 3.3 Add `pictura-select/src/matcher.rs` (2^24-bit cube, separable OR dilation by tolerance, brute-force differential test) and route `similar` through it; verify `cargo nextest run -p pictura-select matcher` (or the package)
- [x] 3.4 Route `magic_wand` and `grow` through the shared flood, preserving the RGB predicate and the size-mismatch errors; add `grow_preserves_soft_selected_coverage`; verify `cargo nextest run -p pictura-select magic_wand` and `cargo nextest run -p pictura-select grow`

## 4. Filters migration

- [x] 4.1 Route `filters::maximum`/`minimum` through the clamp-border morphology and delete the private `morphology` scan and its `par_chunks` scaffolding; verify `cargo nextest run -p pictura-filters morphology` and `cargo nextest run -p pictura-filters --test kernel_digests`
- [x] 4.2 Route `filters::noise::median` through `mask_ops::median` and delete `window_median` (move `sliding_median_matches_the_clamped_sort` to the core test module, or delete it as superseded by `median_matches_the_sorted_window`); verify `cargo nextest run -p pictura-filters median` and `cargo nextest run -p pictura-filters --test kernel_digests`
- [x] 4.3 Verify the ImageMagick filter oracle still classifies Maximum/Minimum/Median as expected; verify `cargo test -p pictura-filters --test oracle` (skips cleanly without `magick`)

## 5. Paint migration

- [x] 5.1 Route `bucket::flood` through the shared scanline fill with an RGBA predicate and delete the DFS body; verify `cargo nextest run -p pictura-paint bucket`
- [x] 5.2 Route `bucket::stroke_selection`'s band through the clamp-border morphology and delete `dilate`/`erode`; verify `cargo nextest run -p pictura-paint band` (or the package)

## 6. Layer-effect migration

- [x] 6.1 Keep the pre-migration `axis_max`/`axis_min` scan as a `#[cfg(test)]` oracle (its output is the pre-change result) and add a differential test over random mattes at several radii
- [x] 6.2 Route `dilate_matte`/`erode_matte` through the f32 clamp-border morphology and delete `axis_max`/`axis_min`; verify `cargo nextest run -p pictura-render layer_effects` and `cargo nextest run -p pictura-render spread`
- [x] 6.3 Assert `spread_and_choke_mattes_match_the_pre_migration_scan` and the exact-pixel effect tests, including `parallel_composite_equals_sequential`; verify `cargo nextest run -p pictura-render layer_effects`

## 7. Performance remediation

- [x] 7.1 After task 3.2, rerun the profile and record the column-band Smooth r=100 baseline (the O(r) row slider was skipped: the spec requires radius-independent cost); measured at 6000×4000 on the 16-core dev box after the strip rewrite: Similar 10k colours ~52–57 ms, Smooth r=10 ~495 ms, Smooth r=100 ~478 ms, Expand r=100 ~55 ms
- [x] 7.2 Implement the column-band histogram (the O(r) row slider is skipped because the spec requires radius-independent cost) with column strips with a `±r` halo, `u32` bins, and a prefix-scan median; verify `cargo nextest run -p pictura-core mask_ops` and the profile prints Smooth under 1 s
- [x] 7.3 If column bands still miss, add the soft-distance binary fast path behind the same exact kernel (design.md Open Questions); verify the differential tests still pass and the profile reports the result
- [x] 7.4 If Similar misses the 100 ms target, remediate with a parallel cube build or a cheaper per-axis 1-D dilation and re-profile; verify the profile prints Similar under 100 ms

## 8. Integration

- [x] 8.1 Confirm the migrated call sites keep their documented border behaviours: selection Expand/Contract/Border use zero, Smooth and Refine Edge's Smooth use clamp, and Maximum/Minimum/Median/paint/render use clamp; verify `cargo nextest run -p pictura-select -p pictura-filters -p pictura-paint -p pictura-render`
- [x] 8.2 Run the full Rust layers; verify `cargo nextest run --workspace` and `cargo test --workspace --doc`
- [x] 8.3 Run the repository gates; verify `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `python3 scripts/check-test-plan.py`, and `openspec validate --all --strict`
