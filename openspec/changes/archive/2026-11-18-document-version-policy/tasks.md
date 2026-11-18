# Tasks

## 1. Native codec tests and implementation

- [x] 1.1 Write `version_info_preserves_unicode_and_versions`, `version_info_handles_missing_or_unsupported_data`, `version_info_reports_written_merged_image`, and `versioned_write_preserves_unrelated_native_bytes` in pictura-codec. Use generated PSD/PSB bytes and check both wire data and emitted values.
- [x] 1.1a Add codec regression tests `ambiguous_version_info_refuses_versioned_write`, `incomplete_resource_section_blocks_versioned_write`, `exposes_icc_resource_by_id`, `preserves_unknown_blend_key_on_write`, `raw_write_preserves_resource_tail` for the corresponding test-plan rows before implementation.
- [x] 1.2 Add typed 1057 parsing and version-aware output to the shared codec. Verify layout, Unicode, malformed payloads, and composite flag cases from 1.1.
- [x] 1.3 Add the private resource, namespace-aware XMP mirror, access verdicts, and version-aware writes. Verify preservation of future accepted stamps, foreign same-ID blocks, unrelated XMP bytes, incomplete resource refusal, and raw-writer preservation of unparsed resource tails. Keep default raw codec writes covered by existing byte-preservation checks.

## 2. Application open, save, and File Info

- [x] 2.1 Add `native_save_stamps_document_versions` in pictura-app and register the Qt suite with `reportsUnsupportedNativeVersion`, `warnsWhenVersionMirrorCannotBeWritten`, `opensLegacyAndCurrentFilesForEditing`, `keepsCompatibleFutureStampOnSave`, `restrictsUnsupportedPrivateVersions`, `editsFileWithAdvisoryWriterMetadata`, `reportsUnsupportedPhotoshopFeatures`, and `showsVersionInfoInFileInfo`. Use generated test files and check open and save through the normal application paths.
- [x] 2.1a Add Qt regression cases `protectsSourceAcrossOutputPaths`, `reportsSourceDecodeFailure`, `restrictsAmbiguousVersionInfo`, `restrictsIncompleteResources` for the corresponding test-plan rows before implementation.
- [x] 2.2 Integrate compatibility diagnostics into eager and lazy native decoding at the existing feature interpretation points. Classify unknown critical layer/global records and dependency-sensitive edits conservatively, combining codec evidence with renderer support at the app boundary. Check known inert resources and supported variant controls alongside the unsupported cases in `reportsUnsupportedPhotoshopFeatures`.
- [x] 2.3 Thread compatibility results and native header errors through open completion. Install the per-document result on open and revert. Verify notice delivery, legacy opens, and Adobe feature-based access with the open cases from 2.1.
- [x] 2.4 Stamp native application output through the shared writer and propagate successful-save warnings through the worker. Verify `native_save_stamps_document_versions`, `warnsWhenVersionMirrorCannotBeWritten`, and `keepsCompatibleFutureStampOnSave`. Expose 1057 in File Info and verify `showsVersionInfoInFileInfo`.

## 3. Read-only viewing and copy

- [x] 3.1 Add Qt cases `readOnlyRejectsEditsAndSaves`, `usesOriginalCompositeForCompatibility`, `refreshesAccessOnReplacement`, `writesFlattenedCompatibilityCopy`, and `refusesUnsafeCompatibilityCopies`. Check content, history, source and destination bytes, and observable warnings.
- [x] 3.2 Handle failed or changed source-backed decoding with a reason and no destination write. Hold the validated source merged image for compatibility viewing and enforce read-only access across mutation and native save paths. Verify the guards, original pixels, missing-preview feedback, and document replacement cases from 3.1.
- [x] 3.3 Route confirmed flattened PNG copies through the existing atomic write, rejecting incompatible formats, missing source composites, and destination aliases in UI and path-based entry points; protect the source against every output path including control-server export_png. Verify both copy cases from 3.1 and rerun the existing editable Save a Copy tests.

## 4. Interchange and repository verification

- [ ] 4.1 Compile after code and test edits are complete, within the 80% CPU limit. Run the planned Rust and Qt tests and the existing PSD/PSB preservation, native save, and copy regressions.
- [ ] 4.2 Run the Photoshop reopen matrix in the test plan with locally generated, Photoshop-authored files kept outside tracked fixtures. Inspect 1057 creation and merged flags in both directions and check the private resource ID for collisions in the specimens. Record actual builds and results in the PR. Missing mandatory builds or cases block shipment unless a deliberate documented scope change revises the requirement.
- [ ] 4.3 Update the issue's roadmap entry when the change ships. Validate the OpenSpec change and test plan against the implemented suite. Archive the completed change with the PR and include the manual interoperability row as a reviewer checklist.
