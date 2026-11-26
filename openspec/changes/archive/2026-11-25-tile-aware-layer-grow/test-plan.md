# Test plan: tile-aware-layer-grow

`pictura-core` unit tests check the grow: sample equality with a row copy, tile
sharing, and skipped empty tiles. The stroke-level speed is a timing on a
21,000² document. It runs in the ignored release profile
`long_stroke_phases_profile` and is checked by hand.

| Scenario | Steps | Expected result | Coverage |
| --- | --- | --- | --- |
| `lgc_pixels` | Pad a 600 × 700 plane with an empty tile row, a uniform tile and dense noise (1 and 2 channels) at offsets (0, 0), (256, 512), (37, 0), (0, 101) and (130, 259). | Every sample equals a row-by-row copy into a zero plane; the existing small grow keeps its pixels and retained samples. | `pictura-core::padded_matches_a_row_copy_at_every_offset`, `pictura-core::grows_channels_and_retained_samples` |
| `lgc_share` | Pad the 600 × 700 plane by 900 px right and down. Then grow a tiled 512² four-channel layer right and down with `grow_layer_to`. | Every full tile keeps its state, and the shared allocations equal the old full dense tiles. The layer's planes hold exactly their old allocations, old pixels are kept, and the new area reads 0. | `pictura-core::a_grow_right_and_down_shares_every_full_tile`, `pictura-core::a_grow_right_and_down_keeps_the_tiles` |
| `lgc_empty` | Pad a 1024² plane with one painted pixel at offset (77, 141) inside `assert_no_materialization`. | Exactly one dense tile is written; no materialization; samples equal a row copy. | `pictura-core::empty_tiles_are_not_copied` |
| `lgc_long_stroke` | Run `cargo test --release -p pictura_app --lib long_stroke_phases_profile -- --ignored --nocapture` and read each stroke's grow, dabs and record. | The grow is a small part of each stroke; before the change it was 1.4–6.7 s of each stroke. | manual: wall-clock timing on a 21,000² document needs an optimized build and an idle machine, so it is an ignored profile run by hand (`pictura_app::long_stroke_phases_profile`) |
