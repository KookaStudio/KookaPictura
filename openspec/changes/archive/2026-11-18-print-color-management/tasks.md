# Tasks

Implementation checkboxes record source and test additions. The CMake app and
Qt test binaries built under the 80% CPU budget. CI owns subsequent builds,
automated test execution, and repository verification. Reviewers still need to
complete the Photoshop and physical printer checks in the test plan. The codec
uses the public field mappings in [print-resource-fields.md](print-resource-fields.md).

## 1. Prepare evidence and regression tests

- [ ] 1.1 Use locally generated scratch documents and public reference mappings to compare Photoshop CS6 and CC print resources across single-field edits. Record the descriptor classes, keys, enum values and units for the required portable fields, and capture the results for CMYK selection and proof simulation flags in a versioned field matrix alongside the implementation evidence.
- [x] 1.2 Add `pictura-codec::print_resources_round_trip_psd_and_psb` and `pictura-codec::opaque_print_resources_survive_save` using generated resource fixtures. Assert the edited field, the preserved unknown fields and bytes, and safe handling of malformed blocks.
- [x] 1.3 Add `pictura-codec::print_pdf_preserves_icc_device_samples`, `pictura-color::hard_proof_outputs_printer_device_samples`, `tst_print::applicationManagedPdf`, and `tst_print::invalidManagedProfileBlocksOutput`. Assert the PDF image channel count, selected ICC and reference samples, and rejection before submission for invalid profiles.
- [x] 1.4 Add `tst_print::previewAidsLeaveOutputSamplesAlone` and `tst_print::hardProofUsesLocalSetup`. Compare preview and output, and assert that the active canvas proof and image history are unaffected.
- [x] 1.5 Add `tst_print::outputMarksMatchPdfLayout` and `tst_print::outputMarksRespectPrintableBounds`. Check physical positions, description and label contents, and mark suppression under constrained printable bounds.
- [x] 1.6 Add `tst_print::printOneCopyUsesResolvedSettings`, `tst_print::acceptedDefaultsSurviveRestartAndReopen`, and `pictura-app::print_metadata_survives_undo_and_concurrent_save`. Check document and machine-local precedence, cancellation, job failure, and save-generation handling.
- [x] 1.7 Add `tst_print::hardProofModeAndSavedSetup`, `tst_print::managedCupsEligibility`, and `tst_print::pageSetupReachesOutput`. Exercise mode and saved proof precedence, eligibility and failure paths, PDF MediaBox and recorded native job options. Extend `tst_print::printSettingsPersistPageSetup` for the retained Qt path.

## 2. Persist portable print metadata

- [ ] 2.1 Read and patch the verified standard PSD/PSB print fields through the image-resource and descriptor codecs. Retain unknown keys, opaque data, and unrelated blocks, and refuse destructive edits to undecodable blocks. Verify with the two codec resource tests from section 1.
- [x] 2.2 Add document-session staging and dirty/save bookkeeping for accepted print metadata. Apply the staged settings to native save snapshots and Save a Copy. Verify that the session regression covers image undo and edits accepted during an in-flight save.
- [x] 2.3 Persist local last-job defaults through the existing app settings mechanism and overlay portable document values. Wire Done, successful Print and Print One Copy to the shared resolution and commit logic. Verify the isolated restart/reopen and one-copy tests.

## 3. Prepare print colour and proof output

- [x] 3.1 Resolve source and printer profiles, expose the colour handling, profile, intent and BPC controls, and build the managed device transform. Verify the reference conversion and confirm that unavailable or malformed profiles block output.
- [x] 3.2 Prepare the reduced preview and print-only proof override, including gamut and paper-white aids. Build the hard-proof raster for the target printer profile and validate it against the Little CMS reference and the two preview/proof UI cases.
- [x] 3.3 Encode the planned print page with the original device sample layout and embedded ICC profile. Verify the image data and profile in the emitted PDF, and preserve atomic file publication and the existing Save as PDF entry point.

## 4. Lay out and deliver the page

- [x] 4.1 Build shared physical page and mark geometry from the chosen page setup. Wire the Output controls and draw decorations in preview and output. Verify placement, metadata labels, bleed and suppression in the two mark tests.
- [x] 4.2 Submit the prepared PDF for supported application-managed CUPS destinations and carry the page and job options into the native route. Keep the existing printer-managed Qt path. Verify one-copy routing with the test submission edge and implement the explicit evidence-backed per-queue eligibility and invalidation rules.

## 5. Verify and close the change

- [ ] 5.1 Confirm the colour, codec, app and `tst_print` suites pass in CI. The local budgeted build and test-plan check are recorded in the PR.
- [ ] 5.2 Run the managed CUPS bypass and legacy physical tray checks, and the manual PSD and PSB round-trips in Photoshop CS6 and the selected CC build. Record exact versions, OS, profiles and the per-field results. Verify that the required settings survive File > Print and the return save in both versions.
- [ ] 5.3 Confirm the CI verification gates pass against the archived tree. Tick the #634 roadmap item after the manual PR checklist is accepted.
