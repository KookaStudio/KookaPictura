# Test Plan — advanced-blending-toggles

Every scenario in the change's spec deltas maps to one row. The Coverage cell
names the automated test that will pin it; tests that do not exist yet are
written in the task group that owns the behaviour.

## codec/psd-advanced-blending

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| A deep-knockout block decodes | Read a tagged PSD with `knko=2` | Knockout is Deep; no `knko` in `extra_blocks` | `pictura-codec::knko_clbl_infx_decode_and_leave_extra_blocks` |
| Absent blocks use CS6 defaults | Read a tagged PSD with no advanced-blending tags | None/true/false/true/false/false/empty | `pictura-codec::advanced_blending_defaults_when_blocks_absent` |
| Empty knko payload does not fail the document | Read a tagged PSD with a zero-length `knko` | Read succeeds; knockout None | `pictura-codec::empty_knko_payload_keeps_default_and_reads` |
| `cbl_empty_bool` | Read zero-length `clbl`, `infx`, `tsly`, `lmgm` and `vmgm` | Every field keeps its default | `pictura-codec::empty_boolean_payloads_keep_defaults` |
| `cbl_infx_on` | Read a tagged PSD with `infx=1` | `blend_interior` is true and the block is consumed | `pictura-codec::infx_byte_one_sets_blend_interior` |
| `cbl_tsly_off` | Read a tagged PSD with `tsly=0` | `transparency_shapes` is false | `pictura-codec::tsly_zero_disables_transparency_shapes` |
| `cbl_lmgm_vmgm` | Read a tagged PSD with `lmgm=1` and no `vmgm` | Layer mask flag true; vector mask flag false | `pictura-codec::lmgm_and_vmgm_decode_independently` |
| `cbl_brst` | Read a tagged PSD with `brst=[2,0]` | List is `[2,0]`; file order preserved; block consumed | `pictura-codec::channel_restrictions_decode_in_file_order` |
| Deep knockout round-trips | Write and re-read a Deep-knockout layer | File has `knko=2`; mode reads back Deep | `pictura-codec::advanced_blending_non_defaults_round_trip_and_defaults_omit_keys` |
| Default fields are omitted | Write a document with every field at default | No advanced-blending tags in the bytes | `pictura-codec::advanced_blending_non_defaults_round_trip_and_defaults_omit_keys` |
| `cbl_infx_write` | Write a layer with `blend_interior` true | File has `infx=1`; reads back true | `pictura-codec::infx_is_written_only_when_on` |
| `cbl_flags_roundtrip` | Write `tsly=0`, `lmgm=1`, `vmgm=1` | All three blocks present with 0/1/1; fields read back | `pictura-codec::transparency_and_mask_flags_round_trip` |
| `cbl_brst_write` | Set the list to `[2,0,2]`; write and read | One `brst` block with 0 and 2; read list `[0,2]` | `pictura-codec::channel_restrictions_write_sorted_and_deduped` |
| `cbl_brst_unknown` | Read `brst=[0,7]`; write again | List is `[0,7]`; both values re-emitted | `pictura-codec::unknown_channel_index_round_trips` |
| `cbl_brst_malformed` | Read a two-byte `brst` | Read succeeds; list empty; the two bytes re-emitted unchanged | `pictura-codec::malformed_channel_restrictions_stay_opaque` |

