# Tasks

## 1. Fixtures

- [x] 1.1 Extend `scripts/generate-fixtures.py` with the GPS IFD, Origin/Keywords IIM records, and XMP Description, Origin, IPTC Extension, Photoshop, `xmp:`, and `xmpMM:` properties; regenerate `metadata.psd` twice and verify byte-identical output.

## 2. Codec foundations

- [x] 2.1 Split `crates/pictura-codec/src/xmp.rs` into `xmp/mod.rs`, `xmp/scan.rs`, `xmp/patch.rs`, and `xmp/serialize.rs` with pure moves; verify `cargo nextest run -p pictura-codec` is unchanged when the build gate runs.
- [x] 2.2 Add the eight new managed properties (`City`, `State`, `Country`, `Instructions`, `AuthorsPosition`, `CaptionWriter`, `WebStatement`, `UsageTerms`) to the typed parser, patcher, and serializer; verify `metadata_description_fields_parse` and `template_serializes_the_expanded_managed_set` pass.
- [x] 2.3 Add `Iptc::values`/`set_values` for repeated datasets; verify `iptc_list_values_round_trip` passes.
- [x] 2.4 Expand the shared IIM↔XMP set to the fourteen fields in design D6 and add `set_file_info_values` for list fields; verify `shared_field_writes_both_channels`, `metadata_keywords_sync_as_a_list`, `metadata_origin_fields_sync_both_channels`, and `metadata_unchanged_shared_field_is_a_noop` pass.
- [x] 2.5 Add `set_xmp_packet` with well-formedness and size validation, resource framing, tail preservation, and empty-removes; verify `metadata_raw_packet_replaces_the_resource`, `metadata_raw_packet_rejects_malformed`, and `metadata_raw_packet_empty_removes` pass.

## 3. Structured XMP

- [x] 3.1 Add the generic property-tree reader in `xmp/tree.rs` reusing the existing tokenizer and namespace scopes; verify `xmp_reads_attribute_and_element_forms`, `xmp_resolves_namespace_prefixes`, `xmp_reads_bag_and_struct_leaves`, and `xmp_malformed_packet_reads_what_it_can` pass.
- [x] 3.2 Add the path patcher with span replacement, bag/struct/lang-alt creation, local namespace declarations, removal, no-op matching, and fail-closed shapes; verify `xmp_patch_preserves_unknown_namespace_bytes`, `xmp_patch_creates_a_bag`, `xmp_patch_creates_a_struct_with_local_namespace`, `xmp_patch_fails_closed_on_unsupported_shape`, and `xmp_patch_removes_only_the_property` pass.

## 4. IPTC Extension

- [x] 4.1 Add `metadata/iptc_extension.rs` descriptors and read rows; verify `iptc_extension_reads_the_fixture_fields`, `iptc_extension_missing_namespace_is_empty`, and `iptc_extension_malformed_packet_does_not_panic` pass.
- [x] 4.2 Add the extension write path (simple, bag, lang-alt, struct, whole-apply pre-validation) and unknown-byte preservation; verify `iptc_extension_adds_a_person_bag`, `iptc_extension_edits_a_location_leaf`, `iptc_extension_clears_a_field`, `iptc_extension_preserves_unknown_extension_bytes`, and `iptc_extension_unsupported_shape_aborts` pass.
- [x] 4.3 Add the read-only artwork/PLUS and repeated-structure listing; verify `iptc_extension_lists_artwork_read_only` and `iptc_extension_lists_extra_structures_read_only` pass.

## 5. GPS

- [x] 5.1 Walk the GPS sub-IFD (`0x8825`) with typed latitude/longitude/altitude/timestamp/date/datum and the shared budget; verify `exif_gps_decodes_the_fixture_values`, `exif_gps_survives_a_truncated_ifd`, and `exif_gps_absent_without_the_pointer` pass.

## 6. Oracles and templates

- [x] 6.1 Extend `crates/pictura-codec/tests/metadata_oracle.rs` with the GPS, IPTC Origin/Keywords, and expanded XMP tags, including `parsed_xmp_agrees_with_exiftool`; verify `metadata_decodes_and_agrees_with_exiftool` and `parsed_xmp_agrees_with_exiftool` pass.
- [x] 6.2 Extend the extension and raw-packet oracle coverage; verify `parsed_extension_fields_agree_with_exiftool`, `edited_extension_fields_survive_save_and_exiftool`, and `raw_packet_round_trips_through_save` pass (self-skipping without `exiftool`).
- [x] 6.3 Extend the template coverage; verify `template_origin_field_reaches_both_channels` and `template_keyword_list_reaches_both_channels` pass.

## 7. Bridge

- [x] 7.1 Emit section-prefixed rows from `exif_rows`, `iptc_rows`, `xmp_rows`, and the renamed `file_info_edit_fields`; include GPS, Origin, IPTC Extension, Photoshop, and empty editable fields; verify `tst_file_info::categoriesInCs6Order` and `tst_file_info::gpsRowsShowDecodedValues` pass.
- [x] 7.2 Dispatch `apply_metadata_edits` in the order raw packet → shared fields → XMP-only → extension, pre-validate each group, and record one `Edit::Meta` only when something changed; verify `tst_file_info::rawAndFieldEditAreOneUndoState` and `tst_file_info::illFormedRawPacketKeepsDialogOpen` pass.

## 8. Dialog and frame

- [x] 8.1 Rebuild `file_info_dialog.{h,cpp}` around the eight sections, form/table/raw pages, shared-id mirroring, list editors, empty editable controls, `onApply`, the existing test hooks, and the Export/Apply Template controls with the merge-mode combo; verify the dialog compiles when the build gate runs.
- [x] 8.2 Wire `frame.cpp` to pass the concatenated rows, call `onApply` only when edits exist, and handle a refusal with a warning while keeping the dialog open; verify the open/accept/cancel paths by inspection until the build gate runs.
- [x] 8.3 Add `crates/pictura-app/cpp/tests/tst_file_info.cpp` and register it in `crates/pictura-app/cpp/tests/CMakeLists.txt`; verify the suite runs when the build gate runs.

## 9. Self-test migration

- [x] 9.1 Retire the file-info checks 456/457/459 and their fixture builder from `selftest_layers_adjustments.cpp`, migrate check 460's dialog construction and `xmp_rows()` assertions to `file_info_edit_fields()`/`xmp_packet()`, keep its template controls, and lower `scripts/selftest-budget.txt` from 506 to 503; verify `bash scripts/check-selftest-budget.sh` passes.

## 10. Documentation

- [x] 10.1 Reword the `ROADMAP.md` File Info item to the four in-scope tabs, point to "Not planned" for Video/Audio/DICOM, and update `docs/dev/doc-code-parity-gaps.md` WF-7 with the new evidence and status (the docs commit carries `TASK-ALLOWS-DOCS`); verify `bash scripts/guard.sh` passes.

## 11. Integration verification

- [ ] 11.1 Run `cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings`; verify both are clean.
- [ ] 11.2 Build, run `tst_file_info` and the neighbouring suites, then `bash scripts/verify-fast.sh`; verify the full gate passes.
- [x] 11.3 Run `python3 scripts/check-test-plan.py` and `python3 scripts/check-cmake-source-fragments.py`; verify both pass.

## 12. Archive

- [ ] 12.1 After task 11 passes, run `openspec validate --all --strict`, archive the change with `openspec archive file-info-tabs`, and verify `bash scripts/check-openspec-archived.sh` passes.
