# Test Plan

One row per spec scenario. `tst_*` references are Qt Test slots, `<crate>::<fn>`
references are Rust test functions, and `manual:` rows name the reason the check
cannot be automated.

## document/layer-comps

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `lc_store_empty` | Create or open a document without comps | Comp list empty; Last Document State unavailable | `pictura-core::layer_comp_store_starts_empty` |
| `lc_store_clone` | Clone a document with comps; add or rename a comp in the clone | Original comps unchanged; clone stores the edit | `pictura-core::layer_comp_store_is_shared_by_document_clones` |
| `lc_store_history` | Create a comp, edit the document, undo | Comp list matches its pre-edit state | `tst_layer_comps_panel::createIsUndoable` |
| `lc_record_visibility_only` | Create a visibility-only comp; move/show/hide/style a layer; apply | Position, blend, and style are unchanged | `pictura-render::visibility_only_comp_ignores_position_and_appearance` |
| `lc_record_all` | Create a comp with all three classes; change each; apply | Visibility, position, blend, and style all restore | `pictura-render::all_class_comp_restores_every_class` |
| `lc_record_smart_filter` | Apply different comps to a layer with a Smart Filter | Smart Filter settings and render stay the same | `pictura-render::smart_filter_settings_do_not_vary` |
| `lc_capture_visibility` | Capture a mixed-visibility tree; apply later | Captured combination returns exactly | `pictura-render::capture_records_current_visibility` |
| `lc_capture_cancel` | Open New Layer Comp and cancel | Comps, pixels, modified state, history unchanged | `tst_layer_comps_panel::createCancelChangesNothing` |
| `lc_capture_undo` | Create a comp; undo | Comp disappears; document otherwise unchanged | `tst_layer_comps_panel::createIsUndoable` |
| `lc_apply_visibility` | Create visibility comp; hide A/show B; apply | Original visibility returns; one history state | `pictura-render::apply_restores_visibility`, `tst_layer_comps_panel::applyRowAddsOneHistoryState` |
| `lc_apply_position` | Create position comp; move a layer; apply | Layer returns within one pixel | `pictura-render::apply_restores_position` |
| `lc_apply_appearance` | Create appearance comp; change blend/style; apply | Captured blend and style return | `pictura-render::apply_restores_appearance` |
| `lc_apply_unknown_blend` | Capture a layer with an unrecognized blend key; change it; apply | Raw blend key returns | `pictura-render::apply_restores_unknown_blend_key` |
| `lc_apply_unrecorded` | Apply a comp without Position after a move | Position is untouched | `pictura-render::apply_leaves_unrecorded_position` |
| `lc_apply_pixels` | Apply a visibility-only comp | Layer pixels unchanged; composite equals a fresh render | `pictura-render::apply_changes_no_pixels` |
| `lc_apply_empty` | Delete every captured layer; apply the comp | No document or Last Document State change; no history | `pictura-render::apply_with_no_resolvable_entries_is_a_noop`, `tst_layer_comps_panel::noopApplyLeavesHistoryAndLastState` |
| `lc_apply_repeat` | Apply and undo a position comp ten times | Same positions; no drift | `pictura-render::repeated_apply_and_undo_is_lossless` |
| `lc_update` | Change a recorded property; Update; apply later | New value restores; Update added one state | `pictura-render::update_recaptures`, `tst_layer_comps_panel::updateRecapturesComp` |
| `lc_update_warning` | Warned comp; Update | Warning gone; apply reports no missing layers | `pictura-render::update_clears_warning` |
| `lc_delete` | Delete a comp | Comp gone; one history state; others unchanged | `tst_layer_comps_panel::deleteRemovesComp`, `tst_layer_comps_panel::metadataEditsRecordOneState` |
| `lc_rename` | Rename a comp | Captured state and entries unchanged | `pictura-render::rename_keeps_captured_state`, `tst_layer_comps_panel::renameComp` |
| `lc_options` | Enable Appearance in Options; do not Update; Apply | Captured state unchanged until Update; Apply uses the new record set | `pictura-render::options_change_record_set_only`, `tst_layer_comps_panel::metadataEditsRecordOneState` |
| `lc_duplicate` | Duplicate a comp | New id, same comment/record/entries, next to the source | `pictura-render::duplicate_copies_captured_state`, `tst_layer_comps_panel::duplicateComp`, `tst_layer_comps_panel::metadataEditsRecordOneState` |
| `lc_comp_id_reuse` | Delete the last comp; create one; save/reopen | New id differs from every earlier id | `pictura-core::comp_ids_are_not_reused` |
| `lc_last_restore` | Apply a comp; apply Last Document State | Pre-apply layer state returns; one state added | `pictura-render::last_document_state_restores_pre_apply`, `tst_layer_comps_panel::lastStateRestoresPreApply` |
| `lc_last_undo` | Restore Last Document State; undo | Comp-produced state returns | `tst_layer_comps_panel::lastStateUndoReturnsToCompState` |
| `lc_last_unavailable` | No apply yet; try Last Document State | It cannot be applied | `tst_layer_comps_panel::lastStateDisabledBeforeApply` |
| `lc_last_noop` | Apply an all-missing comp | Last Document State is unchanged or stays unavailable | `pictura-render::a_noop_apply_does_not_become_last_document_state`, `tst_layer_comps_panel::noopApplyLeavesHistoryAndLastState` |
| `lc_warning_delete` | Capture a layer; delete it | Comp warns; other entries intact | `pictura-render::deleting_a_captured_layer_warns` |
| `lc_warning_merge` | Capture a layer; merge it away | Comp warns | `pictura-render::merging_a_captured_layer_warns` |
| `lc_warning_background` | Capture a layer; convert it to Background | Comp warns | `pictura-render::converting_a_captured_layer_to_background_warns` |
| `lc_warning_clear` | Clear a warning; apply the comp | Warning gone; resolvable entries still apply; one state | `pictura-render::clearing_a_warning_keeps_entries`, `tst_layer_comps_panel::clearWarning` |
| `lc_warning_new_loss` | Clear a warning; delete another captured layer; refresh | Warning appears again | `pictura-render::a_new_loss_warns_again` |
| `lc_warning_relost` | Clear a warning; undo the deletion; delete the same layer again | Warning appears again | `pictura-render::a_relost_id_warns_again` |
| `lc_warning_clear_all` | Two warned comps; Clear All | Neither warns; one history state | `pictura-render::clear_all_warnings`, `tst_layer_comps_panel::clearAllWarningsRecordsOneState` |
| `lc_warning_persist` | Save a warned comp; reopen | Same warning and cleared identities | `pictura-render::warning_state_survives_save_and_reopen`, `pictura-codec::warning_keys_survive_save_and_reopen` |
| `lc_id_rename` | Rename a captured layer; apply | Captured state applies; no warning | `pictura-render::renaming_a_layer_keeps_its_entry` |
| `lc_id_reorder` | Reorder captured layers; apply | Same layers receive their state | `pictura-render::reordering_layers_keeps_entries` |
| `lc_id_missing` | Create a comp on a layer without an identity | Non-colliding identity assigned; comp captures it | `pictura-core::layer_id_allocator_assigns_missing_ids` |
| `lc_id_duplicate` | Duplicate a captured layer; apply | Only the original is captured | `pictura-render::duplicating_a_layer_isolates_the_copy` |
| `lc_id_duplicate_save` | Duplicate a captured layer; save/reopen; apply | Copy is not captured; original still is | `pictura-render::duplicating_a_layer_does_not_persist_stale_comp_state`, `pictura-codec::duplicate_layer_state_is_not_persisted_as_captured` |
| `lc_id_psd` | Save with comps; reopen; apply | Comps resolve and apply without warnings | `pictura-render::comps_still_resolve_after_save_and_reopen` |
| `lc_group_position` | Capture a group; move it; apply | Descendants return once | `pictura-render::a_group_restores_its_children` |
| `lc_group_nested` | Capture nested groups; move the outer; apply | Every layer below moves once | `pictura-render::a_nested_group_moves_once` |
| `lc_adjustment_position` | Capture an adjustment with a mask; move; apply | Mask returns to the captured position | `pictura-render::an_adjustment_mask_moves` |
| `lc_shape_position` | Capture a shape; move; apply | Outline and rendered content return | `pictura-render::a_shape_outline_moves` |

