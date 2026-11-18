# Tasks

## 1. Evidence and verification cases

- [x] 1.1 Before committing to a single shipping PR, confirm the reference is available. If unavailable, report the blocker and obtain user agreement before re-scoping into independently shippable changes. Do not archive unsupported requirements.
- [x] 1.2 Obtain a lawful, locally authored Photoshop reference with a known custom scale and record its resource version, field IDs and types. Keep the local reference outside the tracked tree and use the project’s generators for tracked fixtures. Verify that the observed fields explain the known input calibration before implementing the typed codec.
- [x] 1.3 Add `pictura-core::measurement_scale_converts_length_and_area` and `pictura-core::measurement_scale_rejects_invalid_values`. Verify both names appear in the scenario coverage rows for the calibration value.
- [x] 1.4 Add `tst_measurement_scale::rejectsInvalidCalibration`, `tst_measurement_scale::calibratesFromRulerAndTypedInput`, `tst_measurement_scale::restoresScaleThroughHistoryAndTabSwitch`, `tst_measurement_scale::cancelAndUnchangedScaleAreNoops`, `tst_measurement_scale::standardKeepsUncalibratedRuler`, and `tst_measurement_scale::persistsAndDeletesPresetsIndependently`. Verify the suite is registered with CTest and uses isolated test data.
- [x] 1.5 Add `pictura-codec::measurement_scale_roundtrips_eager_and_lazy_psd_psb`, `pictura-codec::measurement_scale_preserves_untouched_resource_bytes`, `pictura-codec::measurement_scale_replaces_or_resets_opaque_resource`, `tst_measurement_scale::preservesCalibrationThroughEditionSave`, and `tst_measurement_scale::opaqueResourceResetIsUndoable`. Verify generated inputs cover recognized and opaque 1074 plus neighboring resources.
- [x] 1.6 Add `pictura-codec::measurement_export_writes_ordered_utf8_tsv`, `pictura-codec::measurement_export_empty_selection_has_no_output`, `pictura-codec::measurement_export_writes_numbered_histograms`, `pictura-codec::measurement_export_omits_unselected_histograms`, and `pictura-codec::measurement_export_preserves_destinations_on_failure`. Verify the cases exercise file output and a deterministic failure in a temporary directory.

- [x] 1.7 Add `tst_measurement_scale::cancelsDraftOnDocumentChange`, `tst_measurement_scale::checksCurrentScale`, `tst_measurement_scale::resampleResetsCalibration`, `tst_measurement_scale::resizePreservesCalibration`, `tst_measurement_scale::resamplePreservesOpaqueScale`, and `pictura-codec::measurement_scale_preserves_bytes_with_guide_edit`. Cover each corresponding test-plan row.

- [x] 1.8 Add Perspective Crop and background notification cases covering rigid, refused, opaque, and inactive Undo behavior.

## 2. Document scale and PSD/PSB persistence

- [x] 2.1 Implement the validated scale value, default, length and area conversion, and document state for untouched, edited, and reset calibration. Exercise the valid and invalid core cases in the test plan during the final verification run.
- [x] 2.2 Implement version-16 resource 1074 using the observed field layout and the existing descriptor helpers. Resolve it on eager and lazy open and update or remove it in the shared PSD/PSB output path. Verify round-trip, malformed fallback, explicit replacement, byte preservation, and the guide-edit case with the planned codec cases.

- [x] 2.3 Integrate the reversible scale reset into the shared successful document-resize path, preserving opaque resources and calibration on failed or same-dimension resizes. Verify the planned resize cases.

- [x] 2.4 Invalidate calibration for distance-changing Perspective Crop while preserving rigid transforms and opaque resources.

## 3. Calibration controls and presets

- [x] 3.1 Wire Extended-only scale commands and the Custom dialog to the existing Ruler tool, with manual input, canvas calibration, and one history entry per document edit. Verify ruler capture, validation, no-op acceptance, history, and edition gating through the planned Qt cases.
- [x] 3.2 Expose the active scale in the Info panel and document status choices and add calibrated length to the Ruler. Verify display updates after scale edits, Undo, Redo, and a document switch through the planned Qt cases.
- [x] 3.3 Store named scale presets in application data and wire save, apply, and delete. Verify that a preset survives restart and that both calibrated documents retain their values after deletion.

- [x] 3.4 Cancel drafts on active-document change or close and synchronize menu checks. Show the recalibration indication after pixel resampling. Verify the planned Qt cases.

- [x] 3.5 Defer inactive-document recalibration notices and clear stale notices on Undo, document switch, and close.

## 4. Export writers

- [x] 4.1 Implement the Qt-free selected-row TSV and per-histogram CSV writers. Verify Unicode escaping, order, frozen scale values, filenames and references, missing histograms, and empty selection through the planned codec cases.
- [x] 4.2 Add exclusive destination creation and per-attempt cleanup. Verify pre-existing destinations and an injected partial write through the planned failure case.

## 5. Integration and closeout

- [ ] 5.1 After all code and tests are written, run the build within the repository’s 80% CPU budget, then run the planned Rust and Qt tests and repository checks. Rebuild after code fixes only. Verify the test-plan guard resolves every coverage reference.
- [ ] 5.2 Exercise the Photoshop round trip using the local reference and record the result. Include the manual test-plan row in the PR reviewer checklist. Verify the recorded result identifies the Photoshop version and both scale values.
- [ ] 5.3 Record shipped scope in `ROADMAP.md`, keep #510 linked for the Log integration, and archive the change. Verify `openspec validate --all --strict` and the archive guard.

The CMake build passed with the 80% CPU budget. The Roadmap and spec archive
are included in the change. CI will run the automated checks in 5.1 and 5.3.
The PR reviewer checklist carries the Photoshop round trip in 5.2.

