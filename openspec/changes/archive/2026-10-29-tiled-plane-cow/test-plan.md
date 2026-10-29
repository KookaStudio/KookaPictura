# Test plan — tiled-plane-cow

One row per scenario in the delta specs, including the retained MODIFIED
scenarios. Coverage names the test that proves the row; every row runs in CI.

Run with:

```bash
cargo nextest run -p pictura-core plane
cargo nextest run -p pictura_app history
cargo nextest run -p pictura-paint stroke
cargo nextest run -p pictura-render
cargo test -p pictura_app large_document_open_profile -- --ignored --nocapture
```

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `dm_band_fracture` | Fracture a 4000×2000 plane for sharing; count allocations and inspect the band handles | No pixel bytes are allocated or copied, and the handles cover the original allocation | `pictura-core::fracturing_a_plane_copies_no_pixels` |
| `dm_band_write_forks_one` | Clone a fractured plane, write one band of the clone, compare every band allocation | Only the written band's allocation changed; every other band is `Arc::ptr_eq` with the clone; the clone's bytes are unchanged | `pictura-core::a_band_write_forks_only_its_band` |
| `dm_band_reads_agree` | Fracture, clone, write, then read contiguously and per band; read again with no write | Both reads equal the same operations on an unfractured plane; the second contiguous read allocates nothing | `pictura-core::contiguous_and_band_reads_agree` |
| `dm_band_round_trip` | Run arbitrary bytes through fracture, clone, and write on both a fractured and an unfractured plane | Every read is byte-identical | `pictura-core::fracture_preserves_every_byte` |
| `dm_band_whole_write_flattens` | Write a fractured plane through `as_mut_slice` and `DerefMut`; compare the old holders and the band view | The plane is one contiguous allocation with its band view replaced, and the old holders keep their bytes | `pictura-core::a_whole_plane_write_flattens` |
| `dm_band_degenerate_planes` | Fracture an empty plane, a width-zero plane, a non-multiple length with a partial last band, and re-fracture at the same and a different width | Empty and width-zero planes have no bands and read empty; the partial row stays in the last band; the same-width re-fracture is a no-op and a different width flattens once and rebuilds | `pictura-core::degenerate_planes_fracture_safely` |
| `iph_shared_bands` | Open a document, commit a stroke, undo; compare band allocations and bytes | Every restored state is byte-identical and every untouched band is shared by live and history | `pictura-app::opening_a_document_shares_bands_with_history` |
| `iph_capture_skips_shared_bands` | Dab inside one band, capture, inspect the scanned bands | Only the touched band is tile-scanned; every other band stays one shared allocation | `pictura-app::a_capture_scans_only_changed_bands` |
| `iph_fork_on_write` | Write the live document after a capture | Only the written bands are copied; the current state's bytes for those bands are unchanged | `pictura-app::a_live_write_forks_only_touched_bands` |
| `iph_hollow_anchor` | Capture several strokes after open, undo to the oldest state, then redo | Every state is byte-identical and the oldest state rebuilds from the band-shared current state, not a second retained copy | `pictura-app::live_restores_match_a_full_snapshot_history`, `pictura-app::every_state_matches_a_reference_copy` |
| `hr_structural_sharing` | Capture Open, then twenty New Layers without other pixel changes; read retained bytes | Retained bytes are one base, one copy per added layer's bands, and the tile deltas, not twenty documents | `pictura-app::structural_captures_retain_only_new_planes` |
| `hr_structural_exact` | Interleave add, delete, reorder, duplicate, and paint captures; undo, redo, and jump across anchors against a full-snapshot reference | Every restored document and selection is byte-identical, and a capture after a duplicate stays a delta | `pictura-app::live_restores_match_a_full_snapshot_history`, `pictura-app::structural_capture_with_a_duplicated_layer_round_trips`, `pictura-app::a_capture_after_a_duplicate_stays_a_delta` |
| `hr_live_private` | After every structural capture and in-place restore, write a band and compare allocations | The written band shares no allocation with any history state; unwritten bands may share | `pictura-app::a_written_band_is_not_shared_with_history` |
| `hr_restore_reuses_live` | Add a layer, capture, undo; compare live and target band allocations | Live keeps its own band allocations and copies only target-changed bands | `pictura-app::undoing_a_structural_edit_keeps_the_live_planes` |
| `hr_bounded_retention` | Capture many small edits on a large document; compare retained delta bytes and base count | Delta bytes stay bounded by the changed tile area and the history keeps one materialized base that may share unchanged bands with live | `pictura-app::region_deltas_retain_only_changed_tiles` |
| `hr_geometry_anchor` | Capture a resize, crop, flatten, or layer/mask structure change | The state is stored as a full snapshot and becomes a new anchor, and tile deltas span only a stable geometry | `pictura-app::geometry_change_uses_a_full_anchor_and_still_undoes` |
| `large_document_open_profile` | Open an 8-layer 4000×4000 RGBA document, take one stroke, and read the counters | The ignored profile prints unique tracked-plane bytes across the live document and all retained history, materializations during the stroke, and first-dab time | `pictura-app::large_document_open_profile` |

The ignored `large_document_open_profile` is the performance evidence: it prints
rather than asserts the targets because CI hardware varies; the deterministic
allocation bounds are asserted by the unit tests above.
