# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `abl_dialog` | Open the dialog | Panorama, boxes off, illustrations shown | `tst_auto_blend::dialogDefaults` |
| `abl_fill_depth` | Open at 8 and 16 bits | Enabled, then disabled | `tst_auto_blend::fillFollowsDepth` |
| `abl_undo` | Run with Seamless, undo | One state; composite and layers restored | `tst_auto_blend::oneHistoryStateAndUndo` |
| `abl_cancel` | Cancel mid-run | Unchanged | `tst_auto_blend::cancelMidRun`, `pictura-render::auto_blend_cancel_returns_no_plan` |
| `abl_stale` | Edit during a run | Plan dropped, error | `tst_auto_blend::staleRunIsDropped` |
| `abl_selection` | Run | Processed layers selected | `tst_auto_blend::selectionAfterCommit` |
| `abl_focus` | Crossed-focus pair | Composite focus beats inputs | `pictura-render::stack_images_beats_each_input_focus` |
| `abl_soft` | Stack two tones | No hard step | `pictura-render::stack_images_blends_softly` |
| `abl_exposure` | Bracket | Better exposure wins | `pictura-render::stack_images_prefers_good_exposure` |
| `abl_seam` | Object in an overlap | Object from one layer | `pictura-render::panorama_seam_avoids_a_difference` |
| `abl_no_overlap` | Disjoint and contained layers | Content kept; contained black | `pictura-render::panorama_keeps_disjoint_layers`, `pictura-render::panorama_contained_layer_is_hidden` |
| `abl_geometry` | Off-canvas layer, groups | Masks match rects | `pictura-render::auto_blend_handles_off_canvas_layers`, `pictura-render::auto_blend_spans_groups` |
| `abl_depth` | 16- and 32-bit runs | Depth kept, merged native | `pictura-render::auto_blend_keeps_16_bit_depth`, `pictura-render::auto_blend_keeps_32_bit_depth` |
| `abl_replace_mask` | Replace a native mask, undo, reload | New gates; undo restores; reload matches | `pictura-render::replace_layer_mask_drops_the_native_plane`, `pictura-render::replace_layer_mask_round_trips` |
| `abl_determinism` | Run twice; oversized | Identical; within limit | `pictura-render::auto_blend_is_deterministic`, `pictura-render::auto_blend_scale_stays_within_the_limit` |
| `abl_placement` | Clipped run in a group | Above the run, paths unmoved | `pictura-render::merged_sits_above_a_clipped_run` |
| `abl_rerun` | Repeat, rename | No stacking; renamed stays | `pictura-render::rerun_replaces_the_merged_layer`, `pictura-render::rerun_keeps_a_renamed_merged_layer` |
| `abl_seamless` | Gain mismatch, RGB and Gray | Narrower, no adjustment, sources identical | `pictura-render::seamless_narrows_a_gain_mismatch`, `pictura-render::seamless_handles_grayscale` |
| `abl_identity` | Matching layers | Tone kept | `pictura-render::seamless_keeps_matching_tones` |
| `abl_fill` | Uncovered canvas, option on | Opaque, content colours | `pictura-render::content_aware_fill_covers_the_canvas` |
| `abl_fill_ignored` | 16-bit run | Transparency kept | `pictura-render::content_aware_fill_is_ignored_at_high_depth` |
| Stack on real photos | Repeat the horse and lily stack beside Photoshop | Soft blend, no rectangular edges | manual: judging a photograph needs a person; no Photoshop oracle |
| Dialog illustrations | Open the dialog in each theme | Legible at normal size | manual: visual judgement |
