# Tasks

The CPU-budgeted local CMake build passed. CI owns the automated verification gates. The paired Adobe fixtures and manual Photoshop interoperability checks remain pending because the required Photoshop versions were unavailable.

## 1. Adobe fixture evidence

- [ ] 1.1 Produce paired Always and Never PSD and PSB fixtures in Photoshop CS6 and current desktop CC for an editable layered RGB document. Record version, file layout, presence of a real image or placeholder, Version Info, and whether the existing Kooka layer-only variant opens in each version. Apply repository provenance rules to any fixtures added to the tree.

## 2. Automated coverage

- [x] 2.1 Add `native_save_respects_merged_output_choice`, `native_save_restores_merged_image`, and `native_save_preserves_required_image_data` in pictura-app. Use the test plan to verify both container versions, edited merged pixels, detached buffers, native depth, and protected samples. Add `native_save_restores_adobe_omitted_fixture` using provenance-compliant observed Adobe Never representation, `native_save_restores_missing_native_composite` for 16/32-bit PSD and PSB native samples, `native_depth_save_regressions` for the existing native-depth scenarios, and `native_save_codec_preconditions` for direct writer preconditions.
- [x] 2.2 Add and register `tst_psd_compatibility` with `preferencePersists`, `asksPerSaveAndCancels`, `copyPreservesDocument`, `otherSavesBypassPreference`, and `nativeModesDoNotPrompt`. Verify the test-plan checker resolves every new and existing automated reference.

## 3. Preference and save flow

- [x] 3.1 Add the File Handling selector and persist its value using session settings. Verify the default, invalid-value fallback, and restart behavior in `preferencePersists`.
- [x] 3.2 Resolve the native compatibility decision in interactive Save, Save As, and Save a Copy, including explicit save origin, the protected-data rule, and cancellation before dispatch or ICC save-setting mutation. Cover pathless and explicit-path control-server calls and Action replay under Ask. Verify with the remaining `tst_psd_compatibility` cases and the existing Save As options tests.

## 4. Native save output

- [x] 4.1 Carry the per-save choice into the native worker and apply it to the snapshot before staging. Omit without rendering unnecessary merged pixels and restore a correct merged image when including it. Create a missing native store before refresh for Include, preserving 16/32-bit layer samples. Keep direct writer buffer preconditions; use the Adobe observations to guide placeholder or metadata codec changes and confirm all planned Rust checks.
- [ ] 4.2 Verify existing PSD/PSB codec and save checks with both output choices, including cached writes and native depth. Validate the OpenSpec change and test-plan coverage. Run builds after the implementation is complete using the repository CPU budget.

## 5. Cross-application verification and closeout

- [ ] 5.1 Run both manual Photoshop exchange scenarios and record the exact Adobe builds and results in the PR checklist. Confirm editable layers and expected appearance across PSD and PSB with each merged-image choice.
- [x] 5.2 Record #639 in ROADMAP.md, sync the capability specs, and archive the change. CI will run the archive guard.
