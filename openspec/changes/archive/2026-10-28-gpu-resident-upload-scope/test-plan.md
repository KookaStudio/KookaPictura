# Test plan — gpu-resident-upload-scope

One row per scenario in `specs/compositing/gpu-transfer/spec.md` (the modified
resident-upload requirement and the added store requirement). The last two rows
guard the pre-existing region and parity contracts in
`openspec/specs/compositing/gpu-compositing/spec.md` that this change must not
regress. Coverage names the test that proves the row; every row is automated.

Tests named here are created by `tasks.md`; `scripts/check-test-plan.py` reports
"no such test" until the implementation tasks land them.

Run rows with:

```bash
cargo nextest run -p pictura-render resident
cargo nextest run -p pictura-render --features software-vulkan --profile gpu --run-ignored all resident
cargo nextest run -p pictura-render --features software-vulkan --profile gpu --run-ignored all region_matches_full_slice full_region_equals_full_composite
cargo nextest run -p pictura-render --features software-vulkan --profile gpu --run-ignored all gpu_matches_cpu_within_one_lsb groups_and_masks_match_cpu
```

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `gt_resident` | Composite a stamped document twice on the GPU, change a layer's blend mode, write a plane without a stamp and then with a fresh stamp | The repeated composites reuse the uploads and every composite matches the CPU within ±1 LSB | `pictura-render::an_unstamped_plane_is_never_cached`, `pictura-render::resident_uploads_are_reused_and_never_stale` |
| A region composite reuses whole-document sources | Stamp four full-canvas layers, seed with a whole-document composite, then composite 100 moving sub-rects; snapshot the resident counters around the loop | `resident_hits` grows by exactly four per present; every region composite reports `Backend::Gpu` | `pictura-render::resident_uploads_stay_bounded_across_region_presents`, `pictura-render::a_source_key_follows_its_planes_and_geometry` |
| Moving region composites leave the store flat | Compare `resident_entries` and `resident_bytes` before and after the 100 moving sub-rects; then composite the whole document | Both gauges are unchanged; the whole-document composite adds zero misses | `pictura-render::resident_uploads_stay_bounded_across_region_presents` |
| Constant coverage is rebuilt, never retained | Ask for the key of a pixel layer with no mask, and of a layer whose enabled data mask is `disabled` | Both return no key, so no coverage upload is offered to the store | `pictura-render::a_mask_key_needs_a_stamped_plane_only_when_it_samples_one` |
| A sampled mask is reused by a repeated whole-document composite | Stamp a layer's mask plane, composite the whole document twice, and read the hit delta around the second | The second full composite reports one more resident hit for the mask upload | `pictura-render::resident_uploads_stay_bounded_across_region_presents` |
| The byte budget evicts the oldest uploads | Put two uploads whose combined size exceeds the budget, each within it, into a `Resident<u64>`; get both keys and read the stats | Only the most recent upload remains, the byte total is within the budget, and the oldest is gone | `pictura-render::put_evicts_past_the_byte_budget` |
| An upload larger than the budget is not retained | Put an upload larger than `BUDGET` into an empty `Resident<u64>` and read the stats | The entry count and byte total stay at zero | `pictura-render::put_refuses_an_upload_over_the_budget` |
| The entry count stays capped | Put more distinct keys than `MAX_ENTRIES` into a `Resident<u64>` | The retained count equals the cap and the oldest keys are gone | `pictura-render::put_keeps_the_entry_count_bounded` |
| Keeping a key again replaces its entry | Put a key, put it again with a new value/size, then evict past the budget | Exactly one entry exists for the key, the byte total counts only the newer size, and the newer value is the one returned | `pictura-render::put_replacing_a_key_keeps_one_entry` |
| A retained upload is found by its key | Put an upload and get its key; get an unknown key | The put upload is returned; the unknown key returns nothing | `pictura-render::get_returns_a_kept_upload` |
| A lookup does not mutate the store | Read a retained upload and compare `(entries, bytes)` before and after | The store's entries and byte total are unchanged | `pictura-render::get_leaves_the_store_unchanged` |
| The report reflects entries and bytes | Snapshot `metrics::Counters` before and after the seeding whole-document composite; also check the post-eviction `(entries, bytes)` returned by `put` | The entry count grows by exactly the source and sampled-mask count, the byte total is non-zero, and the post-eviction stats match the retained entries | `pictura-render::resident_uploads_stay_bounded_across_region_presents`, `pictura-render::put_reports_post_eviction_stats` |
| Region output is unchanged (regression) | After stamping `region_scenes`, composite the whole document, then several sub-rects including partial layer overlap, and compare every pixel with the full slice | Byte-identical at 0 LSB, and the whole-rect region call equals the full composite | `pictura-render::region_matches_full_slice`, `pictura-render::full_region_equals_full_composite` |
| Full output stays within one LSB (regression) | Composite every supported blend-mode scene and a masked/group stack on the GPU and compare with `composite_rgba` | Every channel differs by at most 1 LSB | `pictura-render::gpu_matches_cpu_within_one_lsb`, `pictura-render::groups_and_masks_match_cpu` |