## codec/psd-layer-comps

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `psdlc_read_list` | Open a PSD with three comps and captured info 1/3/7 | Three comps with the right names and classes | `pictura-codec::read_comp_list_from_resource` |
| `psdlc_read_absent` | Open a PSD without resource 1065 | Empty comp list | `pictura-codec::a_document_without_resource_1065_has_no_comps` |
| `psdlc_read_malformed` | Open a PSD with a truncated 1065 | Open succeeds; no comps; bytes preserved | `pictura-codec::malformed_comp_resource_opens_without_comps` |
| `psdlc_read_comment` | Open a comp with a recognized comment key | Comment is reported | `pictura-codec::comp_comment_is_read` |
| `psdlc_read_visibility` | Open a cmls variant with `enab = false` for comp A | Comp A captures the layer as hidden | `pictura-codec::read_layer_visibility_variant` |
| `psdlc_read_position_blend` | Open a variant with `FXRefPoint` and `blendOptions.Md` | Comp captures position and blend | `pictura-codec::read_layer_position_and_blend_variant` |
| `psdlc_read_base` | Open a variant whose `compList` is only `0` | No comp is created from it | `pictura-codec::base_variant_is_not_a_comp` |
| `psdlc_read_unknown_variant` | Open a variant for an unknown comp id | Open succeeds; bytes preserved on unedited save | `pictura-codec::unknown_comp_variant_is_preserved` |
| `psdlc_read_anchor` | Open a layer whose `origFXRefPoint` differs from `fxrp` | In-memory anchor becomes `origFXRefPoint` | `pictura-codec::read_orig_fx_ref_point_into_fxrp` |
| `psdlc_write_list` | Write two comps; read back | Names, ids, classes, comments match | `pictura-codec::comps_round_trip_through_resource_1065` |
| `psdlc_write_empty` | Delete the last comp; save | No 1065 and no cmls entries; other shmd entries remain | `pictura-codec::deleting_all_comps_removes_resource` |
| `psdlc_write_untouched` | Open and save without changing comps | 1065 bytes equal the input | `pictura-codec::unedited_comp_resource_is_byte_identical` |
| `psdlc_write_unknown` | Keep an unknown key in 1065 and save unedited | Key and bytes preserved | `pictura-codec::unknown_comp_keys_survive_an_unedited_save` |
| `psdlc_write_resource_chain` | Change a path or ICC resource with comps present | 1065 still present and unchanged | `pictura-codec::path_or_icc_rewrite_keeps_comps` |
| `psdlc_write_layer` | Write a visibility+appearance comp; read back | Same layers and values captured | `pictura-codec::layer_comp_state_round_trips` |
| `psdlc_write_position` | Write a position comp; reopen and apply | Captured positions restore | `pictura-codec::position_comp_round_trips` |
| `psdlc_write_vector_mask` | Write a vector-mask position comp; reopen and apply | Vector-mask offset and outline restore | `pictura-codec::vector_mask_position_round_trips` |
| `psdlc_write_nonparticipant` | Save a non-dirty-empty layer captured by no comp | shmd byte-identical | `pictura-codec::non_participating_layer_keeps_shmd` |
| `psdlc_write_dirty_empty` | Save a dirty-empty store | Every cmls gone; other shmd entries remain | `pictura-codec::dirty_empty_store_removes_all_cmls` |
| `psdlc_write_timestamp` | Rewrite comps on a layer with a timestamp entry | Timestamp entry survives | `pictura-codec::timestamp_entry_survives` |
| `psdlc_write_unknown_variant` | Rewrite a variant with an unknown key and matching compList | Unknown key survives | `pictura-codec::unknown_variant_keys_survive_a_dirty_rewrite` |
| `psdlc_id_roundtrip` | Write and read a document with lyid blocks | Every layer keeps its identity | `pictura-codec::lyid_values_round_trip` |
| `psdlc_id_write_assigned` | Assign an identity and save | Layer carries a lyid block | `pictura-codec::assigned_layer_ids_are_written` |
| `psdlc_id_seed` | Save with a new identity above the 1044 seed | 1044 is at least that identity | `pictura-codec::resource_1044_seed_is_advanced` |
| `psdlc_id_compfree` | Open/save a comp-free PSD with no lyid | Byte-identical output | `pictura-codec::comp_free_file_without_lyid_is_unchanged` |
| `psdlc_id_psb` | Write and read a PSB with comps | Comps resolve without warnings | `pictura-codec::psb_layer_comps_round_trip` |
| `psdlc_id_duplicate` | Duplicate a captured layer; save/reopen | Copy has no captured entry; original does | `pictura-codec::duplicate_layer_state_is_not_persisted_as_captured` |
| `psdlc_base_variant_refresh` | Apply a comp; save without comp edits | compID-0 base variant reflects the applied state | `pictura-codec::base_variant_refreshes_after_apply` |
| `psdlc_warning_persist` | Save/reopen a warned comp with cleared ids | Warning and clears survive | `pictura-codec::warning_keys_survive_save_and_reopen` |
| `psdlc_malformed_cmls` | Open a layer with a truncated cmls | Open succeeds; layer intact; block preserved | `pictura-codec::truncated_cmls_does_not_fail_open` |
| `psdlc_unknown_type` | Open a descriptor with an unmodeled value type | Open succeeds; descriptor preserved | `pictura-codec::unmodeled_descriptor_type_is_preserved` |
| `psdlc_preserve_failed` | Save after a failed comp parse without comp edits | Original resource and layer blocks unchanged | `pictura-codec::failed_comp_parse_preserves_bytes` |

