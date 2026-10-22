# Tasks

## 1. #846 Pass Through group vector mask

- [x] Add the failing regression test: `pictura-render::pass_through_group_vector_mask`, comparing the masked Pass Through result with the isolated path on CPU and GPU.
- [x] Add `vector_mask.is_none()` to the pass-through guard in `composite.rs` and `gpu/backend.rs`.
- [x] Run only that test, then the owning suite.

## 2. #847 opacity and fill semantics

- [x] Add the failing regression tests: `pictura-render::layer_opacity_scales_effects`, `pictura-render::adjustment_ignores_fill`.
- [x] Add the guard tests `pictura-render::fill_layer_keeps_fill` and `pictura-render::fill_combines_with_opacity_and_mask` for the fill path that must not regress.
- [x] Add `Canvas.effect_scale` (default 1.0, copied by `with_cover_from`), multiply it in `blend_parts`, and set it to `layer.opacity/255` around both effect passes; keep content scaling in `blend_into_row`.
- [x] Ignore Fill for non-fill adjustments in `composite_adjustment` and in the GPU adjustment dispatch; keep Fill on `SolidFill`/`GradientFill`/`PatternFill`.
- [x] Run only those tests, then the owning suite.

## 3. #848 effect geometry

- [x] Add the failing regression tests: `pictura-render::effects_reach_includes_spread`, `pictura-render::placed_smart_object_effect_matte`
- [x] Share one dilation/blur padding helper between `effects_reach` and the shadow/glow renderers.
- [x] Use `render_smart_source` alpha in `content_matte`, keeping the rect fallback for an unreadable source.
- [x] Run only those tests, then the owning suite; keep the existing channel-less smart-object test green.

## 4. #849 Merge Down in a clip run

- [x] Add the failing regression tests: `pictura-render::merge_clipped_layer_preserves_clip`, `pictura-render::merge_clipping_mask_applies_coverage_once`
- [x] In Merge Down, clear `clipping` on both scratch copies when the target is clipped, so `composite_siblings` composites them as ordinary layers; keep the target's flag on the result.
- [x] Leave the clipped source's flag in the scratch when the target is the base; the result is unclipped.
- [x] Pass the result's `clipping` flag through `pixel_result` instead of hard-coding `false`.
- [x] Delete `fold_clipping` and its stale comment; the compositor's clipping pass is the single coverage source.
- [x] Run only those tests, then the merge and clipping suites.

## 5. #861 Convert to Profile at native depth

- [x] Add the failing regression tests: `pictura-color::identity_rgb32`, `pictura-color::unsupported_format_is_rejected`, `pictura-codec::profile_conversion_preserves_native_samples`, `pictura-codec::profile_conversion_does_not_clamp_32bit`
- [x] Add 32-bit float formats to `pictura_color::formats`.
- [x] Convert `source_planes` color planes and layer `source_channels` `0..3` planes, recursively, at a 16/32-bit store's depth (document size for the composite store, layer rect for a layer store); leave an 8-bit Lab store, partial native or working stores, alpha/mask, and extras untouched.
- [x] Interleave the planar planes and pack in lcms2's native-endian `u16`/`f32` order (not `Samples::to_bytes`, which is big-endian).
- [x] Rewrite each converted channel's 8-bit working plane from `narrow_to_u8()` of the converted native samples so `write_psd` re-emits the native store.
- [x] Run only those tests, then the color and codec suites.

## Final

- [x] `cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings`
- [x] `bash scripts/verify-fast.sh`
- [x] `openspec validate --all --strict`
- [x] `openspec archive compositor-native-bugs`
- [ ] PR body lists manual rows as a reviewer checklist.
