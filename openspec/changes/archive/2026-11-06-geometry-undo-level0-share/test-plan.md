# Test plan

Every scenario of this change, with the check that proves it. The profile row is
manual because it needs a 16507×16196 document and a quiet machine.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `vp_banded_rebuild` | Build a view from a plane whose bands were forked from a shared base, and from the same pixels as contiguous slices | Every level is byte-identical and `Plane::is_mirrored()` is false for the banded source | `pictura-render::a_banded_level0_builds_the_same_levels` |
| `vp_banded_update` | Update one region and crop level 0 from the banded plane, then repeat on the contiguous slices | Both match byte for byte and neither builds a contiguous copy | `pictura-render::a_banded_level0_updates_and_crops` |
| `iph_segment_keeps_composite` | Capture a reorder that leaves the composite byte-identical, then undo it | The live composite shares the state's plane, the undo reports an empty rectangle, and the composite is not rescanned | `pictura-app::a_segment_break_keeps_an_unchanged_composite` |
| `iph_segment_changed_composite` | Capture a reorder that changes the composite, then undo it | The undo reports the changed composite tiles and the live document matches the target state | `pictura-app::a_segment_break_keeps_a_changed_composite` |
| `hrr_level0_shared` | Reset the display for an sRGB document, then check level 0 | Level 0 shares the composite's plane and equals a full copy of it | `pictura-app::a_whole_document_restore_shares_the_composite_plane` |
| `hrr_level0_forks` | Patch a region into a shared level 0 | Only the region's bands fork, the composite's bytes are unchanged, and the frame shows the patched pixels | `pictura-app::a_region_patch_forks_only_its_bands` |
| `hrr_level0_converted` | Install level 0 for a document with a non-sRGB profile | The frame holds converted sRGB pixels and shares no plane with the composite | `pictura-app::level0_carries_the_document_profile_conversion` |
| `hrr_mask_solo_exact` | Build a solo mask plane for a document with an off-rect mask and a default colour, and the serial reference | Every pixel is the serial sample, opaque and grey | `pictura-app::mask_display_plane_matches_a_serial_build` |
| `hrr_mask_solo_manual` | Run `structural_restore_profile` with a soloed mask on a 16507×16196 document | The mask plane and pyramid rebuild in parallel time, not the serial pass | manual: the profile needs a 16507×16196 document and a quiet machine; it is an ignored Rust profile, not an assertion |
