# Tasks

Tests are written before their corresponding production edits. Execution is
deferred until all code is finished, following the compile-last rule.

## 1. Raster mask semantics and creation defaults

- [x] 1.1 Extend `set_linked_round_trips_the_flag_bit_and_preserves_the_rest` for clear=linked, set=unlinked, complete mask-field preservation, and unchanged-document no-ops in both states; verify it covers `pml_link_clears_bit`, `pml_unlink_sets_bit`, and `pml_link_noop` before editing production behavior.
- [x] 1.2 Add `masks_created_by_commands_are_linked`, `rasterized_vector_mask_is_linked`, and `auto_blend_masks_are_linked` in the owning renderer test modules; verify their assertions cover every creation route in `pml_new_mask_linked` before changing those routes.
- [x] 1.3 Rename the raster constant to `MASK_FLAG_UNLINKED`, update re-exports/callers, correct the getter/setter, and initialize linked masks with bit 0 clear in ordinary creation, vector rasterization, and Auto Blend; verify source references use the corrected polarity and, after the final build, the tests from 1.1-1.2 and existing missing-mask reads pass.

## 2. External PSD/PSB link-state coverage

- [x] 2.1 Add small linked/unlinked PSD fixture generators through psd-tools with distinct non-zero layer/mask origins and non-uniform mask pixels, and commit their outputs; verify raw bit values and expected bounds through psd-tools and confirm no pre-existing fixture changed.
- [x] 2.2 Add `linked_external_mask_preserves_flags_and_bounds`, `unlinked_external_mask_preserves_flags_and_bounds`, and `mask_link_toggles_survive_psd_and_psb` to `crates/pictura-codec/tests/mask_link_state.rs`; verify assertions compare the actual renderer predicate and all preserved mask attributes, and after the final build all three tests pass.
- [x] 2.3 Add `psd_tools_reads_both_mask_link_states` with the existing dependency-availability self-skip convention; verify after the final build that independent decoding checks both flag polarities, mask bounds, and coverage, without changing the codec reader/writer's raw-flag representation.

## 3. Mask-control opacity and interaction

- [x] 3.1 Add `tst_mask_link_state.cpp` with `importedMaskStates`, `rasterGlyphOpacity`, `vectorGlyphOpacity`, and `togglesAndHistory`; register it explicitly in the Qt CMake list. Verify tests cover both mask kinds, linked/unlinked imports, DPR 1/2 opacity, unaffected row pixels, clicked-row targeting, and one-step undo/redo before changing the renderer.
- [x] 3.2 Dim the unlinked glyph to 50% in the existing shared link-pixmap path while retaining full opacity for the linked glyph and preserving device-pixel-ratio metadata, artwork, rotation, geometry, and click handling; verify after the final build that the new Qt suite passes and layer-to-layer badge/button behavior remains unchanged.
- [x] 3.3 Record the mask-link interoperability fix and opacity behavior against #1200 in `ROADMAP.md`, preserving links for remaining open items; verify the roadmap matches the delivered scope and production files stay within their existing size ceilings.

## 4. Integrated verification and delivery

- [x] 4.1 After all source and test edits are complete, format and perform the production build using at most 80% of available CPUs; verify the build succeeds and rebuild only when a compiler fix changes code.
- [x] 4.2 Run the codec/render checks named in `test-plan.md`, `tst_mask_link_state`, and the existing Layers-panel and mask-command suites, then the applicable repository verification gates; verify new checks execute rather than silently skip and distinguish any unavailable external-oracle skips in the result.
- [x] 4.3 Run `python3 scripts/check-test-plan.py` and `openspec validate --all --strict`; verify all scenario rows have real test coverage and all OpenSpec artifacts validate. Perform the external Photoshop acceptance row when available, otherwise retain it explicitly as a reviewer checklist item with the local-reference instructions.
- [x] 4.4 Archive `psd-mask-link-state` after implementation validation and include every manual test-plan row in the implementation PR body; verify the corrected main specs contain the new behavior, no open change remains, and the PR/result records the old-Pictura-file compatibility limitation.
