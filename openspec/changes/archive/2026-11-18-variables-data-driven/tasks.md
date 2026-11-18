# Tasks

The checked implementation items record code and test authoring. The local CMake
build passed with the 80% CPU budget. Test execution and the remaining verification
gates are assigned to CI.

## 1. Test coverage and format evidence

- [x] 1.1 Author `variable_bindings_follow_layer_ids`, `variable_bindings_validate_names_and_targets`, `variable_bindings_restore_on_undo`, `variable_data_sets_keep_typed_values`, and `variable_linked_defaults_follow_stack_order` with the inputs and assertions in `test-plan.md`.
- [x] 1.2 Author `variable_apply_is_one_edit`, `variable_empty_text_stays_editable`, `variable_skip_keeps_current_pixels`, `variable_apply_failure_is_atomic`, `variable_pixel_placement_respects_frame`, `variable_pixels_save_in_native_encoding`, and `variable_empty_frame_is_refused` with the mixed-layer and pixel fixtures in `test-plan.md`.
- [x] 1.3 Record the resource observations from the public references with source credit. Record the independently observed `ratio` / centered / unclipped As Is mapping and establish semantics for other combinations, remaining placement modes, visibility, and Do Not Replace from independent observations before treating them as Photoshop values; otherwise report and preserve them as unsupported. Record observed values and Kooka-authored fallbacks separately. Author independent fixture bytes and `psd_variables_read_mesa_templates`, `psd_variables_preserve_unsupported_blocks`, `psd_variables_roundtrip_without_layer_comps`, `psd_variables_rewrite_resources_selectively`, `psd_variables_ratio_as_is`, `psd_variables_preserve_unknown_placement`, and `psd_variables_preserve_duplicate_ids_with_comps` as specified in the test plan.
- [x] 1.4 Author `variable_import_reads_delimited_encodings`, `variable_import_first_column_is_a_binding`, `variable_import_failure_preserves_sets`, `variable_import_commits_once_and_keeps_paths`, and `variable_import_single_column_delimiters` with the parameter sets in `test-plan.md`.
- [x] 1.5 Author `variable_export_uses_a_fresh_baseline`, `variable_export_avoids_path_collisions`, and `variable_export_failure_keeps_completed_files` with a generated template, a temporary destination, and deterministic failure and cancellation cases.
- [x] 1.6 Register `tst_variables` in the Qt test harness and author `commandsTrackTemplateState`, `definitionAndSetCommitTogether`, `cancelDiscardsStagedEdits`, `previewSwitchAndCancel`, `exportDialogCancelsWithoutWriting`, and `desktopOpenLoadsTemplateMetadata`. Verify every scenario has a registered test target in `test-plan.md` using `scripts/check-test-plan.py` after the test sources are present.

## 2. Document state and application

- [x] 2.1 Add the typed document store, stable layer binding validation, edit operations and set naming. Reuse the shared layer ID machinery and connect rename, duplicate, and delete behavior to document transactions. Verify against the binding and typed-value checks in 1.1.
- [x] 2.2 Build raster placement for the four scale methods, nine positions and applicable clipping. Route image decode and native-depth conversion through the existing paths and preserve layer metadata. Verify with the placement, native encoding, and empty-frame checks in 1.2.
- [x] 2.3 Reuse the type editor's modeled layer state and rendering with a variable-specific empty-string path. Evaluate one set against scratch state and commit once through the document transaction. Verify mixed apply/undo/redo, editable empty text persistence, no-op Do Not Replace, and atomic failure using 1.2.

## 3. PSD interoperability

- [x] 3.1 Read the `MeSa` resources into the typed store after the PSD layer tree is available. Capture original `lyid` multiplicities and blocks before comp ID normalization; resolve against original identity, retain ambiguous IDs and dependent opaque comp and variable resources across unrelated saves, and reject edits that would overwrite that evidence. Share resolution between eager and lazy open paths after layer-tree construction. Check XML and ID bounds and preserve raw records when only part of a pair is understood. Verify the read and preservation checks in 1.3.
- [x] 3.2 Implement selective `MeSa` writes and shared `1044` seed handling, including the all-variables-removed path. Keep unrelated image resources and empty-document output intact. Verify generated PSD/PSB reopen, layer IDs, and resource-chain behavior using 1.3.

## 4. Import and export boundaries

- [x] 4.1 Implement the bounded text decoding, quote-aware CSV/TSV parser, typed validation, and path resolution. Integrate one atomic set collection edit for append and replace. Verify the import tests in 1.4.
- [x] 4.2 Build batch planning for selected sets and names, applying each set to a fresh copy of the baseline. Use the PSD writer with the document profile and publish complete outputs without overwrite. Skip and report every per-set evaluation rejection; stop on destination or output failure and cancellation. Verify the three export checks in 1.5.

## 5. Qt workflow

- [x] 5.1 Wire the five existing catalogue entries, the two-page Variables dialog, and the shared import options. Stage edits and layer IDs through the Rust/Qt bridge. Verify menu gating, desktop Open metadata resolution, and dialog commit and cancel with the Qt suite in 1.6.
- [x] 5.2 Connect the Apply chooser and Data Sets Preview to the isolated canvas presentation. Wire the export dialog, progress, and error summary. Verify the preview and export dialog checks in 1.6.

## 6. Integration and shipping

- [ ] 6.1 Run the Rust and Qt suites and repository verification gates in CI. Exercise generated fixtures without the local reference corpus present.
- [x] 6.2 Update both #622 entries in `ROADMAP.md`, sync the main specs, and archive the change for the shipping PR. Opening and applying a Kooka-authored mixed template in Photoshop remains a follow-up.
