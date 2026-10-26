# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `lc_smart_descriptor` | Encode a full 16-slot list to the descriptor and decode it again | Decoded slots equal the encoded ones | `pictura-codec::slots_round_trip_through_the_descriptor` |
| `lc_smart_defaults` | Decode an empty descriptor and one missing a single key | Missing slots take the opening values | `pictura-codec::missing_keys_decode_to_the_opening_slots` |
| `lc_smart_attach` | Attach slots to an embedded smart object; read the filter back | Id, name, and decoded slots round-trip | `pictura-codec::attach_records_the_filter` |
| `lc_smart_attach_keeps` | Re-attach over a preserved descriptor with an extra `Fltr` key | Extra key and `blendOptions` survive; modelled keys are new | `pictura-codec::reattaching_keeps_unmodeled_keys` |
| `lc_smart_foreign` | Read a smart object whose filter id is not ours | Options bytes unchanged | `pictura-codec::a_foreign_filter_id_is_left_alone` |
| `lc_smart_render` | Apply the chain to a buffer with a Lens Correction filter | Equals `lens_correction` with the same parameters | `pictura-render::lens_correction_renders_the_chain` |
| `lc_smart_fallback` | Decode an unknown profile id and a bad slot count | Decode returns nothing | `pictura-render::unmappable_slots_are_refused` |
| `lc_smart_commit` | Apply on an embedded smart object | Filter attached, the composite corrected, one state, 16 slots | `pictura-render::smart_apply_attaches_the_filter`, `tst_lens_correction::smartObjectAddsASmartFilter` |
| `lc_smart_raster` | Apply on a plain raster layer | No smart object; pixels change | `tst_lens_correction::rasterLayerStaysDestructive` |
| `lc_smart_gate` | Run the commit on a 32-bit smart object | Refused, message names "8- or 16-bit" | `tst_lens_correction::smartObjectRefusesOffTheGate` |
| `lc_smart_prefill` | Commit, then reopen the dialog | Stored slots are the committed ones; controls show them | `pictura-render::stored_slots_read_back`, `tst_lens_correction::smartObjectPrefillsOnReEdit` |
| `lc_smart_no_double` | Build the session source for a corrected smart object | Unfiltered embedded source, not the baked proxy | `pictura-app::session_source_is_the_unfiltered_smart_source` |
| `lc_smart_replace` | Apply twice with different slots | One filter, holding the new slots | `pictura-render::smart_apply_replaces_the_entry` |
| Real Photoshop round-trip | Author a Lens Correction smart filter, reopen in Photoshop CS6/CC | — | manual: Pictura-only descriptor; the reference filter id and key map are unavailable, so no cross-app interop is claimed (#934 decision) |
| Interactive re-edit feel | Reopen on a 24 MP smart object and drag sliders | Preview stays responsive | manual: responsiveness on real hardware is not observable offscreen |
