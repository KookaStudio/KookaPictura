# Tasks

## 1. Render region plumbing

- [x] 1.1 Add `clip: PsdRect` and `effect_region: Option<PsdRect>` to `Canvas`. `Canvas::new`/`new_region` set `clip` to their own bounds and `effect_region = None`; copy both in `with_cover_from` and inherit them in the isolated-group, clipping-coverage, and knockout-base canvases; verify `cargo test -p pictura-render --lib region` still passes.
- [x] 1.2 Set `effect_region = Some(region)` for a non-banded proper sub-rect in `composite_rgba_region`, `None` in `composite_whole`, and `None` plus `clip = the overall requested region` for every band in `composite_banded`; verify the banded and region suites (`cargo test -p pictura-render --lib region`, `--lib composite`) pass.
- [x] 1.3 Change `layer_source::effect_passes` and `layer_effects::build_layer_passes` to take `Option<PsdRect>`: `None` keeps the current whole-layer cached build, `Some(region)` builds transiently for that region; verify the existing `layer_source::tests` cache tests pass.

## 2. Effect-pass region builds

- [x] 2.1 Add `pictura-render::region_pass_equals_full_pass_for_every_family` (all ten effect families, region vs full pass slices; use a 200 px drop-shadow distance so the shift is exercised) and assert every pass rect lies inside the requested region `R`, so it fails before the builder changes.
- [x] 2.2 Add `pictura-render::dab_region_pass_is_bounded_and_equal` (paint a dab, build the pass for the dab region, assert every pass rect lies inside the present region and equals the full slice).
- [x] 2.3 Add `pictura-render::gradient_region_matches_the_full_buffer_slice` and `pictura-render::pattern_region_matches_the_full_tile_slice` for aligned and unaligned, linked and unlinked cases.
- [x] 2.4 Add `pictura-render::replacing_smart_object_contents_invalidates_the_composite` (replace, composite, purge, composite, compare).
- [x] 2.5 Region-clip `shadows`, `glows`, `strokes`, `satin`, and `bevel`: intersect `padded` with `grow(region, reach + |ox| + |oy|)` for drop shadow and inner shadow, and `grow(region, reach)` for the others (satin already includes its offset), clip each output loop to the region, and return empty when the intersection misses; verify 2.1 and 2.2 pass.
- [x] 2.6 Region-clip `overlays` and add `fill::gradient_rgba_region` (full extent for the gradient math, sub-rect fill); switch the pattern overlay to `pattern_tile_region` with an offset index; verify 2.3 passes.
- [x] 2.7 Pass one shared per-layer clip, `grow(region, max enabled build reach)` (including the shift for drop shadow and inner shadow), as the `mask_alpha` clip in every builder, so the raster and vector feather planes are built once per layer rather than once per effect family; verify the layer-effect suites pass.
- [x] 2.8 Verify the effect suites: `cargo test -p pictura-render --lib layer_effects` and `cargo test -p pictura-render --test document_oracle`.
- [x] 2.9 Extend `crates/pictura-render/tests/source_cache_profile.rs` with a canvas-sized styled layer, add `pictura-render::canvas_sized_styled_present_is_faster_than_a_full_composite` (assert the dab-sized region composite is at least ten times faster than a full composite and equals its slice), and verify `cargo test -p pictura-render --test source_cache_profile -- --ignored --nocapture` asserts the region/full ratio.
- [x] 2.10 Add `pictura-render::banded_sub_region_reuses_one_pass` (one whole-layer pass replayed into every band, output equals the whole composite) and `pictura-render::sub_region_pass_is_not_retained` (the shared cache gains no pass entry for a non-banded region).
- [x] 2.11 Add `pictura-render::editing_text_invalidates_the_composite` (composite, edit text, composite, purge, composite, compare).

## 3. Mask feather region

- [x] 3.1 Add `pictura-render::mask_feather_region_equals_the_full_slice` and `pictura-render::mask_dab_region_equals_the_full_slice`, assert the built plane extent (add a `#[cfg(test)]` plane-bounds/cache-key accessor in `mask_refine`), and confirm they fail before the clip change.
- [x] 3.2 Route content mask sampling (`blend_into_row` → `mask_alpha_unit`) through the canvas `clip` instead of the document canvas, updating the call sites at `composite_rows.rs:99` and the direct test calls in `tests/native_depth.rs:413-457`, and keep the high-depth fallback consistent; verify 3.1 passes.
- [x] 3.3 Verify the existing feather checks (`feather_ramps_a_hard_edge_monotonically`, `feather_reaches_past_the_rect_into_the_default_colour`) and the vector-mask suites still pass.

## 4. Paint reach in the app

- [x] 4.1 Add `pictura-app::paint_present_covers_the_dab_union_plus_reach` and `pictura-app::mask_present_covers_the_feather_reach`, and confirm they fail before the app change.
- [x] 4.2 Add `pictura-app::styled_layer_dab_commit_equals_full_recomposite`, `pictura-app::inner_effect_dab_commit_equals_full_recomposite`, `pictura-app::feathered_mask_dab_commit_equals_full_recomposite`, and `pictura-app::cancel_styled_stroke_restores_the_halo`.
- [x] 4.3 Add `pictura_render::paint_reach(layer)` (the maximum change spread over all enabled effects: drop shadow `|distance| + spread + blur`, outer glow `spread + blur`, stroke `size`, bevel `reach`, inner shadow `|ox| + |oy| + erode + blur`, inner glow `erode + blur`, satin `|ox| + |oy| + blur`) and `pictura_render::mask_feather_reach(mask)` (0 when the mask has no feather).
- [x] 4.4 Store `paint_pad` on `StrokeSession`, computed at pixel- and mask-stroke start from the target layer as `paint_reach(layer) + (mask stroke ? mask_feather_reach : 0)`.
- [x] 4.5 Grow by `paint_pad` at every refresh site: the pixel, GPU, and mask `queue_present`/`mark_stroke_tile`, the preview replay `stroke_tiles.mark` (`impl_paint.rs:851`), the mask preview replay `tiles.mark` (`mask.rs:337`), the mask commit `tiles.regions` (`mask.rs:352`; exact/GPU mask dabs never mark tiles), and the cancel's internal `composite_region_active` patch (`impl_paint.rs:142`). Thread `paint_pad` into `end_mask_paint`, `mask_dab`, and `mask_gpu_dab`. The reduced-resolution preview present is exempt; skip the `queue_present` at `impl_paint.rs:670`; verify 4.1 and 4.2 pass.
- [x] 4.6 Verify the app paint and damage suites: `cargo test -p pictura-app --lib impl_paint` and `cargo test -p pictura-app --lib damage_policy`.
- [x] 4.7 Add `pictura-app::dab_near_the_canvas_edge_clips_the_grown_rect`.

## 5. Integration and gates

- [x] 5.1 Run `cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings`; fix findings.
- [ ] 5.2 Run `cargo nextest run --workspace` and `cargo test --workspace --doc`; fix failures.
- [x] 5.3 Run `python3 scripts/check-test-plan.py` and `openspec validate --all --strict`; both pass.
- [ ] 5.4 Run `bash scripts/verify-fast.sh` (or the full gate on the reference machine) and confirm the render and paint suites plus the file-size and guard checks pass.
- [ ] 5.5 Archive the change with `openspec archive effect-pass-region-builds` so only `openspec/changes/archive/` remains, and confirm `scripts/check-openspec-archived.sh` passes.