## compositing/advanced-blending-toggles

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `abt_clbl_off` | Composite a 50 % Normal base with a clipped Multiply member at 40 %, flag off | Member uses Multiply at 40 %; base opacity does not fade it | `pictura-render::clbl_off_keeps_member_mode_and_opacity` |
| `abt_clbl_on_default` | Composite the same stack with the flag on | The 50 % base fades the group to the expected mid-tone | `pictura-render::clbl_on_default_is_unchanged` |
| `abt_clbl_off_bounded` | Extend a member past the base with the flag off | Member appears only over base content | `pictura-render::clbl_off_still_bounds_the_member` |
| `abt_infx_on_folds` | Multiply layer + 100 % Normal Color Overlay, flag on | Overlay folds; Multiply applies once | `pictura-render::blend_interior_on_folds_interior_effects` |
| `abt_infx_off_default` | The same layer with the flag off | Overlay uses its own Normal mode | `pictura-render::blend_interior_off_uses_effect_modes` |
| `abt_infx_no_fold_identity` | Layer with no fold effect, flag on and off | Byte-identical composites | `pictura-render::blend_interior_without_fold_effects_is_byte_identical` |
| `abt_infx_after_blend` | Inner Shadow, Stroke and Bevel with folded overlays | All three stay on the blended group result | `pictura-render::blend_interior_keeps_inner_shadow_after_the_group` |
| `abt_infx_fill_layer` | Multiply solid-fill layer + Color Overlay, flag on | Fill and overlay form one group; Multiply applies once | `pictura-render::blend_interior_folds_a_fill_layer` |
| `abt_infx_inert_on_non_content` | Group or destructive adjustment, flag on | Byte-identical to the flag off | `pictura-render::blend_interior_is_inert_on_groups_and_adjustments` |
| `abt_tsly_off_overlay` | Unmasked small square with Color Overlay, `tsly` off | Overlay fills the canvas | `pictura-render::tsly_off_color_overlay_fills_the_canvas` |
| `abt_tsly_off_knockout` | Half-fill knockout, `tsly` off | Whole canvas shows the knockout base | `pictura-render::tsly_off_knockout_cuts_the_canvas` |
| `abt_tsly_off_edge_effects` | Drop Shadow, Outer Glow, Stroke, Inner Shadow, Satin, Inner Glow, Bevel, `tsly` off | No effect changes an in-canvas pixel | `pictura-render::tsly_off_edge_effects_are_absent` |
| `abt_tsly_off_masked` | Half-masked layer with Color Overlay, `tsly` off | Overlay follows the mask when the hide flag is off; output clipped when on | `pictura-render::tsly_off_overlay_still_follows_the_mask_role` |
| `abt_tsly_on_default` | The same effects with `tsly` on | Byte-identical to the content-shaped composite | `pictura-render::tsly_on_is_byte_identical_to_content_shaping` |
| `abt_lmgm_off_source` | Half-masked shadow layer, flag off | Shadow from visible content can spill into the masked half | `pictura-render::lmgm_off_lets_shadow_spill_past_mask` |
| `abt_lmgm_on_output` | The same layer, flag on | Shadow absent wherever the mask is zero | `pictura-render::lmgm_on_clips_shadow_to_mask` |
| `abt_vmgm_on_output` | Vector mask on, raster flag off | Vector clips output; raster still shapes source | `pictura-render::vmgm_on_clips_effects_independently` |
| `abt_mask_disabled` | Disabled mask with either flag on | Byte-identical to the layer without that mask | `pictura-render::disabled_mask_is_inert` |
| `abt_brst_exclude_one` | Opaque green layer over a red backdrop, exclude channel 0 | Red stays 255; G/B blend | `pictura-render::excluded_channel_keeps_backdrop` |
| `abt_brst_effects` | White square + red Color Overlay over green, exclude channel 0 | Neither content nor overlay changes red | `pictura-render::channel_restrictions_cover_effects` |
| `abt_brst_all` | RGB layer excludes 0, 1 and 2 | Composite equals the document without the layer | `pictura-render::all_channels_excluded_removes_layer` |
| `abt_brst_adjustment` | Invert on a non-zero red backdrop, exclude channel 0 | Red equals the pre-adjustment backdrop | `pictura-render::adjustment_leaves_excluded_channel_unadjusted` |
| `abt_brst_transparent` | Restricted layer over a transparent backdrop | Composite stays transparent | `pictura-render::channel_restrictions_leave_a_transparent_pixel_unchanged` |
| `abt_brst_modes` | Grayscale excludes 0; CMYK carries a restriction | Gray keeps the backdrop; CMYK is inert | `pictura-render::channel_restrictions_follow_the_document_mode` |
| `abt_defaults_identity` | Composite a deterministic default fixture | Hand-computed backdrop/content pixels match; the existing suites cover the default effect/clipping paths | `pictura-render::advanced_blending_defaults_are_byte_identical` |
| `abt_gpu_declines` | One non-default per run on a visible layer | `composite_gpu` returns the advanced-blending error | `pictura-render::gpu_declines_each_advanced_blending_toggle` |

