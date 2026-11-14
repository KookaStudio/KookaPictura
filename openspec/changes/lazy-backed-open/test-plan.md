# Test plan

This plan covers `lazy-backed-open` (#1218): the new capabilities
`codec/lazy-psd-open` and `document/source-backed-tiles`.

**Fixtures.**

- **Codec tests** write PSDs with the crate's own writer: a layered RGB/8
  document of 600 × 1100 (several bands, a partial tile column) with seeded
  noise layers, a user mask and a Background, saved RLE and raw. The eager read
  (`read_psd_within_options` without a source) is the oracle. Each test writes
  under its own directory in `std::env::temp_dir()`.
- **Core tests** build backed planes from a closure over a known pattern and
  count its calls, so no file is involved.
- **App tests** open the codec fixture through `decode_psd_with` on a
  `PictureViewRust` built offscreen, as `tiles_tests.rs` does.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `lp_identical` | Open the round-trip corpus and the RLE and raw fixtures lazily and eagerly; save both | Documents equal; saved bytes identical | `pictura-codec::lazy_open_matches_eager` |
| `lp_no_decode` | Open the RLE fixture lazily | Every layer and mask dense tile is away and backed; band decode count 0 | `pictura-codec::lazy_open_decodes_no_layer_pixels` |
| `lp_eager_kinds` | Open a ZIP 8-bit and a 16-bit layered PSD lazily | No backed tile; documents equal eager | `pictura-codec::zip_and_deep_channels_open_eagerly` |
| `lp_rows` | Read one tile of a 1024-row RLE channel, then its band's other tiles | Bytes read = the band's rows; no more bytes for the siblings | `pictura-codec::a_backed_tile_reads_only_its_band` |
| `lp_hostile_index` | Lazily open a PSD with a count table longer than its channel, and one whose channel ends past the file | Same error variant as the eager open | `pictura-codec::lazy_index_rejects_short_channels` |
| `lp_hostile_decode` | Open lazily, truncate the file in place, read a tile past the end | Default samples, tile unreadable, no panic | `pictura-codec::lazy_decode_of_a_truncated_source_is_unreadable` |
| `lp_preview` | Open the Background fixture lazily; read the preview over a rect crossing two bands | RGB equals the eager merged image's crop; alpha 255 | `pictura-codec::merged_preview_matches_the_merged_image` |
| `lp_preview_gate` | Open a fixture whose bottom layer is not a Background, and one with a hidden Background | No preview for either | `pictura-codec::merged_preview_needs_a_visible_background` |
| `sb_redecode` | Read a backed tile, evict it, read again | Same samples; pass wrote 0 bytes; decoder called twice | `pictura-core::backed_tile_dropped_and_redecoded_identity` |
| `sb_detach` | Write a backed tile, evict it, read it | Packed or spilled, not backed; written samples | `pictura-core::a_written_backed_tile_leaves_its_source` |
| `sb_transient` | Read an away backed tile under `transient` | Decoded samples; tile still away | `pictura-core::a_transient_read_leaves_a_backed_tile_away` |
| `sb_priority` | One cold backed and one cold live resident tile; pass for one tile's bytes | Backed freed; live resident | `pictura-core::backed_tiles_are_evicted_first_without_writes` |
| `sb_replaced` | Open lazily; rename another file over the path; read every tile | Equals the eager open of the original | `pictura-codec::source_replaced_keeps_backed_tiles` |
| `sb_deleted` | Open lazily; delete the file; read every tile | Equals the eager open of the original | `pictura-codec::source_deleted_while_open` |
| `sb_modified` | Open lazily; rewrite the file in place with other bytes; read an unread tile | Tile unreadable | `pictura-codec::source_modified_in_place_marks_tiles_unreadable` |
| `sb_save_over` | Open lazily in a view; save over the path; fill a layer rect; save again; compare with the same on an eager open | Second files identical | `pictura-app::save_over_source_with_backed_tiles` |
| `sb_first_paint` | Open the Background fixture lazily in a view; build the fit tiles | Tiles equal the preview pyramid; no layer tile restored | `pictura-app::first_paint_shows_the_files_merged_image` |
| `sb_first_edit` | Then fill a layer rect and rebuild | Preview dropped; tiles equal the layer composite | `pictura-app::an_edit_replaces_the_merged_preview` |
| `sb_picture` | Read `picture` while the preview is shown | Equals the layer composite | `pictura-app::picture_reads_layers_while_the_preview_shows` |
| `sb_m12` | `python3 scripts/profile-tools.py --scenarios M12 --size 17000`, eager and lazy; then `--memory-max 2G` with pan, edit, save | First paint and RSS after it recorded against the eager baseline; constrained run ok, output equal | manual: needs multi-GB generated fixtures, a systemd cgroup memory limit and minutes of a real app; run for this change and recorded in `docs/dev/STATE.md` |
