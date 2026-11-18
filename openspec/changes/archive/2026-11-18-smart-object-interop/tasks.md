# Tasks

## 1. Embedded vector placement

- [x] 1.1 Add `tst_vector_smart_object::placesSelectedPage`, `cancelledOrInvalidPlace`, `scaleRendersFromSource`, `sourceRoundTrips`, and `sourceIdentityRoundTrips` using synthetic PDF and PDF-compatible AI artwork with distinct pages and transparency. Verify the cases cover place, scale, cancel, PSD/PSB reopen, and byte-exact export.
- [x] 1.2 Prepare and persist an embedded vector source with its page, crop, filename, and transparent output through the existing PDF backend and embedded PSD/PSB authoring. Verify source bytes and display pixels with `sourceRoundTrips` and the existing independent PSD reader oracle.
- [x] 1.3 Wire vector inputs through File > Place, page selection, fit, and transform commit. Render at the committed size under the existing worker and output budgets. Verify normal placement, cancellation, failure, and resize undo with the placement cases in `tst_vector_smart_object`.

## 2. External Edit Contents

- [x] 2.1 Add `tst_smart_object_actions::editContentsDispatchesBySourceKind` and extend `pictura-app::layer_can_edit_smart_object_contents_eligibility` with PDF, PDF-compatible AI, malformed vector, and PSB fixtures. Verify the test exercises both eligible routes and the original refusal cases.
- [x] 2.2 Add `tst_vector_smart_object::launchesExternalEditor`, `appliesToSharedInstances`, `refusesInvalidOrStaleEdits`, and `discardsExternalSession`. Use an injected desktop launch result and fixture files to exercise success, refusal, stale sessions, history, and cleanup offscreen.
- [x] 2.3 Extend Edit Contents routing for vector sources and implement the temporary-file handoff and Apply/Discard session. Verify launch, cleanup, eligibility, and the existing native tab path with the tests in 2.1 and 2.2.
- [x] 2.4 Give newly placed vector sources stable identities, preserve them across ordinary duplication and PSD/PSB save/reopen, and allocate a distinct identity for New Smart Object via Copy. Make the writer group authored vector records by identity rather than filename or payload equality. Stage and apply all shared-instance pixels and source bytes in one edit. Verify reordered layers, independent copies, repeated unchanged Apply, and Undo/Redo with the external edit cases.

## 3. Vector Paste As

- [x] 3.1 Add synthetic PDF-only clipboard fixtures and exercise the actual Edit > Paste action with no internal clip or system image. Keep Paths and Shape Layer disabled; pursue editable Illustrator data separately only after a redistributable fixture and supported representation are verified.
- [x] 3.2 Add `tst_vector_paste::offersSupportedDestinations`, `regularPasteIsUnchanged`, `createsLayerDestinations`, `cancelledOrInvalidPaste`. Exercise the Paste action with clipboard fixtures and inspect enabled choices, resulting layers, and history.
- [x] 3.3 Route supported clipboard vectors into Paste As, stage its destinations via vector placement and raster import, and preserve the normal paste route for raster and native clips. Enable Edit > Paste for PDF-only MIME without changing other paste command gates. Verify action enablement, advertised choices and committed results with `tst_vector_paste`.

## 4. Imported external link repair

- [x] 4.1 Collect external linked PSD/PSB fixtures with provenance that cover shared and independent records and an opaque variant. Add `pictura-codec::repaired_external_record_roundtrips_selectively` and `unsupported_external_record_stays_opaque`. Verify the tests compare record locations, UUIDs, preserved fields, and unrelated bytes.
- [x] 4.2 Add `tst_smart_object_relink::opensWithStoredPreview`, `repairsSharedSource`, `refusesInvalidOrStaleRepair`, `refusesIncompatibleRepair`, `repairedLinkSurvivesReopen`, and `rowActionTargetsClickedSource`. Verify coverage for unavailable assets, repair eligibility, cancellation and failure, clicked-row targeting, single-step Undo/Redo, and reopen.
- [x] 4.3 Expose supported external locations at the codec boundary and selectively rewrite repaired records while preserving unknown bytes and untouched records. Keep open independent of the recorded path. Verify the PSD and PSB record tests from 4.1 and inspect the written location and UUID with an independent PSD reader.
- [x] 4.4 Refuse format-family, dimension, page, or crop mismatches; derive original dimensions and page from recognized placed descriptor fields without reading the old file; refuse missing, inconsistent, or ambiguous fields. Update location, filename, file type, file size, and recognized timestamps on compatible repair; update understood cached external data or refuse the variant. Stage previews from explicitly chosen compatible replacements, then update the record and all affected layers in one history transaction. Wire the Layers row command and update the existing embedded-row assertion to cover its disabled repair action. Verify the relink command and preview tests from 4.2.

## 5. Review fixes

- [x] 5.1 Fix cancellation when an external Edit Contents session closes during Apply or a placement transform is discarded through the tool-switch prompt. Cover both paths in Qt Test.
- [x] 5.2 Render enlarged vector artwork at the committed sampling resolution after rotation or warp. Write supported placement geometry and crop into the standard placed descriptor and extend the independent PSD reader coverage.
- [x] 5.3 Preserve local geometry and visible Smart Filters when rebuilding external-link previews. Cover repair after transformation and repair of a masked Smart Filter.

## 6. Integration and archive

- [x] 6.1 Compile the app and Qt test executables within the repository CPU budget and validate the change and main specs with OpenSpec. CI owns test execution, scenario mapping checks, and repository verification gates.
- [x] 6.2 Record the shipped #638 items and follow-ups in ROADMAP.md, sync the main specs, and archive the change. Carry the CI verification handoff into the pull request.