## ui/layer-comps-panel

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `lcp_dock_hidden` | Start a fresh session | Panel registered but not visible | `tst_layer_comps_panel::panelStartsHidden` |
| `lcp_dock_toggle` | Invoke Window > Panels > Layer Comps twice | Checkable item; panel shows then hides | `tst_layer_comps_panel::windowMenuTogglesPanel`, `tst_layer_comps_panel::toggleIsCheckable` |
| `lcp_dock_restore` | Move the panel to a group; restart | Panel returns in that group with visibility | `tst_layer_comps_panel::panelRestoresLayout` |
| `lcp_dock_unchanged` | Show and hide the panel | Document, history, modified state unchanged | `tst_layer_comps_panel::togglingLeavesDocumentUnchanged` |
| `lcp_rows_order` | Document with comps A, B, C | Rows are Last Document State, A, B, C | `tst_layer_comps_panel::rowsFollowCompOrder` |
| `lcp_rows_empty` | Document with no comps | Only Last Document State | `tst_layer_comps_panel::emptyDocumentShowsLastStateOnly` |
| `lcp_rows_multi` | Shift-click and Ctrl-click rows | Contiguous and discontiguous selections | `tst_layer_comps_panel::multiSelection` |
| `lcp_rows_switch` | Switch the active document | List shows the new comps; old selection gone | `tst_layer_comps_panel::documentSwitchRefreshes` |
| `lcp_rows_refresh` | Delete a captured layer while the panel is open | Affected rows show the caution affordance | `tst_layer_comps_panel::layerDeleteShowsWarning` |
| `lcp_create_defaults` | Open the New Layer Comp dialog | Visibility on; Position and Appearance off | `tst_layer_comps_panel::createDialogDefaults` |
| `lcp_create_remember` | Create with Position; reopen the dialog | Position still checked | `tst_layer_comps_panel::createRemembersChoices` |
| `lcp_create_alt` | Alt-click Create | Comp created with defaults; no dialog | `tst_layer_comps_panel::altClickCreatesWithoutDialog` |
| `lcp_create_cancel` | Cancel the dialog | No comp; document unchanged | `tst_layer_comps_panel::createCancelChangesNothing` |
| `lcp_apply_row` | Click a row's apply affordance | Comp applied; one history state | `tst_layer_comps_panel::applyRowAddsOneHistoryState` |
| `lcp_apply_cycle` | Apply B; press Next repeatedly | C applied; Next then unavailable | `tst_layer_comps_panel::previousNextCycles` |
| `lcp_apply_selected` | Select A and C; Next from A | C applied; B skipped | `tst_layer_comps_panel::selectedCompsCycle` |
| `lcp_apply_layers` | Apply a recorded visibility/blend comp | Layers panel shows restored eye/blend | manual: verify the Layers panel reflects the applied comp in the app |
| `lcp_last_disabled` | No apply yet | Last Document State cannot be applied | `tst_layer_comps_panel::lastStateDisabledBeforeApply` |
| `lcp_last_apply` | Apply a comp; apply Last Document State | Pre-comp state returns; one state added | `tst_layer_comps_panel::lastStateRestoresPreApply` |
| `lcp_last_readonly` | Select Last Document State; invoke edit actions | Actions disabled or refused; document unchanged | `tst_layer_comps_panel::lastStateCannotBeEdited` |
| `lcp_edit_update` | Change a recorded value; Update | Comp reflects current state; one state | `tst_layer_comps_panel::updateRecapturesComp` |
| `lcp_edit_delete` | Delete a comp | Comp removed; one state | `tst_layer_comps_panel::deleteRemovesComp` |
| `lcp_edit_rename` | Double-click the name; type; commit | Rename applied; one state | `tst_layer_comps_panel::renameComp` |
| `lcp_edit_options` | Double-click outside the name | Options dialog opens with the comp's fields | `tst_layer_comps_panel::optionsDialogOpens` |
| `lcp_edit_duplicate` | Duplicate a comp | New comp with same state and a new id | `tst_layer_comps_panel::duplicateComp` |
| `lcp_edit_empty_name` | Clear the name; commit | Old name retained; no state | `tst_layer_comps_panel::emptyNameIsRefused` |
| `lcp_edit_history` | Confirm Options, Duplicate, and Delete-key | Each adds exactly one state and changes no pixels | `tst_layer_comps_panel::metadataEditsRecordOneState` |
| `lcp_menu` | Open the panel menu | Seven listed actions are present | `tst_layer_comps_panel::panelMenuActions` |
| `lcp_menu_enabled` | No comp selected | Update, rename, duplicate, delete disabled; New enabled | `tst_layer_comps_panel::menuActionsFollowSelection` |
| `lcp_warn_icon` | Delete a captured layer | Caution affordance appears | `tst_layer_comps_panel::cautionIconAppears` |
| `lcp_warn_clear` | Clear a comp warning | Affordance gone; captured state remains; one state | `tst_layer_comps_panel::clearWarning` |
| `lcp_warn_clear_all` | Clear All for multiple comps | All warnings gone; one state | `tst_layer_comps_panel::clearAllWarningsRecordsOneState` |
| `lcp_warn_apply` | Apply a comp with one missing and one present entry | Present layer restores; missing skipped | `tst_layer_comps_panel::applyingWarnedComp` |
| `lcp_gate_none` | No document open | Empty panel; actions disabled | `tst_layer_comps_panel::noDocumentDisablesActions` |
| `lcp_gate_transform` | Start a transform; apply a comp | Apply refused; session and document unchanged | manual: start a transform and confirm the panel refuses Apply |
| `lcp_gate_close` | Close or replace the active document during refresh | Panel clears rows; old document untouched | manual: close or replace a document with the panel open and confirm no write |

