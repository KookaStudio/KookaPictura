# Test plan

This plan covers `compositing/whole-picture-readers` for `whole-picture-readers`
(#1274).

**Fixtures.** The identity tests use seeded layered documents at odd sizes
(e.g. 997×613), so that tiles and bands fold at the right and bottom edges. The
documents include a group, a clipped layer, a masked layer, a layer with a drop
shadow, and a working profile for the sRGB paths. The whole-composite reference
is today's path (`composite_rgba`, or `picture()` before migration), and it runs
only at these small sizes. The heap guards use the `tests_profiles/memory.rs`
harness at 4250², where one RGBA composite is 68.9 MiB. They assert
`peak < output + composite / 2`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `wp_export` | Build the encoder input for export (sRGB), Save for Web (sRGB) and a raster save format (working space) of a profiled layered document; repeat under the heap guard | The RGBA handed to the unchanged encoders equals the whole composite's; peak below the packed RGBA + half a composite | `pictura-app::export_streams_into_one_rgba`, `pictura-app::export_heap_holds_no_composite` |
| `wp_copy_merged` | Copy Merged with a small selection and with none | Clips equal the whole-composite clips; the region read is the selection bounds | `pictura-app::copy_merged_reads_the_selection_bounds` |
| `wp_image_ops` | Trim a document with transparent margins; Duplicate (merged) a layered document; Trim under the heap guard | Same bounds and pixels as the whole-composite path; Trim keeps no canvas-sized buffer | `pictura-app::trim_and_duplicate_merged_stream`, `pictura-app::trim_heap_holds_no_composite` |
| `wp_channel_selection` | Load the R, G, B and composite rows as selections | Each equals the whole-composite derivation | `pictura-app::channel_row_selection_streams` |
| `wp_match_color` | Take source statistics with and without a selection | Bit-identical to the whole-composite statistics | `pictura-adjust::banded_stats_equal_whole_stats`, `pictura-app::match_color_stats_stream` |
| `wp_select_tiles` | Grow, Similar and the wand over tiles and over a buffer at odd sizes; Similar under the heap guard | Identical selections; peak below the selection + half a composite | `pictura-select::tiled_source_matches_the_buffer`, `pictura-app::similar_heap_holds_no_composite` |
| `wp_refine` | Refine Edge (radius, smooth, feather, contrast, shift) and decontaminate over tiles and over a buffer | Identical masks and pixels | `pictura-select::tiled_refine_matches_the_buffer` |
| `wp_magnetic` | Build the edge map from tiles and from a buffer, trace the same wires; begin a gesture under the heap guard | Same peak and paths; no canvas-sized composite | `pictura-select::tiled_edge_map_matches_the_buffer`, `pictura-app::magnetic_begin_heap_holds_no_composite` |
| `wp_preview_level` | Request the preview picture the Color Range, Refine Edge and Replace Color previews read at 256 on a large and on a small document; repeat under the heap guard | Large: the coarsest level ≥ 256, peak below half a composite; small: equal to today's whole picture | `pictura-app::previews_read_a_reduced_level`, `pictura-app::small_previews_are_unchanged`, `pictura-app::preview_heap_holds_no_composite` |
| `wp_capture` | Build `preview_image`'s display level on a large document and on a small one | Large: the coarsest level ≥ the size, equal to that level of the display pyramid; small: no level, so `image()`'s pixels | `pictura-app::preview_image_reads_a_reduced_level`, `pictura-app::small_previews_are_unchanged` |
| `wp_conversion` | Convert a layered RGB document to Grayscale, CMYK, Lab and 16 bits by the streamed and the whole path; Grayscale under the heap guard | Equal documents; Grayscale peak below output + half a composite | `pictura-render::streamed_conversions_match_the_whole_path`, `pictura-app::gray_conversion_heap_holds_no_composite` |
| `wp_filter_backdrop` | Build the Liquify backdrop for a small layer and the Vanishing Point backdrop | Each equals the same area of the whole hidden-layer composite; Liquify reads only its frame | `pictura-app::liquify_backdrop_reads_its_frame`, `pictura-app::vanishing_point_backdrop_streams` |
| `wp_move_fallback` | Build move and transform backdrops for a group, a drop-shadow layer and an adjustment layer's sibling | A patch over the influence that equals the whole hidden composite's crop | `pictura-app::move_backdrop_patches_the_influence` |
| 17000² profile | `python3 scripts/profile-tools.py` peak runs for export, Color Range, Similar and a Grayscale conversion at 17000² | No 1.16 GB composite in any peak | manual: 17000² needs the reference machine; the 4250² heap guards run in CI |
