# Tasks

## 1. Content identity in core and codec

- [x] 1.1 Add `pictura_core::content_digest`, `SmartObject::payload_digest`, a `payload_digest()` accessor that returns the stored value when non-zero and hashes `payload` when it is zero, a `set_payload` helper, and a manual `PartialEq` that excludes the digest; verify `cargo nextest run -p pictura-core` passes and a round-trip equality test ignores the digest
- [x] 1.2 Write `pictura_codec::smart_object::tests::payload_digest_tracks_replace_and_crs_patch`: read a document, assert a non-zero digest, patch the CRS XMP and replace the payload, and assert the digest changes each time
- [x] 1.3 Set the digest at the five payload write sites (codec read, `convert_to_smart_object`, place, replace, CRS patch); verify the new test and the existing smart-object codec tests pass
- [x] 1.4 Verify `assert_eq!` between a literal `SmartObject` and a parsed one still holds (digest excluded) with `cargo nextest run -p pictura-render smart_object`, and that a literal-built object whose stored digest is zero still returns a payload hash from `payload_digest()`

## 2. Source cache infrastructure

- [x] 2.1 Write `pictura_render::layer_source::tests::second_request_hits`, `eviction_respects_the_byte_budget`, `purge_forces_a_rebuild`, and `parallel_build_of_one_key_runs_once`; run them with `cargo nextest run -p pictura-render layer_source`
- [x] 2.2 Add `crates/pictura-render/src/layer_source/mod.rs` with one process-wide `Mutex` map of content keys to `Arc<OnceLock<Artifact>>`: look up under the lock, clone the `Arc`, release the lock, then build in `get_or_init` outside it. Add the default 256 MiB byte budget (LRU eviction; an artifact larger than the budget is built but not stored), `purge_source_cache`, and stats (`source_bytes` is shared resident bytes). No per-thread cache. Verify the tests from 2.1 pass
- [x] 2.3 Add `layer_source/key.rs` with the content key (plane idents, mask/vector-mask properties, fill blocks, payload digest, filter hash, layer rect, document size/mode) and a visible test that each mutation changes the key; verify with `cargo nextest run -p pictura-render layer_source`
- [x] 2.4 Add `source_hits`, `source_misses`, `source_bytes`, and `effect_decodes` to `metrics::Counters` and print them in the app metrics JSON; verify `cargo nextest run -p pictura-render metrics` and the app metrics test; add `pictura_render::layer_source::tests::counters_are_visible`
- [x] 2.5 Register the module in `lib.rs` and expose `purge_source_cache`; verify `cargo clippy -p pictura-render --all-targets -- -D warnings`
- [x] 2.6 Write `pictura_render::layer_source::tests::every_source_kind_hits_on_the_second_composite`: composite an unchanged document twice and assert cache hits for a proxy-less smart object, a smart-filtered layer, a proxy-less type layer, and a styled layer, and that their decode/filter/shape/effect builds are not repeated

## 3. Embedded smart-object source cache

- [x] 3.1 Write `pictura_render::layer_source::tests::embedded_source_reuses_a_hit` and `hidden_layer_is_not_decoded` (covering a hidden smart object **and** a hidden type layer); verify the first hits on the second composite and the second leaves the counter unchanged
- [x] 3.2 Move payload decoding and the merged-composite fallback into `layer_source::embedded_source`, returning `Arc<PixelBuffer>`; make `render_smart_source` sample from it with the existing nearest-neighbour loop, keeping the existing no-depth-guard `ponytail:` note (a cyclic payload can re-enter the same `OnceLock`)
- [x] 3.3 Verify `cargo nextest run -p pictura-render smart_object` and the `pictura_render::tests::smart_object` suite stay green

## 4. Smart-filter chain cache

