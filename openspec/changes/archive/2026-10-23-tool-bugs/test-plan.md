# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `lbt_auto_erase_pencil_only` | Enable Auto Erase on the Pencil, switch to Brush, paint over the foreground colour | Brush paints the foreground and records one "Brush" state | `tst_options_bars_parity::pencilAutoEraseDoesNotLeakToBrush` |
| `lm_healing_refuses_mask` | Select a layer mask, press Spot Healing or Healing Brush | Refusal message; no state; layer RGB and mask coverage unchanged | `tst_mask_edit::healingRefusesMaskTarget` |
| `pe_clear_zeroes_rgb` | Clear at full opacity and full coverage on an opaque coloured pixel | Alpha 0 and RGB 0 | `pictura-paint::clear_zeroes_rgb` |
| `pe_clear_keeps_rgb` | Clear at 50 % opacity on an opaque coloured pixel | Alpha falls; RGB keeps the pre-stroke colour | `pictura-paint::clear_keeps_rgb_below_full` |
| `pe_noop_no_state` | Behind over opaque; Normal at Opacity 0; Clear on a layer without alpha; a clone source outside the image | No dirty rectangle, no state, document not dirty in each case | `pictura-paint::noop_stroke_adds_no_history`, `pictura-paint::a_clone_stroke_copies_the_snapshot_at_its_offset_and_skips_off_image_sources` |
| `mx_mode_multiply` | Mixer dab at Multiply over an opaque base | Channel-wise product of deposit and base | `pictura-paint::mixer_applies_mode_opacity` |
| `mx_opacity_cap` | Mixer dab at Opacity 50 % over an opaque base | Half-way colour | `pictura-paint::mixer_applies_mode_opacity` |
| `mx_options_expose` | Activate the Mixer Brush; change Mode and Opacity | Both controls are present and write the shared paint options used by the next stroke | `tst_options_bars_parity::mixerExposesModeOpacity` |
| `Zero radius is identity` | `expand(0)` and `contract(0)` | Selection unchanged | `pictura-select::expand_contract_roundtrip` |
| `Expand then contract restores a binary block` | Expand then contract a block clear of the edge | Original block | `pictura-select::expand_contract_roundtrip` |
| `sm_contract_edge` | Contract the all-pixels selection by n | Every side shrinks by n | `pictura-select::contract_all_shrinks` |
| `sm_border_edge_band` | Border an edge-touching selection | Inner band present along the edge | `pictura-select::border_edge_has_inner_band` |
| `qs_union_one_state` | Drag Quick Selection across two disjoint regions | Union selected; exactly one "Selection" state | `tst_tool_commit::quickSelectionOneStatePerStroke` |
| `qs_new_to_add` | Complete one New-mode Quick Selection stroke | Shared mode reads Add; next stroke adds | `tst_tool_commit::quickSelectionOneStatePerStroke` |
| `qs_cancel_restores` | Start a Quick Selection drag, then cancel before release | Pre-stroke selection and history restored exactly | `tst_tool_commit::quickSelectionCancelRestores` |
| `re_distance_pixels` | Measure a straight edge at 1 and 4 columns | Distances 1 and 4; radius 4 reaches only that far | `pictura-select::refine_distance_is_pixels` |
| `re_preview_scale` | Preview a large document at a reduced proxy size | Radius/Smooth/Feather scale; Contrast/Shift Edge do not | `pictura-app::preview_params_scale_with_the_proxy` |
| `re_preview_commit_agree` | Preview a straight edge at Radius R and Feather F on a large document, then commit; compare the edge position and the ramp width at proxy scale | Preview and committed edge position and width agree within proxy tolerance | `tst_refine_edge::previewMatchesCommitWithinProxyTolerance` |
| `tp_baseline_shift` | Render the same path text with shift 0 and +N | Glyphs, caret, and alpha bounds move by N opposite the normal | `pictura-render::type_on_path_applies_baseline_shift` |