## compositing/clipping-masks

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| A clipping group renders as CS6 and psd-tools do | Existing clipping unit and oracle suites | Base fade, Fill 0 clip, hidden base, shared run, unbased member, region match, psd-tools within 2 | `pictura-render::a_clipped_layer_shows_only_over_its_base`, `pictura-render::the_base_opacity_fades_the_group_but_its_fill_does_not`, `pictura-render::a_run_shares_the_base_and_a_bottom_or_hidden_base_behaves`, `pictura-render::a_region_composite_matches_the_full_one`, `pictura-render::psd_tools_composites_a_clipping_group_alike` |
| `clip_clbl_off_mode` | Base flag off, member has Multiply 40 % | Member keeps its own mode/opacity | `pictura-render::clbl_off_keeps_member_mode_and_opacity` |
| `clip_clbl_off_bounded` | Member extends past the base with the flag off | Member is clipped to base content | `pictura-render::clbl_off_still_bounds_the_member` |

## compositing/knockout-compositing

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Deep punches through the intermediate layer | Existing Deep stack | Blue over red; green absent | `pictura-render::deep_knockout_punches_through_to_the_background` |
| Deep punches through a pass-through group to the background | Existing group fixture | Group green absent; blue over red | `pictura-render::pass_through_group_knockout_punches_green_through` |
| Deep stops at an isolated group | Existing isolated-group fixture | Group green punched; yellow below remains | `pictura-render::isolated_group_knockout_punches_green_through` |
| Shallow inside a pass-through group stops at the group backdrop | Existing nested-fixture stack | Group green absent; intervening layer remains | `pictura-render::shallow_group_knockout_stops_at_the_group_backdrop`, `pictura-render::knockout_shallow_in_pass_through_group_matches_psd_tools` |
| Shallow equals Deep at the document root | Composite a top-level Shallow stack | Byte-identical to the Deep result | `pictura-render::shallow_equals_deep_at_the_root` |
| None is a no-op | Non-bottom layer with None | Byte-identical to the stack without the field | `pictura-render::none_knockout_is_byte_identical_to_no_field` |
| A transparent pixel does not punch through | Zero-alpha knockout pixel, shaping on | Backdrop unchanged | `pictura-render::transparent_knockout_pixel_leaves_the_backdrop` |
| `ko_tsly_off_canvas` | The same knockout with `tsly` off, with and without a half mask | Every canvas pixel the masks allow shows the knockout base; with no mask, the whole canvas | `pictura-render::tsly_off_knockout_cuts_the_canvas`, `pictura-render::tsly_off_knockout_respects_the_layer_mask` |
| `ko_tsly_off_masked` | The knockout layer has a half mask and `tsly` off | The masked-out half is not punched through; the middle layer remains | `pictura-render::tsly_off_knockout_respects_the_layer_mask` |

## compositing/layer-effects

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `lef_lmgm_off_source` | Half-masked drop shadow, flag off | Source matte is masked; shadow can spill into the hidden half | `pictura-render::lmgm_off_lets_shadow_spill_past_mask` |
| `lef_lmgm_on_output` | The same layer, flag on | Source unshaped; output absent where the mask is zero | `pictura-render::lmgm_on_clips_shadow_to_mask` |
| `lef_vmgm_independent` | Vector flag on, raster flag off | Vector clips output; raster shapes source | `pictura-render::vmgm_on_clips_effects_independently` |
| `lef_lmgm_on_fill_source` | Fill/shape layer with a mask, flag on | Source box is the whole layer box, not the mask box; output is clipped | `pictura-render::lmgm_on_uses_the_whole_fill_source_box` |