- [x] 4.1 Write `pictura_render::tests::region::smart_filter_region_matches_the_full_slice` using a Lens Correction filter; confirm it fails before the change (the current region path applies the warp to a region-sized buffer)
- [x] 4.2 Cache the **filtered** `apply_smart_filter_chain` output at `layer.rect ∩ document bounds`, keyed by payload digest + full layer rect + document width/height + chain hash (`smart_filters_enabled` plus each filter's `enabled`, `filter_id`, `options`); sample the unfiltered base from the cached decoded source at the same clipped rect for the masked-out pixels, and apply the filter mask per requested region
- [x] 4.3 Verify 4.1 passes and `cargo nextest run -p pictura-render smart_filter` stays green (masked-out pixels keep the source)

## 5. Type raster cache

- [x] 5.1 Write `pictura_render::layer_source::tests::text_raster_hits_and_invalidates` covering text, style, box, vertical, font registration, and missing-glyph flag changes; verify each mutation misses and the unchanged repeat hits
- [x] 5.2 Add an atomic font-registry generation bumped by `register_font` and include it plus `missing_glyph_protection()` in the text key; verify the test from 5.1
- [x] 5.3 Cache `render_text_buffer` output by TypeTool fingerprint, rect, and font generation; make `composite_type_source` sample it
- [x] 5.4 Verify `cargo nextest run -p pictura-render text` and the type-layer oracle tests pass

## 6. Effects decode once and build passes

- [x] 6.1 Write `pictura_render::tests::layer_effects::cache::effect_passes_are_byte_identical_cached` for every effect family (drop shadow, outer glow, inner shadow, inner glow, colour/gradient/pattern overlay, satin, stroke, bevel) using `purge_source_cache` between builds; put the matrix in `crates/pictura-render/src/tests/layer_effects/cache.rs` and add `mod cache;` to the parent, not in the 1152-line `layer_effects.rs`
- [x] 6.2 Cache `decode_layer_effects` by authoritative block tag and bytes; resolve it once in `composite_layer_inner` and pass the `Arc` to the below, above, and reach paths
- [x] 6.3 Add `layer_effects/planes.rs` with `EffectPass { rect, mode, color: EffectColor, alpha: Vec<f32> }`, where `EffectColor` is `Constant([f32; 3])` or `PerPixel(PixelBuffer)`, and a generic replay through `blend_parts`; add an offset-canvas test proving replay is canvas-origin agnostic
- [x] 6.4 Refactor each effect module into `build_*` (document-space geometry, matte, mask, dilate, blur, noise) and `apply` (clip to canvas, call `blend_parts`), keeping `composite_*` as build-then-apply for the existing test hooks. Replace `canvas.w/h` used as the document size in `overlays.rs` and `strokes.rs` (unaligned gradient/pattern) with document bounds, in addition to `clip_rect`/`pad_rect`. A bevel emits up to four passes (two `lit` slots × shadow/highlight; `Pillow` uses both slots)
- [x] 6.5 Make `effects_reach` return a total `PsdRect` (fill and inverted-mask content bounded by the layer rect) and include the bevel's real padding (`size + blur_support(rounding(technique, size)) + blur_support(soften)`); update `rasterize`, add `pictura_render::tests::layer_effects::rasterized_styled_object_bakes_the_effect_reach` (smart/type/fill/inverted-mask bakes now include the effect reach), and verify `effects_reach_includes_spread`
- [x] 6.6 Cache `Arc<Vec<EffectPass>>` by content key plus effect digest; do **not** cache any pass whose build reads `decode_patterns` (pattern overlay, pattern-filled stroke, or a matte over pattern-fill content) — rebuild those per composite, with a `ponytail:` note naming the pattern-digest upgrade path; verify 6.1, 6.7, and 6.8
- [x] 6.7 Write `pictura_render::layer_source::tests::effects_decode_once` using the `effect_decodes` counter; a multi-effect layer must decode exactly once per composite after a purge
- [x] 6.8 Write `pictura_render::layer_source::tests::effect_passes_invalidate_on_style_or_rect`: change an effect parameter, the master switch, the layer mask, and the layer rect; each must rebuild the passes and match a purge-then-composite
- [x] 6.9 Write `pictura_render::layer_source::tests::cold_and_warm_composites_agree`: purge, composite a document with a smart object, smart filter, type layer, mask, and effects, then composite again warm and compare bytes

## 7. Region composites draw effects

- [x] 7.1 Move `composite_rgba_region`, `banded`, and `composite_banded` (not `slice_region`) into `composite_region.rs` as a pure move; register the module in `lib.rs`; verify `cargo nextest run -p pictura-render region` is unchanged
- [x] 7.2 Write `pictura_render::tests::region::styled_layer_region_matches_the_full_slice` covering a drop shadow, outer/inner glow, inner shadow, stroke, satin, bevel, and overlays, and `offset_canvas_effect_replay_matches`
- [x] 7.3 Remove the bounded-effect full-composite fallback and `effects_may_reach`; apply cached passes clipped to the requested region; remove only the region path's use of `Canvas::skip_effects` and keep an internal no-effects flag for `composite_clipping::coverage` and `knockout_base` (rename the field if clearer); delete `slice_region`; update the stale comments in `composite.rs:38-51,137-139` and `gpu/mod.rs:239-245`
- [x] 7.4 Verify 7.2, the existing `region` suite, and `tests/layer_effects` pass; confirm a region near a styled layer no longer calls `composite_rgba`
- [x] 7.5 Write `pictura_render::tests::region::bounded_effect_does_not_full_composite`: a 4000² document's 512×120 region near a styled layer must leave the largest canvas region-sized

## 8. Banded compositing

- [x] 8.1 Write `pictura_render::tests::region::banded_equals_whole_with_object_layers` for a tall document with a smart-filtered layer, a proxy-less type layer, and a styled layer
- [x] 8.2 Widen `banded` to `rows > 2 * BAND` and delete the per-layer exclusions, except a stack whose passes read the document pattern library (those passes are not cached); verify 8.1 and the existing banded tests
- [x] 8.3 If 8.1 cannot be made byte-identical cheaply, revert 8.2 and record the reason in the change (and drop the `banded-compositing` delta); the other contracts do not depend on it

## 9. Layer Style preview bounds

- [x] 9.1 Add `pictura_render::layer_paint_bounds(doc, path) -> Option<PsdRect>`: pixel/smart/type/fill = `rect ∪ effects_reach`; group = union of descendants' paint bounds; adjustment layer = the document canvas or its bounded mask bounds; missing path = `None`. Add a Rust unit test for a pixel layer, a fill layer, a group, an adjustment layer, and a missing path
- [x] 9.2 Write `tst_layer_style::previewIsBounded`: change a drop-shadow size and a bevel size through `layer_style_set`; assert the emitted `regionDamaged` rect is larger than the layer and smaller than the document, and cover a group, a masked adjustment layer, and a missing path in the Rust `layer_paint_bounds` tests. Write `tst_layer_style::refusedValueEmitsNothing`: a refused or unchanged value emits nothing
- [x] 9.3 In `layer_style_set`, capture the paint bounds before and after, union them, and call `refresh_region` instead of `recomposite`; keep the content-revision bump. Because the preview no longer emits `changed`, connect `PictureView::regionDamaged` to the recovery-stale set in `frame_recovery.cpp:trackRecovery` (or emit a notify on the `Edit::Record` commit) and verify with `tst_recovery`
- [ ] 9.4 (optional, not taken) Optionally switch `set_all_effects_visible` to a region edit over the union of affected paint bounds; verify the existing layers-effects tests
- [x] 9.5 Add `pictura-app::dimension_change_recomposites_fully`, `pictura-app::unbounded_adjustment_recomposites_fully`, and `pictura-app::unbounded_effect_recomposites_fully` in a new `crates/pictura-app/src/cxxqt_object/damage_policy_tests.rs` (register `mod damage_policy_tests;` in `cxxqt_object.rs`) for the `described-damage` delta scenarios; do not add them to the 1390-line `tests.rs`

## 10. Benchmark and full gate

- [x] 10.1 Add `crates/pictura-render/tests/source_cache_profile.rs` with an `#[ignore]` benchmark named `warm_region_is_ten_times_faster`: 4000² document, a Camera Raw (or Lens Correction) smart-filtered layer, and a text layer with a drop shadow; measure cold vs warm 512×120 region composites (height below `2 * BAND`) and repeated full composites
- [x] 10.2 Run `cargo test -p pictura-render --test source_cache_profile -- --ignored --nocapture` and confirm the warm region is at least 10× the cold baseline and full composites report cache hits
- [x] 10.3 Run `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run --workspace`, and `cargo test --workspace --doc`
- [x] 10.4 Run `bash scripts/verify-fast.sh`, `openspec validate --all --strict`, and `openspec archive layer-source-cache`; verify only `openspec/changes/archive/` remains for this change
