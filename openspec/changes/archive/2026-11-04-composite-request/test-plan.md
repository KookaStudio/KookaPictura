# Test Plan

Every scenario in this change's spec deltas maps to an automated test or a
justified manual check. The new request matrix test is
`pictura-render::request_matrix_matches_the_oracles`; it composites a fixture
with a group, a mask, an adjustment, and a separable blend through all four
`(region, gpu_enabled)` combinations, compares each result to the existing
oracle, and covers the 32-bit region fallback.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| No Vulkan adapter | Call `composite(CompositeRequest::whole(doc, true))` with no usable device, and pin the backend to CPU. | `Backend::Cpu`; bytes equal `composite_rgba(doc)`; no panic. | `pictura-render::cpu_pin_composites_on_cpu` |
| Document exceeds device limits | Composite a document past the 2-D product limit on a device. | The GPU attempt reports `GpuError::TooLarge`; the test does not panic; an unallocatable document is outside the CPU-fallback contract. | `pictura-render::document_past_2d_product_limit_returns_too_large` |
| Fallback for a CPU-only stack | Composite a Dissolve stack with `gpu_enabled` true. | `Backend::Cpu`; bytes equal `composite_rgba(doc)`. | `pictura-render::fallback_matches_cpu_for_cpu_only_mode` |
| Parity after the data-path change | Composite every GPU mode on a device. | Each channel within ±1 LSB of `composite_rgba(doc)`. | `pictura-render::gpu_matches_cpu_within_one_lsb` |
| Output shape is unchanged | Composite any document through the GPU path. | Same width, height, and channel count as `composite_rgba(doc)`. | `pictura-render::gpu_matches_cpu_within_one_lsb` |
| Group layer dispatch | Composite a stack with pass-through and isolated groups. | GPU bytes within ±1 LSB of the CPU oracle. | `pictura-render::groups_and_masks_match_cpu` |
| A region matching the full canvas equals the full composite | Call the request with the whole-document rect on CPU and GPU. | Bytes equal the whole-document request; same `Backend`. | `pictura-render::full_region_equals_full_composite`, `pictura-render::cpu_region_matches_full_slice` |
| A sub-rect equals the corresponding slice of the full composite | Composite a sub-rect on CPU and GPU. | Every pixel equals the whole-document slice at the offset. | `pictura-render::region_matches_full_slice`, `pictura-render::cpu_region_matches_full_slice` |
| Out-of-bounds and empty regions are clamped without panicking | Pass a region outside the document and a zero-area region. | Clamped buffer or 0×0 buffer; no panic. | `pictura-render::empty_and_out_of_bounds_region_are_safe` |
| A device that matches the oracle stays enabled | Composite the validation scene on a matching device. | `gpu_available()` true; `Backend::Gpu`. | `pictura-render::the_gpu_gate_is_consistent_with_the_validation_scene`, `pictura-render::enabled_gpu_matches_availability` |
| A device that diverges is treated as absent | Force a failing validation scene. | `gpu_available()` false; request reports `Backend::Cpu` with CPU bytes. | manual: a device whose validation scene diverges cannot be forced on lavapipe/WARP; the test only branches on the real device |
| The GPU paint stroke falls back on a diverging device | Check `GpuStroke::available()` against `gpu_available()`. | Both false on a diverging device; `GpuStroke::new` returns `Unavailable`. | manual: same validation-divergence limitation; forcing a failing device is not reproducible on lavapipe/WARP |
| The CPU-only mode check still precedes the gate | Request a Dissolve stack on an unusable device. | The GPU attempt reports `UnsupportedMode(Dissolve)`, not `Unavailable`. | `pictura-render::dissolve_still_cpu_only` |
| The check runs at most once | Call `gpu_available()` repeatedly. | The validation scene runs once; the cached value is returned. | `pictura-render::validation_runs_at_most_once` |
| A 16-bit document declines | Run the GPU eligibility check on a 16-bit document. | `Err(GpuError::UnsupportedDepth)`. | `pictura-render::sixteen_bit_document_declines` |
| The active backend falls back for a 32-bit document | Request a 32-bit document with `gpu_enabled` true. | `Backend::Cpu`; bytes equal `composite_rgba(doc)`. | `pictura-render::thirty_two_bit_active_backend_is_cpu`, `pictura-render::request_matrix_matches_the_oracles` |
| A 32-bit region request falls back to CPU | Request a sub-rect of a 32-bit document with `gpu_enabled` true. | `Backend::Cpu`; bytes equal the matching `composite_rgba(doc)` slice. | `pictura-render::request_matrix_matches_the_oracles` |
| An 8-bit document stays eligible | Run the eligibility check on a supported 8-bit document. | The document is accepted. | `pictura-render::eight_bit_document_stays_eligible` |
| Adapter present and GPU enabled | Request a document with `gpu_enabled` true on a usable adapter. | `Backend::Gpu`; within ±1 LSB of `composite_rgba(doc)`. | `pictura-render::enabled_gpu_matches_availability` |
| GPU disabled or unavailable | Request with `gpu_enabled` false, and with no adapter. | `Backend::Cpu`; bytes equal `composite_rgba(doc)`. | `pictura-render::disabled_gpu_returns_cpu_backend`, `pictura-render::cpu_pin_composites_on_cpu` |
| A depth-16 document yields u16 samples | Run the internal source-depth composite on a 16-bit document. | `Some(Samples::U16(_))` of length `width * height * 4`. | `pictura-render::depth16_native_composite_keeps_sub_8bit_adjustment_samples` |
| A depth-32 document yields f32 samples | Run the internal source-depth composite on a 32-bit document. | `Some(Samples::F32(_))`. | `pictura-render::composite_native_is_none_without_depth_and_f32_at_32` |
| An 8-bit document yields nothing | Run the internal source-depth composite on an 8-bit document. | `None`. | `pictura-render::composite_native_is_none_without_depth_and_f32_at_32` |
| The application-facing refresh stays exposed | Call `refresh_native_composite` on a layered 16-bit document. | Native color planes and the 8-bit composite are updated; returns true. | `pictura-render::refresh_splices_native_color_planes_into_a_layered_depth_document` |
| A GPU-eligible scene matches the oracle | Composite a Multiply fixture through the request on the software device. | `Backend::Gpu`; every channel within 1 LSB of `composite_rgba`. | `pictura-render::fixture_scenes_match_oracle_on_active_backend` |
| A declined scene equals the oracle | Composite a Dissolve fixture through the request. | `Backend::Cpu`; bytes equal `composite_rgba`. | `pictura-render::declined_fixture_scene_equals_oracle` |
| A decline is attributed | Request a Dissolve stack with metrics on. | The `unsupported_mode` decline count increases. | `pictura-render::dissolve_decline_is_counted` |
| No timestamps, no fake zero | Build a composite record with no device time. | The report line has no `gpu_ms=` field. | `pictura-app::record_without_device_time_omits_field` |

