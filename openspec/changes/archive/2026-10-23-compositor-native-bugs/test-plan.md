# Test plan

One row per scenario in the change's deltas. Engine rows run headless; UI rows
run in the Qt Test suite offscreen.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Pass-through equals the flattened stack | Composite the same layers with and without a Pass Through wrapper | Output buffers identical | `pictura-render::pass_through_group_equals_ungrouped_stack` |
| Pass-through with opacity falls back to isolated | Pass Through group at opacity 128 | Isolated composite, Normal blend | `pictura-render::pass_through_group_with_opacity_falls_back_to_isolated` |
| `lc_pass_through_vector_mask` | Pass Through group with a vector mask versus the same stack with the group blend `Normal` | Identical output on CPU and GPU, masked region hidden | `pictura-render::pass_through_group_vector_mask` |
| Default fill is byte-identical | Composite all-fill-255 documents | Byte-identical to the pre-fill composite | `pictura-render::fill_255_is_byte_identical_to_pre_change_composites` |
| Fill scales the layer contribution | Full-alpha layer with fill 128 over an opaque backdrop | Source alpha scaled by 128/255 | `pictura-render::fill_below_255_scales_contribution` |
| Fill combines with opacity and mask | Masked layer with opacity 200 and fill 128 | Contribution scaled by 200/255 × 128/255 × mask | `pictura-render::fill_combines_with_opacity_and_mask`, `pictura-render::masked_layer_zeroes_masked_alpha` |
| GPU matches the CPU oracle with fill | Non-default fill scene on GPU and CPU | ±1 LSB parity per channel | `pictura-render::fill_scene_matches_cpu` |
| Group fill is inert | Group with non-default fill | Composite equals fill 255; value round-trips | `pictura-render::fill_255_is_byte_identical_to_pre_change_composites` |
| `lc_adjustment_ignores_fill` | Invert adjustment at Fill 0/50/100, opacity 0/50/100, on CPU and GPU when available | Fill has no effect; opacity gates the invert | `pictura-render::adjustment_ignores_fill`, `pictura-render::fill_layer_keeps_fill` |
| `le_layer_opacity_styles` | Drop Shadow + stroke at opacity 0 and 128 | No contribution at 0; both scaled by 128/255 at 128 | `pictura-render::layer_opacity_scales_effects` |
| `le_effects_reach_spread` | Spread 100; damage a region inside the spread but outside the blur-only reach | Region slice equals the full composite | `pictura-render::effects_reach_includes_spread` |
| `le_smart_object_matte` | Placed PSD with transparent corners + Drop Shadow; an unreadable embedded source keeps the fallback | Shadow follows the source alpha, not the rectangle; an unreadable source uses the opaque-rectangle fallback | `pictura-render::placed_smart_object_effect_matte`, `pictura-render::channel_less_smart_object_shadow_covers_the_layer_rect` |
| Merge Down composites two layers | Build a masked upper layer over a raster lower layer and call `merge_scope(Down)` | One layer whose pixels and alpha match the compositor oracle, including the mask and the union of nontransparent areas | `pictura-render::merge_down_replaces_pair_and_inherits_lower`, `pictura-render::merge_down_applies_masks_into_alpha` |
| Merge Layers replaces the selection at the topmost position | Select two or more layers and call `merge_scope(Selected)` | One layer at the topmost selected position, composite pixels, Normal/255 | `pictura-render::merge_layers_lands_topmost_and_resets_blend` |
| The merge label follows a single selection | Select one layer, refresh the command registry, read the `layer.merge.layers` action text | Action text is `Merge Down` | `tst_merge_commands::layerMenuContextSensitive` |
| The merge label follows a multiple selection | Select two layers, refresh, read the action text and the Layer menu entries | Action text is `Merge Layers`; no separate `Merge Down` action exists | `tst_merge_commands::layerMenuContextSensitive` |
| No separate Merge Down item remains | With one layer selected, inspect the command table and the Layer menu | Exactly one merge action exists; `layer.merge.down` has no action or handler | `tst_merge_commands::layerMenuContextSensitive` |
| The merge command is disabled with no selection | Clear the panel selection, refresh, read the action | Disabled and labelled `Merge Layers` | `tst_merge_commands::layerMenuContextSensitive` |
| Ctrl+E merges down with one selected layer | Select one layer, assert `Ctrl+E` maps to `layer.merge.layers`, dispatch it | Row count drops by one, history grows by exactly one, undo restores the pair | `tst_merge_commands::layerMenuContextSensitive` |
| Ctrl+E merges only the selected layers | Select two layers above a third, assert `Ctrl+E` maps to `layer.merge.layers`, dispatch it | Only the two selected layers collapse; the third survives; history grows by exactly one | `tst_merge_commands::layerMenuContextSensitive` |
| A multi-selection cannot run Merge Down | With two layers selected, check for `layer.merge.down` and dispatch the retired id | No action/handler; dispatch returns false and history is unchanged | `tst_merge_commands::layerMenuContextSensitive` |
| The Layers panel merge label follows the selection | Open the live Layers panel header menu with one selection, then with two selections | Entry reads `Merge Down` for one and `Merge Layers` for two, never both labels at once | `tst_merge_commands::layersPanelContextSensitiveLabel` |
| The Layers panel merge entry is disabled when the merge cannot run | Open the live Layers panel header menu with only the bottom layer selected | Entry is disabled and reads `Merge Down`; with a valid pair it is enabled | `tst_merge_commands::layersPanelContextSensitiveLabel` |
| An adjustment target is refused | Run Merge Down over an adjustment target and Merge Layers with an adjustment bottom in the engine tests; dispatch a no-layer-below refusal through the Qt bridge | Both engine refusals leave the document unchanged; the bridge refusal records no history state | `pictura-render::merge_down_refuses_adjustment_targets`, `pictura-render::merge_layers_refuses_adjustment_bottom`, `tst_merge_commands::layerMenuContextSensitive` |
| Merge Down needs a layer below | Select the bottom layer only and run Merge Down | Refused; the document and history are unchanged | `pictura-render::merge_down_refuses_without_a_layer_below`, `tst_merge_commands::layerMenuContextSensitive` |
| A merge is one undo step | After each merge in the Qt test, call undo and compare the recovered tree, kinds, names, masks, and sampled pixels | The exact pre-merge state is restored in one undo step | `tst_merge_commands::layerMenuContextSensitive` |
| `merge_clipped_layer_preserves_clip` | Merge a clipped layer into a clipped target, then a clipped layer into its base | Whole-document composite unchanged, target clip flag preserved in the first case, upper layers stay in the run | `pictura-render::merge_clipped_layer_preserves_clip` |
| A clipping group collapses into its base | Merge a clipped layer into a raster base with `Merge Clipping Mask` | One raster layer at the base position whose alpha reflects the clipping coverage | `pictura-render::clipping_mask_folds_base_alpha_into_siblings` |
| A non-raster base is refused | Run `Merge Clipping Mask` with an adjustment or fill-content base | The command is refused and the document is unchanged | `pictura-render::clipping_mask_refuses_non_raster_base` |
| `merge_clipping_mask_applies_coverage_once` | Merge a clipped layer into a partial-alpha raster base | Merged pixels equal the pre-merge composite (coverage applied once, not squared) | `pictura-render::merge_clipping_mask_applies_coverage_once` |
| 16-bit input is preserved at 16-bit | Convert a 16-bit RGB buffer | Same number of 16-bit samples | `pictura-color::identity_rgb16` |
| `cm_convert_32bit` | Convert a 32-bit float RGB buffer | Same number of finite f32 samples | `pictura-color::identity_rgb32` |
| Unsupported format is rejected | Call `convert` with 2 channels or 12 bits | `ColorError::Unsupported` | `pictura-color::unsupported_format_is_rejected` |
| `picc_convert_16bit` | 16-bit ramp → Convert to Profile → save/reopen | >256 destination-profile levels; working copy matches native | `pictura-codec::profile_conversion_preserves_native_samples` |
| `picc_convert_32bit` | 32-bit float samples outside [0,1] → Convert to Profile | Native samples stay f32 and are not pre-clamped; alpha/mask unchanged | `pictura-codec::profile_conversion_does_not_clamp_32bit` |

No manual-only rows.
