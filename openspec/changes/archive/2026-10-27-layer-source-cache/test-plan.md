# Test plan — layer-source-cache

One row per scenario in the change's spec deltas
(`specs/compositing/layer-source-cache/spec.md`,
`specs/compositing/described-damage/spec.md`, and
`specs/compositing/banded-compositing/spec.md`). Coverage names the test that
proves the row; `manual:` is used only where the check cannot run in CI.

Tests named here are created by `tasks.md`; `scripts/check-test-plan.py` reports
only "no such test" until the implementation tasks land them.

Run rows with:

```bash
cargo nextest run -p pictura-render layer_source
cargo nextest run -p pictura-render region
cargo nextest run -p pictura-render layer_effects
cargo nextest run -p pictura-codec smart_object
cargo nextest run -p pictura-app
ctest --test-dir build -R '^tst_layer_style' --output-on-failure
cargo test -p pictura-render --test source_cache_profile -- --ignored --nocapture
```

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| Cold and warm composites agree | Composite a fixture with a smart object, smart filter, type layer, mask, and effects; purge; composite again; compare bytes | Byte-identical outputs | `pictura-render::cold_and_warm_composites_agree` |
| Existing oracle suites still pass | Run the full render oracle suites | No baseline change | `pictura-render::fixture_scenes_match_oracle_on_active_backend` |
| A second composite hits | Composite an unchanged document twice; read the source-cache counters | The second composite reports hits for the smart object, smart filter, type layer, and styled layer, and repeats no decode/filter/shape/effect build | `pictura-render::every_source_kind_hits_on_the_second_composite` |
| A hidden layer costs nothing | Hide a proxy-less smart object and a proxy-less type layer; composite; read counters | No decode/raster for those layers; backdrop unchanged | `pictura-render::hidden_layer_is_not_decoded` |
| Replacing smart-object contents invalidates | Place a smart object, read it, replace its payload, composite | Digest changes; output equals purge-then-composite | `pictura-codec::payload_digest_tracks_replace_and_crs_patch`, `pictura-render::replace_clears_proxy_and_renders_new_source` |
| Editing text invalidates | Change text, style, box, vertical, register a font, flip missing-glyph protection; composite each time | Each change misses; the unchanged repeat hits; output equals purge-then-composite | `pictura-render::text_raster_hits_and_invalidates` |
| Editing an effect invalidates | Change an effect parameter, the master switch, the mask, and the layer rect; composite each time | Each change rebuilds passes; output equals purge-then-composite | `pictura-render::effect_passes_invalidate_on_style_or_rect`, `pictura-render::effect_passes_invalidate_on_master_switch_and_mask` |
| A styled layer's region matches the full slice | Composite a full document and a region intersecting each effect family; compare every pixel | Equal | `pictura-render::styled_layer_region_matches_the_full_slice` |
| A smart-filtered region matches the full slice | Repeat with a Lens Correction smart filter | Equal (this fails before the change) | `pictura-render::smart_filter_region_matches_the_full_slice` |
| No bounded effect forces a full composite | Composite a 4000² document's 512×120 region near a styled layer; read `metrics::take_largest_canvas` | Largest canvas is region-sized, not document-sized | `pictura-render::bounded_effect_does_not_full_composite` |
| One decode for a multi-effect layer | Purge; composite a layer with shadow, inner shadow, stroke, bevel; read the `effect_decodes` counter | Exactly one descriptor decode for that layer | `pictura-render::effects_decode_once` |
| A constant-colour effect replays exactly | Build and replay a shadow/glow pass on a known canvas; compare to the uncached composite | Every touched pixel equal | `pictura-render::effect_passes_are_byte_identical_cached` |
| A per-pixel-colour effect replays exactly | Repeat with gradient overlay, pattern overlay, and stroke | Every touched pixel equal | `pictura-render::effect_passes_are_byte_identical_cached` |
| Banded equals whole with object layers | Composite a tall document with smart-filtered, type, and styled layers banded and whole | Byte-identical | `pictura-render::banded_equals_whole_with_object_layers`, `pictura-render::pattern_effects_disable_banding_and_stay_identical` |
| A slider tick recomposites the paint bounds union | Change a drop-shadow size through `layer_style_set` and capture `regionDamaged`; exercise a group, a masked adjustment, and a missing path through the Rust helper | Rect is larger than the layer and smaller than the document where the bounds are smaller; the Rust helper covers group, masked adjustment, and missing path | `tst_layer_style::previewIsBounded`, `pictura-render::paint_bounds_unions_a_group_and_bounds_a_masked_adjustment` |
| A non-pixel style change does nothing | Request a refused or unchanged style value | No recomposite and no damage signal | `tst_layer_style::refusedValueEmitsNothing` |
| The budget evicts | Insert artifacts past the byte budget; request the oldest | It rebuilds correctly and resident bytes stay under budget | `pictura-render::eviction_respects_the_byte_budget` |
| Purge forces rebuilds | Purge; composite; compare with the pre-purge output | Miss then byte-identical output | `pictura-render::purge_forces_a_rebuild` |
| Counters are visible | Composite, then read the metrics report | Report includes source hits, misses, resident bytes, and effect decodes | `pictura-render::counters_are_visible` |
| A warm region composite is at least ten times faster | Run the ignored profile on a 4000² document: cold vs warm 512×120 region, repeated full composites | ≥10× warm region; full composites report hits | `pictura-render::warm_region_is_ten_times_faster` |
| A dimension change recomposites fully | Resize, crop, rotate, flip, or change the canvas size | The whole document is recomposited and the whole canvas is marked damaged | `pictura-app::dimension_change_recomposites_fully` |
| An unbounded adjustment recomposites fully | Apply an adjustment layer with an unbounded mask or a non-zero default color | The whole document is recomposited | `pictura-app::unbounded_adjustment_recomposites_fully` |
| A layer with an effect block recomposites fully | Edit a layer with an effect block inside a knockout/pass-through stack whose pixels cannot be confined to a rectangle | The whole document is recomposited | `pictura-app::unbounded_effect_recomposites_fully` |
| `dd_effect_bounded` | Change a Layer Style value on a layer with a bounded effect block | Only the union of the before and after reach is recomposited | `tst_layer_style::previewIsBounded` |
| `bc_exact` | Composite the offset/partial-layer stack whole and over a sub-region | Both byte-identical to the single-canvas composite | `pictura-render::banded_compositing_equals_one_canvas` |
| `bc_type` | Add a proxy-less type layer to the same stack | Banded equals the single-canvas composite | `pictura-render::banding_with_a_short_type_layer_equals_one_canvas` |
| `bc_objects` | Composite a tall document with a smart-filtered layer, a proxy-less type layer, and a styled layer banded and whole | Byte-identical | `pictura-render::banded_equals_whole_with_object_layers`, `pictura-render::pattern_effects_disable_banding_and_stay_identical` |
