# Benchmark — layer-source-cache (#969)

The benchmark is the ignored integration test
`crates/pictura-render/tests/source_cache_profile.rs`, function
`warm_region_is_ten_times_faster`. It builds a 4000×4000 RGB document with:

- a 200×200 embedded smart object (with a real merged composite) scaled into a
  2000×2000 layer and carrying a Lens Correction smart filter;
- a 400×200 proxy-less type layer with a drop shadow;
- no backdrop, so the measured work is the object layers themselves.

It then measures a 512×120 region composite at `(100, 100)` — short enough to
use the region path rather than banding — and a full 4000×4000 composite,
each cold (cache purged before every iteration) and warm.

## Reproduce

```bash
cargo test --release -p pictura-render --test source_cache_profile -- \
  --ignored --nocapture
```

Debug numbers are similar in ratio; use `--release` for absolute times.

## Environment

- 11th Gen Intel Core i7-11700K @ 3.60 GHz, 16 hardware threads
- Linux, Rust 1.98, release profile (`--release`)
- 5 iterations per measured phase

## Results (release)

| Phase | Cold | Warm | Ratio |
|---|---|---|---|
| 512×120 region composite | 242.29 ms/op | 3.49 ms/op | **69.4×** |
| Full 4000×4000 composite | 759.47 ms | 43.51 ms | **17.5×** |

Counters after the region phase: `hits=24`, `misses=25`,
`resident=16,772,452 bytes` (the 2000×2000 filtered chain), `effect_decodes=5`.
After the full phase: `hits=323`, `misses=53`, `effect_decodes=6` — the second
full composite serves the same artifacts from the cache.

Debug profile, same command without `--release`: region 249.74 → 3.28 ms/op
(76.0×), full 795.85 → 49.33 ms (16.1×), same resident size.

## What the cold cost is

The cold region pays the one-time smart-object decode plus the Lens Correction
warp over the full 2000×2000 layer rect, the type shaping/raster, and the effect
pass build. The warm region samples the cached decoded source, the filtered
buffer, and the cached text/effect passes. The full cold pays the same once-time
work plus a full-resolution banded composite; the warm full reuses all of it.

The acceptance target in the issue was 10× for the region; both profiles exceed
it by more than 6×.
