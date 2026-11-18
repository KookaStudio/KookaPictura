# Test plan

This plan covers `coarse-tile-folding` (#1321): the new capability
`compositing/coarse-tile-folding`.

**Fixtures.** Render tests use `region_level::tests::seeded_doc` at 2101×1301
(odd on both sides, four levels: 2101 → 1051 → 526 → 263) and count calls to
the level-0 source through a wrapper closure, recording the rectangles it was
asked for. The oracle is `build_tile(None, …)`, the direct level-0 build,
which `compositing/tile-pyramid` already checks against a whole rebuild. App
tests use `tiles_tests.rs`'s layered 2101×1301 view with `force_async`, and
run the worker's build inline as `run_tile_job` does.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `ctf_fold_cached` | Cache every level-1 tile; build each level-3 tile with `build_folded` | Each equals its direct build; source called 0 times; level-2 tiles appear in `built` | `pictura-render::the_coarsest_level_folds_from_a_cached_finer_level` |
| `ctf_partial` | Cache the level-1 tiles under the left half of a level-3 tile only; build it | Equals its direct build; every source rect lies outside the cached tiles' level-0 footprint | `pictura-render::only_uncovered_parts_read_level_zero` |
| `ctf_same_job` | Empty cache; one build of the coarsest key followed by every level-1 key (coarse given first) | Every tile equals its direct build; the source's rects add up to the level-1 tiles' footprints only | `pictura-render::one_build_folds_coarser_keys_from_finer_ones` |
| `ctf_stale` | Cache every level-1 tile, edit the document, damage the whole cache, build a level-3 tile from a snapshot | Equals the direct build of the edited document; the source was read | `pictura-render::stale_tiles_are_not_folded_from` |
| `ctf_worker` | Large-document path (`force_async`): land the level-1 tiles through the worker job, change the top layer's pixels behind the display's back, then run the worker job for the coarsest level | Coarsest crop equals the whole rebuild of the unchanged document (so nothing was composited); every level-2 tile is cached | `pictura-app::a_worker_folds_the_coarsest_level_from_the_fit_level` |
| `ctf_halve` | Halve seeded premultiplied regions of odd and even sizes, aligned at the origin, inside the level and at its far edge | Equal to a straightforward per-pixel reference of the rounded in-bounds average | `pictura-render::interior_halving_matches_the_general_rule` |
| `ctf_m12` | `profile-tools.py` M12 at 17000² before (master build) and after (this branch), same host, idle | Total time to the settled fit view recorded against 9433 ms eager and 3598 ms after #1218 | manual: needs the 17000² fixture, a GUI build and minutes of an idle machine; not a CI-sized test |
