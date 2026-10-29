# Test Plan

One row per scenario in the delta specs. `manual:` rows are reviewer checks;
everything else names the automated test that proves it.

## compositing/layer-source-cache

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| A region pass equals the full pass | Build a document with a drop shadow (200 px distance), outer glow, inner shadow, inner glow, stroke, satin, bevel, and color/gradient/pattern overlay. Build the passes for the whole layer and for a sub-region, then compare each pass's overlap and bounds. | Every pass sample inside the region is identical to the full build and every pass rect lies inside the requested region. | `pictura-render::region_pass_equals_full_pass_for_every_family` |
| A full composite still reuses its cached pass | Composite a styled document whole twice with the cache warm. | The second composite reports a pass hit and rebuilds no pass. | `pictura-render::every_source_kind_hits_on_the_second_composite` |
| A stroke present rebuilds only the dab and its reach | Paint a dab on a canvas-sized styled layer, then build the pass for the dab region grown by the effect reach. | Every pass rect lies inside the present region, and the region composite equals the full composite's slice. | `pictura-render::dab_region_pass_is_bounded_and_equal` |
| An overlay source is bounded by the region | Generate an aligned and an unaligned gradient, and a linked and an unlinked pattern, over a sub-region and over the full extent. | The region buffer length is the region area and every sample equals the full buffer's slice. | `pictura-render::gradient_region_matches_the_full_buffer_slice`, `pictura-render::pattern_region_matches_the_full_tile_slice` |
| A banded sub-region reuses one pass | Composite a tall sub-region in bands and whole. | One whole-layer pass is built and replayed into every band, and the banded output equals the whole composite. | `pictura-render::banded_sub_region_reuses_one_pass` |
| A sub-region pass is not retained | Run a non-banded sub-region composite on a styled layer and inspect the shared cache. | The cache gains no pass entry for that region. | `pictura-render::sub_region_pass_is_not_retained` |
| Replacing smart-object contents invalidates | Replace a smart object's payload, composite, then purge and composite again. | The two composites are byte-identical. | `pictura-render::replacing_smart_object_contents_invalidates_the_composite` |
| Editing text invalidates | Edit the type model or the font registry, composite, then purge and composite again. | The two composites are byte-identical. | `pictura-render::editing_text_invalidates_the_composite`, `pictura-render::text_raster_hits_and_invalidates`, `pictura-render::text_state_invalidates_sources_and_passes` |
| Editing an effect invalidates | Change an effect parameter, the master switch, or the layer mask, composite, then purge and composite again. | The two composites are byte-identical. | `pictura-render::effect_passes_invalidate_on_style_or_rect`, `pictura-render::effect_passes_invalidate_on_master_switch_and_mask` |
| A painted layer invalidates its region pass | Paint a dab, build the region pass, then compare it with a pass built from scratch for the same region. | Identical pass bytes. | `pictura-render::dab_region_pass_is_bounded_and_equal` |
| A second composite hits | Composite an unchanged document containing a proxy-less smart object, a smart-filtered layer, a proxy-less type layer, and a styled layer twice. | The second composite reports a cache hit for each layer and does not repeat their work. | `pictura-render::every_source_kind_hits_on_the_second_composite` |
| A hidden layer costs nothing | Composite a document whose smart-object and type layers are not visible. | Neither source is decoded or cached and the backdrop is unchanged. | `pictura-render::hidden_layer_is_not_decoded` |
| A warm region composite is at least ten times faster | Run the ignored 512×120 region benchmark with a warm cache and against a purged baseline. | The warm region composite is at least ten times faster and repeated full composites hit the cache. | `pictura-render::warm_region_is_ten_times_faster` |
| A canvas-sized styled present beats a full composite | Run a dab-sized region composite on a canvas-sized styled layer and compare it with a full composite. | The region composite is at least ten times faster and equals the full composite's slice. | `pictura-render::canvas_sized_styled_present_is_faster_than_a_full_composite` |