| Separable mode parity | Composite Normal and every separable blend mode on a device. | Every channel within ±1 LSB of `composite_rgba`; same shape. | `pictura-render::gpu_matches_cpu_within_one_lsb` |
| Non-separable mode parity | Composite Hue, Saturation, Color, and Luminosity on a device. | Every channel within ±1 LSB of `composite_rgba`. | `pictura-render::non_separable_modes_match_cpu` |
| Adjustment layer parity | Composite invert, posterize, threshold, brightness/contrast, hue/saturation, levels, curves, exposure, and gradient map. | Within ±1 LSB at the layer's stack position. | `pictura-render::adjustment_layers_match_cpu`, `pictura-render::adjustment_luts_match_cpu` |
| Gradients, masks, and opacity | Composite masked, partial-alpha, and gradient-fill layers. | Within ±1 LSB, including masked-out and transparent regions. | `pictura-render::groups_and_masks_match_cpu`, `pictura-render::fill_layers_match_cpu` |
| Group semantics | Composite pass-through and isolated groups. | Within ±1 LSB for the group and its children. | `pictura-render::groups_and_masks_match_cpu`, `pictura-render::proxied_type_in_groups_matches_cpu` |
| Raster source parity | Composite proxy-less type, smart-filtered smart object, and channel-less smart-object sources. | Within ±1 LSB, including the smart-filter mask mix and layer mask. | `pictura-render::raster_sources_match_cpu` |
| Fill layer parity | Composite Solid, Gradient, and Pattern fill layers. | Within ±1 LSB. | `pictura-render::fill_layers_match_cpu`, `pictura-render::fill_scene_matches_cpu` |
| Effect and clipping parity | Composite an enabled effect and a clipping group. | Within ±1 LSB. | `pictura-render::effects_match_cpu`, `pictura-render::clipping_matches_cpu` |
| Dissolve mode | Call the GPU compositor on a Dissolve stack. | `Err(GpuError::UnsupportedMode(Dissolve))`. | `pictura-render::dissolve_still_cpu_only` |
| Supported adjustment composites | Call the GPU compositor on each supported adjustment kind. | `Ok`; adjustment applied at the stack position. | `pictura-render::adjustment_layers_match_cpu`, `pictura-render::adjustment_luts_match_cpu` |
| Unsupported adjustment kind | Call the GPU compositor on an unsupported adjustment layer. | `Err(GpuError::UnsupportedAdjustment)`. | `pictura-render::unsupported_adjustment_falls_back_or_errors`, `pictura-render::adjustment_without_a_gpu_kind_declines_at_any_rect` |
| Advanced blending still declines | Call the GPU compositor on a non-default Blend If or knockout. | `Err(GpuError::UnsupportedAdvancedBlending)`. | `pictura-render::advanced_blending_still_declines` |
| A document above the one-dimensional limit composites on the GPU | Composite 4000×4000 on a device. | `Backend::Gpu`; within ±1 LSB; alpha unchanged. | `pictura-render::large_document_over_1d_limit_composites_on_gpu` |
| A document above the two-dimensional product limit is rejected | Call the GPU compositor on a document past the 2-D product limit. | `Err(GpuError::TooLarge)`; no panic. | `pictura-render::document_past_2d_product_limit_returns_too_large` |
| The parity contract holds after the data-path change | Repeat a composite, compare region to the full slice, and compare to the CPU oracle. | Repeated bytes identical; region slice equal at 0 LSB; within ±1 LSB. | `pictura-render::fresh_white_document_composites_uniform_white_twice`, `pictura-render::full_region_equals_full_composite`, `pictura-render::gpu_matches_cpu_within_one_lsb` |
| The ignored profile test prints the phase timings | Run the ignored 4000×4000 profile on a device. | Prints per-phase timings and the total. | `pictura-render::composite_profile_4000` |
| A full composite stays within a small time budget | Time the 4000×4000 full GPU composite after warm-up. | Completes within a small budget. | manual: timings vary by machine; the profile test prints evidence without asserting a ratio |
| Every declining content route has a reason | Run the eligibility check on each unsupported content route. | Returns that route's `GpuError` with no device. | `pictura-render::every_declining_route_names_its_reason` |
| A proxied type layer stays eligible with the gamma on | Run eligibility on a proxied type layer with the gamma on. | Accepted. | `pictura-render::proxied_type_layer_with_gamma_is_eligible`, `pictura-render::proxied_type_gamma_values_and_gates_match_cpu` |
| A channel-less type layer is accepted | Call the GPU compositor on a renderable channel-less type layer. | Accepted; composites the cached type raster. | `pictura-render::channel_less_type_layer_is_accepted` |
| An unrenderable channel-less type layer declines | Run eligibility on a type layer whose raster cannot render. | `Err(GpuError::UnsupportedText)`; counts `type_source`. | `pictura-render::unrenderable_type_source_declines` |
| An unrenderable proxy-less smart object declines | Run eligibility on a smart object whose source cannot render. | `Err(GpuError::UnsupportedSmartObject)`. | `pictura-render::unrenderable_smart_source_declines` |
| A decoded vector fill is accepted | Run eligibility on a decodable `vscg` fill. | Accepted on the fill route. | `pictura-render::decoded_vector_fill_is_accepted` |
| Effects and clipping no longer decline | Run eligibility on an enabled drop shadow and a clipping group. | Accepted. | `pictura-render::effects_and_clipping_are_accepted`, `pictura-render::disabled_effects_stay_eligible` |
| The CPU oracle is unchanged | Composite one document per content route with `composite_rgba`. | Bytes equal the recorded baseline. | `pictura-render::cpu_oracle_unchanged_for_every_route` |
| A drop-shadow layer composites on the GPU | Composite the drop-shadow fixture. | `Ok`; shadow at the CPU position; within ±1 LSB. | `pictura-render::effects_match_cpu` |
| Every effect family matches the CPU | Composite each effect-family fixture. | Within ±1 LSB. | `pictura-render::effects_match_cpu` |
| A multi-effect layer keeps its draw order | Composite a layer with drop shadow, stroke, and inner shadow. | Below/content/above order; within ±1 LSB. | `pictura-render::effect_draw_order_matches_cpu` |
| A disabled or undecodable effect stays eligible | Composite a disabled effect and the legacy/undecodable fixtures. | Accepted; no pass; matches the CPU. | `pictura-render::disabled_effects_stay_eligible`, `pictura-render::effects_match_cpu` |
| An enabled effect the CPU renders as a no-op stays eligible | Composite the bevel, legacy, stroke, and overlay fixtures. | Accepted; no pass for the no-op; matches the CPU. | `pictura-render::effects_match_cpu` |
| Effect region parity | Composite a styled document over a sub-region and whole. | Every region pixel equals the full-composite pixel. | `pictura-render::new_routes_region_matches_full_slice` |
| An edited depth-16 layered document saves at 16-bit with native samples | Edit and save a layered 16-bit RGB document. | Header depth 16; a native sample is not the 8-bit widening. | `pictura-render::refresh_splices_native_color_planes_into_a_layered_depth_document` |
| Alpha and extra channels are preserved | Save a refreshed layered 16-bit document. | Alpha/document extra planes and channel count unchanged. | `pictura-render::refresh_splices_native_color_planes_into_a_layered_depth_document` |

Manual checks: the diverging-device and stroke-fallback rows above. Forcing a
device whose validation scene diverges is not reproducible on lavapipe/WARP, and
adding a production test hook is outside this change. Other GPU rows are
device-dependent and self-skip or `#[ignore]` with their existing reason strings
when no device is present.
