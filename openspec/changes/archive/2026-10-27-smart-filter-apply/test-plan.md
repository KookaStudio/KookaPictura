# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `sfa_append` | Select an embedded Smart Object, apply Filter > Blur > Gaussian Blur twice | The Smart Filters row lists two entries, the proxy is unchanged, the canvas is blurred | `pictura-render::apply_appends_and_keeps_proxy`, `tst_smart_filter_apply::filterMenuAppendsSmartFilter` |
| `sfa_last_filter` | Apply a filter to a Smart Object, then Filter > Last Filter | Two entries | `pictura-app::last_filter_appends` |
| `sfa_refuse` | Apply a filter to a linked Smart Object, a legacy placed layer, a raster layer through the smart op; and destructively to a Smart Object | Each is refused, the layer unchanged | `pictura-render::refuses_ineligible`, `pictura-app::pixel_target_refuses_smart_object` |
| `sfa_mask_from_selection` | Select the left half, apply a filter to a Smart Object | A filter mask appears, white on the left; only the left is filtered | `pictura-render::selection_becomes_mask`, `tst_smart_filter_apply::selectionBecomesFilterMask` |
| `sfa_mask_kept` | Apply with a selection to an object that already has a mask | The mask is unchanged | `pictura-render::existing_mask_kept` |
| `sfa_no_selection` | Apply with no selection | No mask | `pictura-render::no_selection_no_mask` |
| `sfa_record_round_trip` | Encode a stack record and decode it | Kinds, slots, gallery flag unchanged | `pictura-codec::steps_round_trip`, `pictura-codec::gallery_flag_round_trips` |
| `sfa_record_malformed` | Decode empty, step-less, and wrongly typed records | `None` each | `pictura-codec::malformed_options_decode_none` |
| `sfa_render_matches` | Render a Gaussian Blur record over a buffer with transparency | Equal to the destructive blur | `pictura-render::generic_record_matches_destructive_apply` |
| `sfa_gallery_order` | Render a two-step gallery record | Equal to the two destructive filters in order | `pictura-render::gallery_steps_apply_in_order` |
| `sfa_excluded_undecodable` | Decode a record naming Liquify | Undecodable | `pictura-render::excluded_kind_is_undecodable` |
| `sfa_gallery_one_entry` | Commit a two-effect Filter Gallery stack on a Smart Object | One "Filter Gallery" entry with both steps | `pictura-app::gallery_commits_one_entry`, `tst_smart_filter_apply::filterGalleryAddsOneEntry` |
| `sfa_excluded_disabled` | Select a Smart Object, open the Filter menu | Liquify, Vanishing Point, Lens Blur, Displace, Adaptive Wide Angle are disabled; Gaussian Blur enabled | `pictura-filters::smart_capable_excludes_cs6_list`, `pictura-filters::resource_kinds_are_not_smart_capable`, `tst_smart_filter_apply::excludedFiltersDisabledOnSmartObject` |
| `sfa_adjustments_disabled` | Select a Smart Object, open Image > Adjustments | Entries disabled; enabled on a raster layer | `tst_smart_filter_apply::adjustmentsDisabledOnSmartObject` |
| `sfa_preview_cancel` | Open a filter dialog on a Smart Object, change a value, Cancel | The layer and its Smart Filters are as before | `pictura-app::preview_cancel_restores_layer`, `tst_smart_filter_apply::previewCancelRestoresSmartObject` |
| `sfa_preview_commit` | Preview twice, then OK | Exactly one new entry | `pictura-app::preview_then_commit_appends_once` |
| `sfa_undo` | Apply a filter to a Smart Object, Edit > Undo | No Smart Filter | `tst_smart_filter_apply::undoRemovesSmartFilter` |
| `sfa_no_reedit` | Double-click a generic Smart Filter entry | Nothing opens | `tst_smart_filter_apply::editSmartFilterDisabledForGenericEntry` |
| `sfa_save_reopen` | Apply two filters with a selection, save, reopen | Same entries, mask, and pixels | `pictura-render::save_reopen_keeps_filters_and_mask` |
| `sfa_photoshop` | Save a generic Smart Filter with a mask, open in Photoshop | The mask shows on the Smart Filters row | manual: needs a Photoshop install; the generic record is Pictura-only and the authored FEid is unverified |
| `sfm_author` | Write a masked smart object with no FEid, read back | Same mask rect and coverage | `pictura-codec::authored_mask_reads_back` |
| `sfm_author_unchanged` | Write a document read with an unedited mask | FEid and FMsk bytes unchanged | `pictura-codec::unchanged_mask_bytes_preserved` |
| `sfm_author_changed` | Edit one of two masks, write | Only that entry changes | `pictura-codec::changed_mask_replaces_entry_only` |
| `sfm_author_converted` | Write a converted smart object with a mask | Keyed by the authored uuid; attaches on read | `pictura-codec::converted_object_keyed_by_authored_uuid` |
| `sfm_author_psb` | Write as PSB and read back | The mask attaches | `pictura-codec::authored_mask_round_trips_psb` |
| `sfm_author_flag` | Set filterMaskEnable on a preserved descriptor, write, read | Typed and preserved flags agree | `pictura-codec::filter_mask_enable_flag_round_trips` |
| `sfm_author_photoshop` | Open a document with an authored mask in Photoshop | The mask shows | manual: needs a Photoshop install to validate the authored FEid/FMsk |
| Attaching into a preserved descriptor | Attach a filter over a preserved SoLd | The list gains the entry; typed view matches | `pictura-codec::attach_inserts_into_a_preserved_sold_and_preserves_other_keys` |
| Re-attaching over a modelled filter keeps unmodeled keys | Re-attach the same id | Not duplicated; unmodeled keys survive | `pictura-codec::reattach_over_a_preserved_filter_keeps_unmodeled_keys` |
| Attaching to an embedded object without a preserved block | Attach to a converted object, write, read | Authored on save | `pictura-codec::attach_authors_then_write_read_round_trips` |
| Non-smart or legacy layers are refused | Attach to a raster layer | Refused, unchanged | `pictura-codec::attach_errors_without_a_smart_object` |
| `psf_append` | Append two same-id filters, with and without a preserved block | Both kept; lists aligned | `pictura-codec::append_stacks_same_id_preserved`, `pictura-codec::append_stacks_same_id_typed` |
| Manual smoke | In `./build/pictura`, Gaussian Blur twice on a Smart Object, toggle, reorder, delete, apply with a selection, save, reopen | Two rows, source unchanged, the mask and result survive reopen | manual: end-to-end visual check across panel, canvas, and file dialogs |