## compositing/mask-refinement

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Feather softens a hard edge | Feather a hard black/white mask by 8 and read across the edge. | Values are monotonic across the edge and intermediate at the edge. | `pictura-render::feather_ramps_a_hard_edge_monotonically` |
| Feather reaches past the mask rect | Feather a white mask with default colour 0 by 8 and sample just outside its rect. | Values just outside the rect are above 0. | `pictura-render::feather_reaches_past_the_rect_into_the_default_colour` |
| A region composite feathers only its region | Composite a feathered mask whole and over a sub-region; inspect the built plane bounds. | Region pixels equal the full slice and the plane is bounded by the mask's bounds intersected with the region, grown by the feather reach. | `pictura-render::mask_feather_region_equals_the_full_slice` |
| A mask dab rebuilds its region | Write a dab into a feathered mask, composite the dab region, inspect the plane bounds, and compare with a full composite. | Region pixels equal the full slice and the rebuilt plane is bounded by the mask's bounds intersected with the region, grown by the feather reach. | `pictura-render::mask_dab_region_equals_the_full_slice` |

## tools/paint-engine

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `lpe_dirty_per_dab` | Place several dabs and read `Stroke::take_dirty` after each. | Each read reports only the dabs since the previous read. | `pictura-paint::take_dirty_reports_each_dab_not_the_stroke_union` |
| A present covers only the dabs since the previous present | Place several dabs between two presents of a stroke on a styled layer. | The present rectangle is no larger than the dab union grown by the paint-change reach. | `pictura-app::paint_present_covers_the_dab_union_plus_reach` |
| `lpe_region_equal` | Composite a region that intersects a bounded effect and compare with the full composite. | Every region pixel equals the full composite. | `pictura-render::a_region_an_effect_reaches_still_matches_the_full_composite` |
| `lpe_present_effect_reach` | Paint a dab on a styled layer (outer and inner effects), then on a feathered mask, and inspect the present rectangle before committing. | The present rectangle includes the paint-change reach and feather reach, so the moved halo and ramp are refreshed. | `pictura-app::paint_present_covers_the_dab_union_plus_reach`, `pictura-app::mask_present_covers_the_feather_reach`, `pictura-app::styled_layer_dab_commit_equals_full_recomposite`, `pictura-app::feathered_mask_dab_commit_equals_full_recomposite` |
| `lpe_dab_budget` | Run the ignored 4000×4000 dab profile on the reference machine. | Input-to-first-pixel at most 16 ms and sustained redraw at or above 60 FPS. | manual: wall-clock timing depends on the machine; `cargo test -p pictura-app --lib large_brush_profile_4000 -- --ignored --nocapture` (and `mask_dab_profile_4000`) reports the numbers |

## compositing/described-damage

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| A styled-layer dab refreshes the halo | Paint a dab on a layer with a drop shadow, inner shadow, satin, or bevel, commit, and compare `doc.composite` with a full recomposite. | Equal, and the present/commit rectangle includes the halo. | `pictura-app::styled_layer_dab_commit_equals_full_recomposite`, `pictura-app::inner_effect_dab_commit_equals_full_recomposite` |
| A mask dab refreshes the feather ramp | Paint a dab on a feathered raster mask, commit, and compare `doc.composite` with a full recomposite. | Equal, and the refreshed rectangle includes the feather reach. | `pictura-app::feathered_mask_dab_commit_equals_full_recomposite` |
| A cancelled styled stroke restores the halo | Paint on a styled layer, cancel, and compare `doc.composite` with a full recomposite of the restored document. | Equal; the pre-stroke halo is restored without a full-document composite. | `pictura-app::cancel_styled_stroke_restores_the_halo` |
| The grown rectangle is clipped to the canvas | Commit a dab near the canvas edge and inspect the refreshed rectangle. | The refresh is clipped to the canvas and no out-of-canvas pixel is touched. | `pictura-app::dab_near_the_canvas_edge_clips_the_grown_rect` |
