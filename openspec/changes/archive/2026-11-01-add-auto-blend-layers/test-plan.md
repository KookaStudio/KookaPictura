# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `cla_soft_base` | Clip an Invert to half-alpha and half-masked bases | Alpha equals the base's, colour fully inverted | `pictura-render::clipped_adjustment_keeps_a_soft_base_alpha`, `pictura-render::clipped_adjustment_follows_a_half_mask` |
| `cla_region` | Composite a region serially and in parallel | Matches the full composite | `pictura-render::clipped_adjustment_region_matches_full` |
| `abl_refused` | Ask availability for refused modes and selections | Unavailable, document unchanged | `pictura-render::auto_blend_refuses_modes`, `pictura-render::auto_blend_refuses_selected_kinds`, `tst_auto_blend::refusedModes` |
| `abl_outside_selection` | Select two rasters beside a Smart Object | Available | `pictura-render::auto_blend_ignores_a_smart_object_outside_the_selection` |
| `abl_skipped` | Mixed selection | Only rasters gain masks | `pictura-render::auto_blend_skips_ineligible_layers` |
| `abl_menu` | Open Edit with one and two layers | Disabled, then enabled | `tst_auto_blend::menuEnablement` |
| `abl_dialog` | Open the dialog | Panorama, Seamless off | `tst_auto_blend::dialogDefaults` |
| `abl_undo` | Run with Seamless, undo | One state; composite and layers restored | `tst_auto_blend::oneHistoryStateAndUndo` |
| `abl_cancel` | Cancel mid-run | Unchanged | `tst_auto_blend::cancelMidRun`, `pictura-render::auto_blend_cancel_returns_no_plan` |
| `abl_stale` | Edit during a run | Plan dropped, error | `tst_auto_blend::staleRunIsDropped` |
| `abl_selection` | Run with corrections | Processed layers selected | `tst_auto_blend::selectionAfterCommit` |
| `abl_focus` | Blend a crossed-focus pair | Composite focus beats each input | `pictura-render::stack_images_beats_each_input_focus` |
| `abl_exposure` | Blend a bracket | Better-exposed regions win | `pictura-render::stack_images_prefers_good_exposure` |
| `abl_seam` | Object inside an overlap | Object from one layer | `pictura-render::panorama_seam_avoids_a_difference` |
| `abl_no_overlap` | Disjoint and contained layers | Content kept; contained black | `pictura-render::panorama_keeps_disjoint_layers`, `pictura-render::panorama_contained_layer_is_hidden` |
| `abl_geometry` | Off-canvas layer, nested groups | Masks match rects | `pictura-render::auto_blend_handles_off_canvas_layers`, `pictura-render::auto_blend_spans_groups` |
| `abl_depth` | Blend 16- and 32-bit layers | Depth and planes unchanged | `pictura-render::auto_blend_keeps_16_bit_depth`, `pictura-render::auto_blend_keeps_32_bit_depth` |
| `abl_replace_mask` | Replace a retained native mask, undo, reload | New mask gates; undo restores; reload matches | `pictura-render::replace_layer_mask_drops_the_native_plane`, `pictura-render::replace_layer_mask_round_trips` |
| `abl_determinism` | Run twice; run oversized | Identical bytes; within the limit | `pictura-render::auto_blend_is_deterministic`, `pictura-render::auto_blend_scale_stays_within_the_limit` |
| `abl_seamless` | Gain mismatch in RGB and Grayscale | Difference narrows, pixels identical | `pictura-render::seamless_narrows_a_gain_mismatch`, `pictura-render::seamless_handles_grayscale` |
| `abl_identity` | Matching layers | No correction | `pictura-render::seamless_identity_adds_nothing` |
| `abl_placement` | Clipped run in a nested group | Correction above the base; paths remap | `pictura-render::correction_sits_below_a_clipped_run` |
| `abl_rerun` | Repeat runs, Seamless off, rename | No stacking; removed; renamed stays | `pictura-render::rerun_replaces_the_correction`, `pictura-render::rerun_keeps_a_renamed_correction` |
| Seam quality on real photographs | Blend an overlapping panorama and a focus stack from a camera | No visible seams, sharp detail throughout | manual: judging seams on photographs needs a person |
| CS6 comparison | Compare the same stack with CS6 | Comparable masks and tones | manual: no CS6 oracle exists; the algorithm is an approximation |
