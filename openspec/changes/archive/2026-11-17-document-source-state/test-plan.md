# Test Plan

This is an implementation plan, not a record of executed checks. Tests marked
**new** below must be written before implementation tasks; existing checks retain
their expected values and migrate only their state access. All rows are automated;
there are no manual reviewer checklist items. The standalone `/test-plan` helper
is not installed in this checkout/session, so this table follows the repository's
documented format directly.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `dss_legal_states` | Construct every source row in design.md with the supported composite/output combinations; compare core and render mode/depth queries and native authority. | Every supported case succeeds and produces its established predicate answers. | `pictura-core::source_state_legal_cases`, `pictura-render::stored_composite_queries_agree_with_document` |
| `dss_invalid_states` | Attempt Indexed without palette, non-Indexed with palette, mismatched declared/sample depths, and sample replacements with different type or length. | Checked construction rejects each contradiction and does not mutate the document; sample replacement preserves metadata. | `pictura-core::source_state_rejects_invalid_combinations`, `pictura-core::sample_replacement_preserves_metadata_and_rejects_shape_or_type_changes` |
| `dss_recorded_depth` | Record depth Sixteen with no document/layer native samples; inspect effective and retained depth. | Provenance survives; effective depth is Eight and retained-depth is false. | `pictura-core::source_state_legal_cases`, `pictura-core::retains_source_depth_tracks_the_retained_store` |
| `dss_layer_only_depth` | Put native Sixteen/ThirtyTwo channels in a nested descendant, omit the document store, then request matching planes with matching recorded depth and no converted mode; also vary recorded depth and source mode. Convert a layer-only Sixteen document without a composite to ThirtyTwo. | Retained-depth is true independent of provenance; authority follows recorded depth and mode; conversion preserves native layer precision. | `pictura-core::source_queries_include_descendant_stores`, `pictura-render::layer_only_native_depth_converts_with_absent_composite` |
| `dss_native_authority` | Vary recorded depth (including absent and mismatched), normalized source mode, rectangle, native depth, and requested channel independently, including a matching control. | Converted/mismatched/missing planes fall back; the matching working-mode control returns its native plane. | `pictura-core::native_plane_authority_preserves_fallbacks` |
| `dss_stored_composite` | Inspect matching nonempty, absent, empty, and canvas-mismatched buffers through core and render forwarders. | All consumers agree on true/false and empty buffers never count as stored. | `pictura-render::stored_composite_queries_agree_with_document` |
| `dss_composite_roles` | Inspect Grayscale one-plane and RGB four-plane buffers; refresh an ordinary layered RGB/8 document. | Storage, shape, and retention answers remain distinct; ordinary layered RGB stays on demand. | `pictura-core::composite_roles_and_output_policy_are_independent`, `pictura-render::a_layered_rgb_full_refresh_stores_nothing`, `pictura-render::a_non_rgb_or_non_four_channel_composite_is_not_shape_valid` |
| `dss_merged_output` | Start with an absent composite and output enabled; exercise the application picture-save path and read the PSD back. Exercise output disabled with a maintained buffer, skipped-merged open, and direct codec rejection of an absent buffer. | The application save writes rendered merged pixels when enabled and omits image data when disabled; skipped decode preserves the output choice; direct codec buffer errors remain unchanged. | `pictura-core::composite_roles_and_output_policy_are_independent`, `pictura-app::save_absent_composite_preserves_merged_output_policy`, `pictura-codec::a_layered_merged_image_is_skipped_on_request`, `pictura-codec::streamed_merged_image_is_byte_identical`, `pictura-codec::no_composite_document_writes_no_image_data_and_round_trips` |
| `dss_transition_atomic` | Clone a valid document; attempt invalid source replacement and internally inconsistent composite replacement. | Both refusals preserve document equality and the prior shared stores. | `pictura-core::state_replacement_refusal_is_atomic` |
| `dss_profile_transition` | Open the ICC fixture with Convert, clone, assign another working profile, and compare profile records, pixels, and allocations; edit unique and shared flat composites. | Original and working profiles coexist; pixels and untouched shared allocations are preserved; unique fixed-length edits retain their allocation and shared edits fork. | `pictura-codec::assigning_working_profile_preserves_converted_source`, `pictura-core::composite_fixed_slice_edits_keep_unique_allocation_and_fork_shared_data` |
| `dss_history_planes` | Hollow and restore a document with a shared composite and other tracked planes; inspect the detached metadata, slot order and allocations. | Geometry, channels, policy and slot order survive; restored unchanged planes share allocations. | `pictura-app::history_hollow_reattaches_shared_composite` |
| `dss_codec_compatibility` | Run unchanged read/write, depth, color-mode, profile and merged-image oracles, including PSD/PSB and all existing compression cases; round-trip packed Bitmaps with extras and inspect a missing merged image. | Existing bytes, headers, resources, native samples, placeholders, and golden expectations pass unchanged. | `pictura-codec::depth_documents_preserve_source_depth_on_save`, `pictura-codec::grouped_depth_fixtures_round_trip_whole`, `pictura-codec::sixteen_bit_cmyk_and_lab_save_back_in_source_mode_and_depth`, `pictura-codec::thirtytwo_bit_cmyk_and_lab_save_back_in_source_mode_and_depth`, `pictura-codec::assigned_profile_round_trips_through_saved_psd`, `pictura-codec::streamed_merged_image_is_byte_identical`, `pictura-codec::packed_bitmap_composite_keeps_native_and_extra_planes`, `pictura-codec::bitmap_without_merged_image_keeps_placeholder_and_output_policy` |
| Cloning a document shares native samples and payload | Clone a document with a retained native store and smart payload; compare values and allocation identity. | The clone equals the original and shares both stores. | `pictura-core::a_document_clone_shares_native_samples_and_payload` |
| A smart-object payload and its raw linked section share | Clone a document with payload and raw linked bytes; inspect both allocations. | Both stores remain shared and byte-identical. | `pictura-core::a_smart_object_payload_and_raw_section_share` |
| A layer clone shares its byte blocks and layer tree | Clone the comprehensive layer fixture and inspect blocks and children. | Blocks/children share until written; the original stays unchanged. | `pictura-core::a_layer_clone_shares_blocks_and_children` |
| Preserved document byte blocks share on clone | Construct both profile records plus preserved blocks, clone, and borrow both values. | Borrowed views retain the same allocations and original bytes. | `pictura-core::a_document_clone_shares_preserved_blocks` |
| A structural edit forks the tree | Clone a tree, add a node to one side, and compare the containers and original nodes. | Only the edited tree forks and the other stays unchanged. | `pictura-core::a_clone_shares_and_a_structural_edit_forks` |
| A written store forks alone | Write one shared sample or preserved store and inspect untouched stores. | Only the written store forks. | `pictura-core::a_written_store_forks_alone` |
| A freshly opened document is in the sRGB working space | Read untagged and sRGB-tagged documents through the existing policy tests using borrowed profile access. | No working ICC is exposed and stored pixels are unchanged. | `pictura-codec::policy_leaves_a_profile_less_file_unchanged`, `pictura-codec::policy_leaves_an_srgb_profile_unchanged` |
| A read-normalised document records the source separately | Read the non-sRGB ICC fixture with Convert and inspect both borrowed profile records. | Original ICC is present; working ICC is absent. | `pictura-codec::policy_convert_is_the_read_normalisation_and_drops_the_profile` |
| A preserved document carries the embedded profile | Read the ICC fixture with Preserve and compare against Off. | Working ICC equals embedded bytes, original ICC is absent, and pixels are unchanged. | `pictura-codec::policy_preserve_keeps_the_embedded_profile_and_pixels` |

New tests: `source_state_legal_cases`, `source_state_rejects_invalid_combinations`,
`source_queries_include_descendant_stores`,
`native_plane_authority_preserves_fallbacks`,
`composite_roles_and_output_policy_are_independent`,
`state_replacement_refusal_is_atomic` (core),
`stored_composite_queries_agree_with_document` (render), and
`assigning_working_profile_preserves_converted_source` (codec),
`save_absent_composite_preserves_merged_output_policy` and
`history_hollow_reattaches_shared_composite` (app).

The ten initial tests and five review regressions are written, with the core state cases in a dedicated module.
The CMake app and Qt test build passed on 2026-11-17 under the 80% CPU budget
(12 of 16 allowed CPUs), after migrating one stale PDF field initializer.
The user subsequently authorized archival, commits, and a PR with remaining
checks delegated to CI. All spec deltas are synced and this change is archived;
strict OpenSpec validation and the archive guard passed. Test execution and the
remaining verification gates are delegated to CI. No passing test result is
claimed here, and no build was rerun during archival. There are no manual rows.

CI must run the focused core/render/codec checks, the full named oracle binaries,
and the required workspace gates. Ensure the independent tools are present when
claiming oracle verification; a self-skip is not proof of compatibility.
