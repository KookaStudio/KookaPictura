# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `sfm_read` | Read a document whose `FEid` carries a coverage channel for a smart object's uuid | The object's `filter_mask` holds the rect and decoded coverage | `pictura-codec::filter_mask_reads_from_the_feid_block`, `pictura-codec::a_mask_attaches_to_its_smart_object_by_uuid`, `pictura-codec::a_mask_attaches_by_the_placed_uuid` |
| `sfm_absent` | Read a document with no `FEid`/`FMsk` block | Every smart object's `filter_mask` is `None` | `pictura-codec::a_document_without_a_mask_has_none` |
| `sfm_roundtrip` | Read and write a document carrying a mask | The re-read mask rect and coverage equal the original's | `pictura-codec::a_mask_survives_a_document_round_trip` |
| `sfm_delete` | Remove a mask and re-read the written document | The mask is `None` and no other block changes | `pictura-codec::deleting_a_filter_mask_drops_the_blocks` |
| `sfm_delete_absent` | Remove a mask for a uuid no entry carries | The call returns `None` | `pictura-codec::removing_an_unrelated_uuid_changes_nothing` |
| `sfm_render` | Composite a filtered smart object whose mask is black over a region | The region shows the unfiltered source; the rest the filtered result | `pictura-render::the_chain_renders_through_the_mask` |
| `sfm_render_none` | Composite the same object with `filter_mask` `None` | Every pixel is the filtered result | `pictura-render::a_layer_without_a_mask_is_unchanged` |
| Photoshop reopens an authored mask | — | — | manual: create-from-selection is deferred; no Photoshop to validate an authored `FEid` (#956) |
