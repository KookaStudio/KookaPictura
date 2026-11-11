# Test plan

This plan covers `compositing/region-composite`, `compositing/tile-pyramid`, and
the modified `compositing/current-picture` and `compositing/view-pyramid` for
`tiled-composite` (#1216).

**Fixtures.** The engine tests use seeded layered documents built by
`pictura-render`'s `test_support` (every blend mode the oracle covers, a group, a
clipped layer, a masked layer, and one layer with a drop shadow), at odd sizes
(e.g. 997×613) so that the right and bottom edges fold at every level. The
whole-composite reference (`composite_rgba` + `ViewPyramid::rebuild`) is used
only at these small sizes (R5). The app tests use `PictureViewRust` documents of
the same shape. A "large" document in a unit test is one above the 16 M-pixel
deferral threshold, made of uniform tiles so it costs no memory.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `rc_level0` | Composite random rectangles at level 0 of each seeded document | Each equals the same crop of `composite_rgba` | `pictura-render::composite_region_matches_full_crop` |
| `rc_levels` | Composite random rectangles at levels 1–3, including ones that touch the odd right and bottom edges | Each equals the same crop of `ViewPyramid::rebuild` of the whole composite | `pictura-render::composite_region_matches_full_crop`, `pictura-render::composite_region_folds_odd_edges` |
| `rc_outside` | Request a rectangle half outside the level | The outside part is transparent; the inside part equals the crop | `pictura-render::composite_region_outside_is_transparent` |
| `rc_banded` | Composite a whole-document region at level 2 with a band budget smaller than the footprint | Equals the unbanded result; the heap peak stays below one level-0 footprint | `pictura-render::streamed_region_equals_unbanded` |
| `rc_save` | Save the corpus's layered RGB PSDs with the streamed merged image and with the whole composite | The files are byte-identical | `pictura-codec::streamed_merged_image_is_byte_identical`, `pictura-app::save_streams_the_merged_image` |
| `rc_save_peak` | Save a large layered document under the heap guard | No allocation of the full composite size in the merged-image path | `pictura-app::save_allocates_no_whole_composite` |
| `rc_flatten` | Flatten a layered document | The flattened layer equals the whole composite; no canvas-sized composite buffer besides the new layer | `pictura-render::flatten_composites_by_band` |
| `rc_sample_all` | Magic Wand, Paint Bucket and Eyedropper with Sample All Layers on a layered document | Same masks and colours as sampling the whole composite; the whole-composite counter is unchanged | `pictura-app::sample_all_reads_regions`, `tst_eyedropper::sampleAllLayersReadsTheComposite` |
| `tp_crop` | Request crops at levels 0–3 from a cold cache | Each equals the same crop of a rebuild from the whole display composite | `pictura-app::tile_crops_match_a_whole_rebuild` |
| `tp_visible_only` | Present a large document at 100 % in an 800×600 view | Only tiles under the view and its margin are resident | `pictura-app::only_visible_tiles_are_built` |
| `tp_no_frame` | Open and present a large layered document | No resident buffer of the full display size (no `level0`, no stored level) | `pictura-app::no_full_resolution_frame_is_kept` |
| `tp_damage` | Paint a region while the view shows it at levels 0 and 2 | Damaged tiles equal a rebuild; tiles outside the damage keep their allocation | `pictura-render::damage_repairs_cached_tiles`, `pictura-app::an_edit_repairs_the_visible_tiles` |
| `tp_stale` | Queue a tile build, damage the tile, deliver the old result | The old result is discarded; the next request rebuilds the edited tile | `pictura-render::a_stale_tile_is_not_installed` |
| `tp_history` | Undo, redo and restore a named snapshot with tiles cached; then on the canvas, fill, snapshot, edit and restore | Every presented tile equals a rebuild; the canvas shows the restored pixels | `pictura-app::history_presents_current_tiles`, `tst_history_snapshots::snapshotEditRestoreRoundTrip` |
| `tp_budget` | Request tiles over a document larger than a small test budget | Resident bytes never exceed the budget; level-0 tiles are evicted first | `pictura-render::the_cache_keeps_to_its_budget`, `pictura-core::the_display_budget_is_a_quarter_of_the_ceiling` |
| `tp_pressure` | Report high pressure with tiles cached and a view pinned | Unpinned tiles are dropped and the pinned ones remain | `pictura-render::pressure_drops_unpinned_tiles` |
| `tp_counters` | Present the same view twice | The second presentation is all hits | `pictura-render::hits_and_misses_are_counted` |
| `tp_async` | Pan a large document to uncached tiles | The crop comes back at once from coarser tiles; once the worker delivers, the exact tiles are presented | `pictura-app::a_missing_large_tile_does_not_block` |
| `tp_sync_small` | Present a small document from a cold cache; pan a canvas in the GUI | Every visible tile is exact on the first presentation; the panned canvas shows the document pixels | `pictura-app::small_documents_build_tiles_synchronously`, `tst_canvas_pan::panShowsTheDocumentPixels` |
| `tp_solo_restore` | Solo a mask, restore the whole document, compare the visible tiles, leave solo | Mask tiles equal the exact whole-mask pyramid; leaving solo shows the restored document tiles | `pictura-app::solo_mask_restore_presents_visible_tiles`, `tst_canvas_mask_domain::soloMaskRestoreShowsTheMask` |
| `tp_solo_budget` | Run `structural_restore_profile` at 16507×16196 in a 1920×1080 viewport and `solo_mask_profile` at 8000² | The visible mask is presented within 50 ms of GUI-thread time; worker completion and peak are recorded | manual: a timing bound at 16507×16196 is measured on the reference machine (`--ignored` profiles), and CI runners are too noisy and too small to assert it |
| `cp_after_edit` | Commit a pixel edit, then request a region of the picture | Served from the cache; no compositor run for cached tiles | `pictura-app::picture_is_served_after_a_committed_edit` |
| `cp_after_preview` | Present an opacity or fill preview, then request a region | Served from the cache, including the preview | `pictura-app::picture_is_served_after_a_preview` |
| `cp_after_history` | Undo, redo and jump, requesting a region each time | Served from the restored state's cache | `pictura-app::picture_is_served_after_history_navigation` |
| `cp_consumer` | Sample a region of a fresh cached document through Sample All Layers | No compositor run | `pictura-app::a_read_only_consumer_composites_nothing` |
| M1/M2/M12 peak | `python3 scripts/profile-tools.py --scenarios M1,M2,M12 --peak --size 17000x17000` | No 1.16 GB composite; peak at open ≤ layer tiles + 256 MiB | manual: 17000² runs need the reference machine; the 4250² heap guards run in CI (`pictura-app::new_document_heap_peak`) |
| M11 save peak | `profile-tools.py --scenarios M11 --peak --size 17000x17000` | Peak ≤ layer tiles + 128 MiB | manual: reference-machine size; 4250² guard in CI |
| M13 pan/zoom | `profile-tools.py --scenarios M13 --metrics` against master `8c740994` | Frame time within 10 % of the baseline; first paint ≤ baseline | manual: frame timing on a real display; offscreen CI cannot measure it |
| GPU tiles | Compare M13 with `gpu_compute` on and off | GPU tile misses are enabled only if they are faster; tiles are within the parity tolerances | manual: needs a Vulkan device; the parity itself is `pictura-render::gpu_region_matches_cpu` (ignored without an adapter) |
