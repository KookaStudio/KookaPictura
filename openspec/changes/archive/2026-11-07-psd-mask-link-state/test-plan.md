# Test plan

All named automated checks are implemented; execution results are recorded in
`verification.md`. Every spec scenario is automated. The extra manual row
checks the external Photoshop application, which the Linux harness cannot
observe.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `pml_link_clears_bit` | Link a non-origin, non-uniform mask with flags `0xA1`; compare its full before/after mask. | Flags become `0xA0`, the predicate reports linked, and every other mask field is unchanged. | `pictura-render::set_linked_round_trips_the_flag_bit_and_preserves_the_rest` |
| `pml_unlink_sets_bit` | Unlink the same mask with flags `0xA0`; compare its full before/after mask. | Flags become `0xA1`, the predicate reports unlinked, and every other mask field is unchanged. | `pictura-render::set_linked_round_trips_the_flag_bit_and_preserves_the_rest` |
| `pml_new_mask_linked` | Create masks through all ordinary mask kinds with valid inputs, rasterize a vector mask, and run Auto Blend. Inspect each created raster mask. | Every mask reads as linked and has bit 0 clear. | `pictura-render::masks_created_by_commands_are_linked`, `pictura-render::rasterized_vector_mask_is_linked`, `pictura-render::auto_blend_masks_are_linked` |
| `pml_link_noop` | Set each link state twice and compare the document before the repeated call. | The repeated call returns false and leaves the entire document equal. | `pictura-render::set_linked_round_trips_the_flag_bit_and_preserves_the_rest` |
| `pml_codec_linked` | Read the psd-tools-authored linked fixture with distinct non-zero layer/mask origins; save/reopen and compare mask attributes. Open it in the app and inspect the Layers row. | Bit 0 stays clear, the renderer and row report linked, and flags, bounds, and coverage are preserved. | `pictura-codec::linked_external_mask_preserves_flags_and_bounds`, `tst_mask_link_state::importedMaskStates` |
| `pml_codec_unlinked` | Repeat using the independent unlinked fixture. | Bit 0 stays set, the renderer and row report unlinked, and flags, bounds, and coverage are preserved. | `pictura-codec::unlinked_external_mask_preserves_flags_and_bounds`, `tst_mask_link_state::importedMaskStates` |
| `pml_codec_toggle` | For PSD and PSB, link and unlink a mask carrying coherent unrelated flags and save/reopen each state. Compare all mask attributes. | The saved state follows each edit; only flag bit 0 differs between the two masks. | `pictura-codec::mask_link_toggles_survive_psd_and_psb` |
| `pml_codec_oracle` | Save both link states and have psd-tools decode each result, including mask channel bytes. | Independent results report clear/set bit 0 respectively and the expected bounds and coverage. | `pictura-codec::psd_tools_reads_both_mask_link_states` |
| `pml_ui_raster_opacity` | Render a raster-mask row linked/unlinked at DPR 1 and 2 over the controlled row background; compare each glyph to its expected compositing and compare geometry and non-glyph pixels. | Linked ink is full strength, unlinked ink is half strength within byte-rounding tolerance, and the rest of the row and geometry are unchanged. | `tst_mask_link_state::rasterGlyphOpacity` |
| `pml_ui_vector_opacity` | Repeat the opacity and non-glyph comparisons for a vector-mask row at DPR 1 and 2. | Vector controls have the same full/half opacity policy with no dimming outside the glyph or geometry change. | `tst_mask_link_state::vectorGlyphOpacity` |
| `pml_ui_toggle_history` | For raster and vector masks on a non-active row, click the dimmed glyph, inspect history, undo, and redo. Inspect both rows and render each state. | Only the clicked mask links, exactly one history step is added, undo restores half-opacity unlinked, and redo restores full-opacity linked. | `tst_mask_link_state::togglesAndHistory` |
| Photoshop linked-file acceptance | Open the local `_auxien/20261009_01.psd` reference in Photoshop and Pictura. Save a separate copy from Pictura and reopen that copy in Photoshop; keep the original untouched. Compare the linked Smart Object mask and its placement. | The mask confirmed linked by the user remains linked and in the same position after the round trip. | manual: the harness cannot inspect the Photoshop application's chain state; the original artwork is an external local reference |

## Execution during apply

- Put new codec tests in `crates/pictura-codec/tests/mask_link_state.rs` and
  new GUI tests in `crates/pictura-app/cpp/tests/tst_mask_link_state.cpp`.
- Register the new Qt suite explicitly. Reuse committed small independent
  fixtures; do not make automated tests depend on the local artwork paths.
- Finish all source/test edits before building; follow the repository's
  compile-last rule and 80% CPU limit.
- After the build, run the targeted Rust and Qt suites and the test-plan
  checker. Copy the manual row above to the implementation PR checklist.
