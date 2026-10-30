# Test Plan

Every scenario in the delta specs is mapped to an automated test. GPU scenes are
`#[ignore]`d and run in the `gpu-parity` CI job on lavapipe
(`cargo nextest run -p pictura-render --features software-vulkan --profile gpu
--run-ignored all`); they fail, not skip, when no device is present. The CPU
oracle stays the reference and is never re-baselined.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| Separable mode parity | Composite the separable scenes on a usable adapter | Every channel within ±1 LSB of `composite_rgba` | `pictura-render::gpu_matches_cpu_within_one_lsb` |
| Non-separable mode parity | Composite the Hue/Saturation/Color/Luminosity scenes | Every channel within ±1 LSB | `pictura-render::non_separable_modes_match_cpu` |
| Adjustment layer parity | Composite the nine adjustment kinds | Applied at the same stack position, within ±1 LSB | `pictura-render::adjustment_layers_match_cpu`, `pictura-render::adjustment_luts_match_cpu` |
| Gradients, masks, and opacity | Composite masked, partial-alpha, reduced-opacity, Fill, and non-separable scenes | Within ±1 LSB including masked-out regions | `pictura-render::groups_and_masks_match_cpu`, `pictura-render::fill_scene_matches_cpu`, `pictura-render::non_separable_modes_match_cpu` |
| Group semantics | Composite pass-through and isolated groups | Within ±1 LSB for the group and children | `pictura-render::groups_and_masks_match_cpu`, `pictura-render::proxied_type_in_groups_matches_cpu` |
| Raster source parity | Composite a proxy-less type layer, a smart-filtered object with a filter mask, and a channel-less smart object | Within ±1 LSB including the mask mix | `pictura-render::raster_sources_match_cpu` |
| Fill layer parity | Composite Solid, Gradient, and Pattern fill layers | Within ±1 LSB | `pictura-render::fill_layers_match_cpu` |
| Effect and clipping parity | Composite a styled layer and a clipping group | Within ±1 LSB | `pictura-render::effects_match_cpu`, `pictura-render::clipping_matches_cpu` |
| Dissolve mode | Composite a Dissolve layer | `Err(UnsupportedMode(Dissolve))`, no panic | `pictura-render::cpu_only_modes_error_without_panicking`, `pictura-render::dissolve_still_cpu_only` |
| Supported adjustment composites | Composite each of the nine kinds | `Ok` and applied in place | `pictura-render::adjustment_layers_match_cpu`, `pictura-render::adjustment_luts_match_cpu` |
| Unsupported adjustment kind | Composite an unsupported adjustment | `Err(UnsupportedAdjustment)` | `pictura-render::unsupported_adjustment_falls_back_or_errors` |
| Advanced blending still declines | Composite a Blend If and a knockout layer | `Err(UnsupportedAdvancedBlending)` | `pictura-render::advanced_blending_still_declines` |
| A region matching the full canvas equals the full composite | Region over the whole document | Byte-identical to `composite_active`, same backend | `pictura-render::full_region_equals_full_composite` |
| A sub-rect equals the corresponding slice of the full composite | Region over each new route | Every pixel equals the full composite's slice | `pictura-render::new_routes_region_matches_full_slice` |
| Out-of-bounds and empty regions are clamped without panicking | Out-of-bounds and empty rects | Clamped/zero-size buffer, no panic | `pictura-render::empty_and_out_of_bounds_region_are_safe` |
| Plan and dispatch agree for every content route | Build one document per route and run the plan | Accepted routes dispatch the named source; declined routes error before dispatch | `pictura-render::plan_and_dispatch_agree` |
| A hidden layer is not dispatched | Plan a stack with a hidden effect/unsupported layer | No decline, no dispatch | `pictura-render::hidden_layer_is_not_planned` |
| A hidden clipping base suppresses its run | Plan/ composite a visible clipped layer above a hidden base | The run contributes nothing, matching the CPU | `pictura-render::hidden_clipping_base_suppresses_run`, `pictura-render::clipping_hidden_base_matches_cpu` |
| The plan is device-independent | Compute the plan with and without an adapter | Same accept/decline result | `pictura-render::plan_is_device_independent` |
| Every declining content route has a reason | Plan one document per declined route | Each returns its named `GpuError`, no device needed | `pictura-render::every_declining_route_names_its_reason` |
| A proxied type layer stays eligible with the gamma on | Plan a proxied type layer with the gamma on | Accepted | `pictura-render::proxied_type_layer_with_gamma_is_eligible` |
| A channel-less type layer is accepted | Plan a renderable proxy-less type layer and composite it | Plan accepts; the GPU scene composites the cached raster | `pictura-render::channel_less_type_layer_is_accepted`, `pictura-render::raster_sources_match_cpu` |
| An unrenderable channel-less type layer declines | Plan an empty proxy-less type layer | `Err(UnsupportedText)`, `type_source` counted | `pictura-render::unrenderable_type_source_declines` |
| An unrenderable proxy-less smart object declines | Plan a channel-less smart object whose source cannot render | `Err(UnsupportedSmartObject)` | `pictura-render::unrenderable_smart_source_declines` |
| A decoded vector fill is accepted | Plan a decodable `vscg` fill with no adjustment block | Accepted and routed as a fill | `pictura-render::decoded_vector_fill_is_accepted`, `pictura-render::fill_layers_match_cpu` |
| Effects and clipping no longer decline | Plan a drop-shadow layer and a clipping stack | Accepted | `pictura-render::effects_and_clipping_are_accepted` |
| The CPU oracle is unchanged | Composite one document per route with the CPU | Bytes equal the pre-change recording | `pictura-render::cpu_oracle_unchanged_for_every_route` |
| A drop-shadow layer composites on the GPU | Composite a drop-shadow scene | `Ok`, shadow at the CPU position, within ±1 LSB | `pictura-render::effects_match_cpu` |
| Every effect family matches the CPU | Composite one scene per family | Within ±1 LSB | `pictura-render::effects_match_cpu` |
| A multi-effect layer keeps its draw order | Composite drop shadow + stroke + inner shadow | Below/content/above order, within ±1 LSB | `pictura-render::effect_draw_order_matches_cpu` |
| A disabled or undecodable effect stays eligible | Plan/ composite a disabled or malformed effect | Accepted, no pass, CPU parity | `pictura-render::disabled_effects_stay_eligible` |
| An enabled effect the CPU renders as a no-op stays eligible | Plan/ composite a Stroke-style bevel, a no-renderer legacy effect, or an undecodable stroke/overlay payload | Accepted, no pass, CPU parity | `pictura-render::disabled_effects_stay_eligible` |
| Effect region parity | Region and full composite of a styled document | Region equals the full slice | `pictura-render::new_routes_region_matches_full_slice` |
| A clipping group matches the CPU | Composite a clipped green layer over a red base | Within ±1 LSB, clipped to the base | `pictura-render::clipping_matches_cpu` |
| A hidden base hides the run | Composite with the base hidden | Clipped layers contribute nothing | `pictura-render::clipping_hidden_base_matches_cpu` |
| A clipped layer with no base is unclipped | Composite a bottom clipped layer | Composited unclipped | `pictura-render::clipping_no_base_matches_cpu` |
| Multiple clipped layers share one base | Composite two clipped layers over one base | Both share the coverage, within ±1 LSB | `pictura-render::clipping_multiple_layers_match_cpu` |
| The base's Fill and effects do not narrow the clip | Composite a base at Fill 0 and with an effect | Coverage is the base's content alpha, within ±1 LSB | `pictura-render::clipping_base_fill_and_effects_match_cpu` |
| Clipping region parity | Region and full composite of a clipping group | Region equals the full slice | `pictura-render::new_routes_region_matches_full_slice` |
| A solid fill matches the CPU | Composite a Solid Color fill layer | Within ±1 LSB | `pictura-render::fill_layers_match_cpu` |
| A gradient fill matches the CPU | Composite every gradient kind and angle | Within ±1 LSB | `pictura-render::fill_layers_match_cpu` |
| A pattern fill matches the CPU | Composite a Pattern fill layer | Within ±1 LSB | `pictura-render::fill_layers_match_cpu` |
| A pattern fill is not served stale | Change the pattern content and composite again | New pattern, equals a purge-then-composite | `pictura-render::pattern_fill_is_not_served_stale` |
| A decoded vector fill matches the CPU | Composite a decodable `vscg` Solid, Gradient, or Pattern fill | Within ±1 LSB | `pictura-render::fill_layers_match_cpu` |
| Levels matches the CPU | Composite a Levels layer with composite and channel records | Within ±1 LSB | `pictura-render::adjustment_luts_match_cpu` |
| Curves matches the CPU | Composite a Curves layer with composite and channel curves | Within ±1 LSB | `pictura-render::adjustment_luts_match_cpu` |
| Exposure matches the CPU | Composite an Exposure layer | Within ±1 LSB | `pictura-render::adjustment_luts_match_cpu` |
| Gradient Map matches the CPU | Composite a Gradient Map with transparent stops | Within ±1 LSB including the original-colour blend | `pictura-render::adjustment_luts_match_cpu`, `pictura-adjust::gradient_map_lut_carries_alpha` |
| A masked LUT adjustment is gated like the CPU | Composite a masked, reduced-opacity LUT layer | Within ±1 LSB | `pictura-render::adjustment_luts_match_cpu` |
| gt_resident | Composite twice, change blend, write with/without a stamp | Hits on the unchanged passes, all within ±1 LSB | `pictura-render::resident_uploads_are_reused_and_never_stale` |
| A region composite reuses whole-document sources | Full composite then repeated region composites | Every source reports a resident hit | `pictura-render::resident_uploads_stay_bounded_across_region_presents`, `pictura-render::cached_raster_source_is_reused` |
| Moving region composites leave the store flat | 100 moving region composites | Entry count and bytes do not grow; next full composite has zero misses | `pictura-render::resident_uploads_stay_bounded_across_region_presents` |
| Constant coverage is rebuilt, never retained | Composite a maskless layer | Coverage is not offered to the resident store | `pictura-render::a_mask_key_needs_a_stamped_plane_only_when_it_samples_one` |
| A sampled mask is reused by a repeated whole-document composite | Composite a masked layer twice | Second composite hits the mask upload | `pictura-render::resident_uploads_stay_bounded_across_region_presents` |
| A cached raster source is reused | Composite a type/smart/Solid or Gradient fill document twice | Second composite hits each raster upload | `pictura-render::cached_raster_source_is_reused` |
| A cached raster source changes when its content changes | Edit the text/filter/payload/fill and composite again | New upload, equals a purge-then-composite | `pictura-render::cached_raster_source_invalidates` |
| A cached raster upload is not retained for a sub-region build | Build a raster for a sub-region composite | Used, not retained | `pictura-render::raster_key_absent_for_region_build` |
| A pattern fill upload is not retained | Composite a Pattern fill layer | The upload is used and not offered to the resident store | `pictura-render::pattern_fill_upload_is_not_retained` |
| A smart-filter mask mix is cached exactly | Composite a smart-filtered object with a filter mask twice | Second composite reuses the mixed buffer, equals a purge-then-composite | `pictura-render::composited_smart_filter_mask_mix_is_cached`, `pictura-render::raster_sources_match_cpu` |
| A placed smart source is cached exactly | Composite a channel-less smart object twice | Second composite reuses the placed buffer | `pictura-render::composited_smart_source_is_cached` |
| A proxy-less type raster is cached exactly | Composite a proxy-less type layer twice | Second composite reuses the raster | `pictura-render::composited_type_raster_is_cached` |
| A filter-mask edit invalidates the buffer | Change the smart-filter mask and composite again | Rebuilt, equals a purge-then-composite | `pictura-render::composited_source_invalidates_on_filter_mask` |
| A payload or text edit invalidates the buffer | Change the payload, text model, or rect and composite again | Rebuilt, equals a purge-then-composite | `pictura-render::composited_source_invalidates_on_payload_or_text` |

The LUT builder is also proven without a device:
`pictura-adjust::adjustment_lut_matches_apply` compares each accepted table
against applying the adjustment to the same 0..255 ramp.
