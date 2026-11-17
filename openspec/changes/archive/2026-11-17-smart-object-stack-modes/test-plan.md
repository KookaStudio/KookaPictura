# Test plan — smart-object-stack-modes

Every scenario in the change's deltas is automated; the coverage column names
the test that proves it.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `smst_model` | Construct a `SmartObject`, assign `Some(StackMode::Median)`, compare | The default reports `None`; the assigned object is unequal to the default | `pictura-core::stack_mode_defaults_and_equality` |
| `smst_frames` | Build a payload with a hidden layer, a visible group, and a nested smart object, reduce it | Only the visible top-level frames contribute; the group is one frame; the nested object contributes its rendered source | `pictura-render::stack_frames_are_visible_top_level_layers`; `pictura-render::stack_frames_render_a_nested_smart_object` |
| `smst_samples` | Reduce frames that are transparent at one pixel and partially cover another | Uncovered pixels are transparent; only covering frames contribute | `pictura-core::reduce_stack_excludes_transparent_samples_and_sets_max_alpha` |
| `smst_output_alpha` | Reduce frames with different partial alpha at a pixel | The output alpha is the largest sample alpha | `pictura-core::reduce_stack_excludes_transparent_samples_and_sets_max_alpha` |
| `smst_formulas` | Reduce a three-frame stack once per mode and compare every output pixel | Each channel equals the rounded, clamped formula result | `pictura-core::reduce_stack_matches_each_formula` |
| `smst_even_median` | Reduce four samples whose middle values differ | The output is the rounded mean of the two middle values | `pictura-core::reduce_stack_even_median_averages_middle_samples` |
| `smst_moment_guards` | Reduce one-sample and equal-sample sets with variance, stddev, skewness, kurtosis | All four results are 0 | `pictura-core::reduce_stack_guards_single_and_zero_deviation_samples` |
| `smst_entropy_scale` | Reduce a pixel with uniform samples across the 8-bit range | The result is `255 / 8` times the entropy in bits, rounded and clamped | `pictura-core::reduce_stack_scales_entropy_to_the_channel_range` |
| `smst_clamp` | Reduce a stack whose summation exceeds 255 and whose moments leave the range | Every output channel stays within 0 through 255 | `pictura-core::reduce_stack_clamps_summation_and_moments` |
| `smst_non_cumulative` | Render with Mean, then with Maximum, and compare to a fresh Maximum | The two Maximum renders are byte-identical | `pictura-render::stack_mode_mean_then_maximum_is_not_cumulative` |
| `smst_set` | Apply Median to an eligible object, then None | The object reports Median, then `None` | `pictura-render::set_stack_mode_sets_and_clears` |
| `smst_refuse_frames` | Apply a mode to a one-frame embedded object | Returns false; document unchanged | `pictura-render::set_stack_mode_refuses_fewer_than_two_frames` |
| `smst_refuse_target` | Apply a mode to a group, adjustment, non-smart layer, unreadable payload, and missing path | Every call returns false and mutates nothing | `pictura-render::set_stack_mode_refuses_ineligible_targets` |
| `smst_render_median` | Composite a three-frame stack with Median active | Each pixel is the per-channel median, placed in the layer rect | `pictura-render::stack_mode_renders_the_per_pixel_median` |
| `smst_render_filters` | Composite a stack-mode object with a decodable enabled Smart Filter | The filter applies once to the reduced source | `pictura-render::stack_mode_applies_smart_filters_after_reduction` |
| `smst_render_unknown_filter` | Composite a proxied stack-mode object with an enabled Smart Filter whose options cannot be decoded | The proxy renders and the unknown filter is not applied twice | `pictura-render::stack_mode_unknown_filter_keeps_the_proxy` |
| `smst_render_fallback` | Composite a stack-mode object whose payload is unreadable or single-frame | The proxy/source renders with no error | `pictura-render::stack_mode_falls_back_for_an_unusable_stack` |
| `smst_render_clear` | Clear the mode and composite again | The proxy (or unreduced source) renders | `pictura-render::clearing_a_stack_mode_restores_the_proxy` |
| `smst_rasterize` | Rasterize a proxied object with Maximum active | The channels hold the maximum reduction and the object is gone | `pictura-render::rasterize_bakes_the_stack_render` |
| A proxy is left unchanged | Rasterize a proxied smart-object layer with no eligible active stack mode | Its pixel channels are byte-for-byte the same as before | `pictura-render::rasterize_keeps_proxy_channels_and_drops_config` |
| A source-only layer materializes from the payload | Rasterize a channel-less smart-object layer with a decodable embedded payload | Its color channels and `-1` alpha hold the decoded source scaled into the layer rect | `pictura-render::rasterize_materializes_source_into_channels` |
| A stack-mode proxy bakes the reduction | Rasterize a proxied layer with an eligible active Maximum stack mode | Its pixel channels hold the reduced stack source and the layer no longer carries a smart object | `pictura-render::rasterize_bakes_the_stack_render` |
| A non-applicable stack mode keeps the proxy | Rasterize a proxied layer whose stack mode has one frame or an unreadable payload | Its pixel channels are byte-for-byte the same as before | `pictura-render::rasterize_keeps_the_proxy_when_the_stack_does_not_apply` |
| The smart object is cleared | Rasterize a smart-object layer successfully | `layer.smart_object` is `None` | `pictura-render::rasterize_materializes_source_into_channels` |
| Layout fields are untouched | Rasterize a layer | Its name, rect, blend, opacity, fill, mask, and position are unchanged | `pictura-render::rasterize_preserves_layout_fields` |
| `smst_menu_rows` | Inspect the 12 stack-mode actions in an Extended build | All 12 exist with the documented order, labels, and ids | `tst_smart_object_actions::stackModeMenuRows` |
| `smst_menu_state` | Select an eligible object with Median active | Median is checked, None is enabled, ineligible layers disable mode actions | `tst_smart_object_actions::stackModeStateFollowsTheLayer` |
| `smst_menu_undo` | Dispatch the Median command on an eligible object | One `Stack Mode: Median` state is added | `tst_smart_object_actions::stackModeRecordsOneState` |
| `smst_menu_refusal` | Dispatch on an ineligible object, then re-select the active mode | No history state is added and the document is unchanged | `tst_smart_object_actions::stackModeRefusalRecordsNothing` |
| Proxy is used | Composite a proxied object with no stack mode | The proxy renders and the payload is not decoded | `pictura-render::proxy_wins_over_source` |
| No behaviour change for proxied files | Composite proxied objects without a mode | Pixels equal the pre-change result | `pictura-render::proxy_wins_over_source` |
| `sor_stack_over_proxy` | Composite a proxied object with an active mode | The reduced source renders, not the proxy | `pictura-render::stack_mode_overrides_the_proxy` |
| Scale into the rect | Render a source whose size differs from the layer rect | Each destination pixel samples the nearest source pixel | `pictura-render::source_scales_into_layer_rect` |
| Stored composite preferred | Render a payload with a usable stored merged composite | Those pixels are used as the source | `pictura-render::embedded_source_renders_without_proxy` |
| Layers composited as fallback | Render a payload with no usable stored merged composite | The decoded document's layers are composited to produce the source | `pictura-render::absent_composite_falls_back_to_layers` |
| Transform is not applied | Render an object carrying a `Trnf`/warp transform | The source renders axis-aligned in the layer rect, without bilinear resampling | `pictura-render::source_scales_into_layer_rect` |
| An eligible stack mode replaces the stored composite | Render an eligible stack-mode object whose payload also carries a usable stored merged composite | The reduced stack source is used instead of the stored merged composite | `pictura-render::stack_mode_renders_the_per_pixel_median` |
| `lsc_stack_invalidates` | Composite with Median, switch to Maximum, composite again, compare with a purge | The rebuilt source equals the purge-then-composite | `pictura-render::stack_mode_change_invalidates_the_cached_source` |
| `lsc_stack_raster` | Composite twice, change the mode, composite again | The first two composites reuse the raster; the third rebuilds it | `pictura-render::stack_mode_change_invalidates_the_composited_raster` |
| `cso_stack_round_trip` | Write a document with an active mode, read it back | The mode is restored; payload and config descriptor are unchanged | `pictura-codec::stack_mode_marker_round_trips` |
| `cso_stack_cleared` | Clear the mode, write, read, inspect the layer blocks | No mode and no `PiSt` block | `pictura-codec::stack_mode_marker_clears` |
| `cso_stack_unknown` | Write a layer with a `PiSt` block of version 9, read, write again | No mode is set and the block re-emits unchanged | `pictura-codec::unknown_stack_mode_marker_is_preserved` |
| `cso_stack_replaced` | Apply a mode to a layer carrying an unrecognized `PiSt` block, write, inspect the layer blocks | The output carries exactly one `PiSt` block, the modelled marker | `pictura-codec::unknown_stack_mode_marker_is_replaced_by_a_modelled_mode` |
| `cso_stack_proxy` | Capture channel bytes, apply a mode, write | Colour and alpha bytes equal the pre-mode bytes | `pictura-codec::stack_mode_leaves_the_proxy_untouched` |
| Extended-only commands are absent in Standard | Build a Standard window and inspect the 3D/Measurement/Count commands | No action exists | `tst_edition_gating::editionGating` |
| Timeline stays in Standard | Build a Standard window and inspect the Timeline command | The action exists | `tst_edition_gating::editionGating` |
| Extended keeps the commands | Build an Extended window and inspect the 3D/Measurement/Count commands | The actions exist | `tst_edition_gating::editionGating` |
| `edg_stack_absent` | Build Standard and Extended windows and inspect the 12 stack-mode ids | Absent in Standard; registered in Extended | `tst_edition_gating::editionGating` |
