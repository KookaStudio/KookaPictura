# Test Plan

One row per scenario in the delta specs. All rows are automated.

## compositing/layer-source-cache

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| lsc_steal | On a one-thread rayon pool, `join` two lookups of one key; the first lookup's build calls `rayon::yield_now`, which runs the queued second lookup on top of it. A 30 s watchdog fails the test if the pool wedges. | The yield ran the second lookup, both lookups return, and both hold the same `Arc`. Before the fix this deadlocks in `OnceLock::initialize` and the watchdog fails. | `pictura-render::same_key_lookup_stolen_mid_build_does_not_deadlock` |
| lsc_steal_cross | On a two-thread rayon pool, the build of a key `join`s two halves: one spins until the other finishes, so the idle worker must steal the other, which looks up the same key. A 30 s watchdog fails the test if the pool wedges. | The halves ran on different workers, both lookups return, and both hold the same `Arc`. Before the fix the stolen lookup waits in `OnceLock::initialize` on the build that waits for it, and the watchdog fails. | `pictura-render::same_key_lookup_in_a_stolen_half_of_the_build_does_not_deadlock` |
| lsc_dedupe | Eight scoped OS threads (not pool workers) look up one key whose build sleeps 10 ms and counts itself. | One build. | `pictura-render::parallel_build_of_one_key_runs_once` |
| lsc_panic | Outside the pool, a lookup whose build panics is caught; the same key is then looked up again, under a 30 s watchdog. | The first lookup panics, the second returns the built artifact instead of waiting on the abandoned build. | `pictura-render::panicking_build_releases_its_claim` |
| lsc_huge_glow | Composite a 512×512 document with a `1e30` inner glow (banded) and compare outside-layer pixels with the plain composite; run it alone 20 times, each under `timeout 120`. | Every run passes in under 15 s; pixels outside the layer match. | `pictura-render::huge_inner_glow_is_bounded` |
| lsc_band_once | Purge the cache, composite a tall sub-region of a styled document in bands, read the layer's pass build count, then compare with the whole composite. | One build, and the banded output equals the whole composite. | `pictura-render::banded_sub_region_reuses_one_pass` |