## interop/layer-comps-export

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `lce_dialog` | Choose Layer Comps to Files on an open document | Dialog shows the controls and buttons | manual: inspect the modal Layer Comps to Files dialog |
| `lce_dialog_defaults` | Open the dialog on poster.psd | Folder, prefix, PSD, ICC off | manual: inspect the modal Layer Comps to Files defaults |
| `lce_dialog_icc` | Choose BMP, then PSD | ICC disabled for BMP; enabled for PSD | manual: inspect the modal Layer Comps to Files ICC control |
| `lce_dialog_cancel` | Cancel the dialog | No file; document unchanged | manual: cancel the modal Layer Comps to Files dialog |
| `lce_dialog_no_document` | No document | Command disabled | `tst_layer_comps_export::commandNeedsADocument` |
| `lce_one_per_comp` | Export a three-comp document | Three files | `tst_layer_comps_export::filesWritesOnePerComp` |
| `lce_comp_content` | Export two visibility-different comps | Each file shows its own comp | `tst_layer_comps_export::filesRenderEachComp` |
| `lce_document_unchanged` | Complete a Files export | Document states, pixels, history, path, dirty unchanged | `tst_layer_comps_export::filesLeaveDocumentUnchanged` |
| `lce_icc` | Export with ICC on and Adobe RGB working profile | PSD/JPEG/TIFF/PDF carry the profile | `tst_layer_comps_export::filesIncludeIcc` |
| `lce_files_empty` | Export a document with no comps | Nothing written; message shown | `tst_layer_comps_export::filesEmptyWritesNothing` |
| `lce_session_gate` | Start a transform; request a comps export | Export refused or settled; no mid-session state | manual: start a transform and confirm comps export refuses |
| `lce_name_sanitize` | Prefix Export, comp a/b:c? | File Exporta_b_c_.tif | `tst_layer_comps_export::namesAreSanitized` |
| `lce_name_collision` | Two comps sanitize alike | Second gets ` 2`; no overwrite | `tst_layer_comps_export::nameCollisionsGetSuffix` |
| `lce_name_empty` | Empty prefix and comp name | Name starts with Layer Comp | `tst_layer_comps_export::emptyNameFallsBack` |
| `lce_name_long` | Name over 255 bytes | Prefix preserved; comp name truncated | `tst_layer_comps_export::longNamesAreTruncated` |
| `lce_name_long_prefix` | Prefix alone over the cap | Prefix truncated first; suffix fits | `tst_layer_comps_export::longPrefixIsHandled` |
| `lce_pdf_pages` | Export three comps to PDF | Three pages | `tst_layer_comps_export::pdfOnePagePerComp` |
| `lce_pdf_size` | 600x300 at 300 ppi | MediaBox 144x72 points | `pictura-codec::multi_page_pdf_keeps_page_size` |
| `lce_pdf_content` | Render two visibility-different comps | Each page matches its comp | manual: render the comps PDF and compare each page to its comp |
| `lce_pdf_cancel` | Cancel the PDF destination dialog | No PDF; document unchanged | manual: cancel the PDF destination dialog |
| `lce_pdf_empty` | PDF export with no comps | No file; clear message | `tst_layer_comps_export::pdfEmptyWritesNothing` |
| `lce_cancel` | Cancel a multi-comp Files export | Stops before the next file; document unchanged | manual: cancel a multi-comp export and confirm it stops |
| `lce_bad_destination` | Bad destination | No file; failure reported | `tst_layer_comps_export::badDestinationWritesNothing` |
| `lce_directory_target` | A computed path is a directory | Batch stops; reports comp and path | `tst_layer_comps_export::directoryTargetStopsBatch` |
| `lce_replace` | The computed file already exists | File replaced with the comp output | `tst_layer_comps_export::filesReplaceExistingFile` |
| `lce_write_failure` | A write fails partway | Batch stops; reports comp and path | manual: force a write failure and confirm the batch stops |
| `lce_no_history` | Finish or cancel either export | History depth and dirty state unchanged | `tst_layer_comps_export::exportLeavesHistoryAndDirtyUnchanged` |

## Manual checks

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| Photoshop-authored comps round-trip | Open a Photoshop-authored PSD with comps, apply each, save, reopen in Photoshop, apply each | Names/classes match; visibility applies; position and appearance apply within the inferred-key limits; comps remain editable in Photoshop | manual: requires a Photoshop-authored sample and a Photoshop installation; the committed fixtures are generated by the project |
| Photoshop ignores project keys | Save a document with comp comments and cleared warnings, open it in Photoshop, edit and save a comp | Comps remain editable; the project-owned comment/warning keys are ignored without losing the standard comp data | manual: requires Photoshop to verify tolerance of project-owned descriptor keys |
