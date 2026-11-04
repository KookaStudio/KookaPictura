# Test plan

This change has no spec scenarios (`skip_specs: true`). The rows below map the
issue's acceptance criteria and the design risks to runnable checks. The five
`pictura-core` tests are written by tasks 1.1–1.2; `check-test-plan.py` passes
once they exist.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Empty, zero, and inverted rects | Build zero-width, zero-height, and inverted `PsdRect`s | `is_empty()` is true; a huge saturating span is not empty | `pictura-core::psd_rect_is_empty_covers_zero_inverted_and_huge` |
| Intersect miss | Intersect overlapping, edge-touching, disjoint, and inverted rects | Overlap returns `Some`; touching/disjoint/inverted return `None` | `pictura-core::psd_rect_intersect_returns_none_when_disjoint_or_touching` |
| Clip to the canvas | Clip rects with negative and beyond-canvas edges, and one fully off-canvas | Edges clamp to `0..width`/`0..height`; a miss returns `None` | `pictura-core::psd_rect_clip_to_clamps_and_reports_a_miss` |
| Union is a plain bounding box | Union overlapping and disjoint rects, including an empty rect | Result is the min/max bounding box; an empty rect is not treated as identity | `pictura-core::psd_rect_union_is_the_bounding_box_without_an_empty_identity` |
| Saturating growth | Grow a rect by a positive pad at the `i32` limits, then shrink it with a negative pad | Edges saturate at `i32::MIN`/`i32::MAX` instead of overflowing; a negative pad shrinks inward | `pictura-core::psd_rect_grow_saturates_at_the_i32_limits` |
| Damage, clipping, and compositing are unchanged | Run the region/damage suites after the helper removal | Byte-identical goldens; no region-path regression | `pictura-render::unbased_clipped_run_keeps_the_conservative_union`, `pictura-render::bounded_clipping_damage_matches_the_full_composite` |
| Render oracle | Run `cargo test -p pictura-render --test document_oracle` after the helper removal | Oracle suite passes unchanged | manual: cargo test -p pictura-render --test document_oracle |
| Render behavior is unchanged | Run `cargo nextest run -p pictura-render` after the helper removal | Behavior is unchanged; tests pass | `pictura-render::clipping_run_is_influenced` |
| App behavior is unchanged | Run `cargo nextest run -p pictura-app` after replacing the call sites | Behavior is unchanged; tests pass | `pictura-app::visibility_union_falls_back_for_unbounded_layer` |
| Paint and filters behavior is unchanged | Run `cargo nextest run -p pictura-paint -p pictura-filters` | Healing, stroke, and liquify results are unchanged | `pictura-paint::take_dirty_reports_each_dab_not_the_stroke_union`, `pictura-filters::an_identity_mesh_leaves_the_layer_bit_identical` |
| No leftover copies or size regression | Run the task 5.1 grep and `scripts/check-file-size.sh` | No deleted helper function remains and no new private copy is introduced; `lib.rs` under 1200 LOC | manual: task 5.1 grep plus the file-size gate |
