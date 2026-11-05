# Test plan

Coverage references below name implemented std `#[test]` checks using generated
fixtures in `pictura-render`. Tests were written before the scheduling change
and compiled after implementation. The ignored profile was run explicitly;
see verification.md for measurements and limitations.

Select sequential or concurrent preparation explicitly per call. This selector
must neither consult nor toggle the global row-kernel `FORCE_SERIAL` flag, so
overlap tests do not depend on its state in unrelated tests. Hold
`cache_test_guard` around purge/count tests and temporary budget changes.
Every exact cold build-count fixture below uses distinct effect-pass keys;
the shared-key row intentionally accepts the cache's existing duplicate builds.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `pep_overlap` | Purge the cache; composite two styled root layers with distinct pass keys in a dedicated two-worker pool. Synchronize actual build entry with a bounded rendezvous and observe first-band entry. | Both controlled builds enter before either finishes, demonstrating concurrency capability; each distinct key builds once; no band begins until both complete. | `pictura-render::banded_effect_builds_overlap` |
| `pep_nested` | Repeat the controlled synchronized test with distinct-key styled children under visible groups, including siblings within one group; compare with a single-canvas render. | Controlled child build intervals overlap, each resolves once, output bytes agree. | `pictura-render::banded_nested_effect_builds_overlap` |
| `pep_hidden` | Add a hidden styled root and a hidden group containing a visible styled child; purge and composite; compare with those subtrees removed. | No effect build or node resolution for hidden content or descendants; identical pixels. | `pictura-render::banded_hidden_subtrees_build_no_effects` |
| `pep_one_worker` | Run a cold multi-style banded composite in an explicit one-worker pool with a bounded completion check; compare with a single-canvas render. | Completes without waiting for another worker; identical bytes. | `pictura-render::banded_effect_preparation_one_worker` |
| `pep_reuse` | Use the cache test guard; purge; composite two styled layers with distinct pass keys cold, then unchanged warm while both passes fit the budget. Inspect per-key build counts and node-resolution counts. | One build per distinct key cold, no further builds warm; one node resolution per call, independent of band count; identical bytes. | `pictura-render::banded_multiple_effects_reuse_passes` |
| `pep_eviction` | Under the cache test guard, use distinct-key styled layers and lower the retention budget below the fixture's pass size; composite many bands and compare with one canvas; restore the budget. | Each distinct key builds once for the banded call; eviction does not cause a per-band lookup or rebuild; identical pixels. | `pictura-render::banded_effect_passes_survive_eviction` |
| `pep_shared_key` | Under the cache test guard, purge; prepare two visible styled layers deliberately sharing one pass key on a multi-worker pool with enough retention budget. Observe per-layer resolutions, artifact identity, aggregate physical builds, band work, and bounded completion; compare with one canvas. | One resolution per layer; both receive the same published artifact. Accept either a cache hit or duplicate in-flight builds without worker waits; no band lookup/build; exact pixels. | `pictura-render::banded_shared_effect_key_completes_exactly` |
| `pep_invalidate` | Warm two styled layers with distinct pass keys; change one style; compare per-key build counts and output with a subsequent purge-then-composite. | Only the edited layer's key rebuilds while the other remains resident; output equals a fresh composite. | `pictura-render::banded_parallel_effect_edit_invalidates` |
| `pep_exact` | Render a deterministic multi-style stack with raster/vector masks, clipping, and groups, full and over an offset tall region. Repeat with serial plan preparation selected per call and compare to a single canvas. | Every byte matches the relevant whole-canvas slice; stacking order and all blending stay exact. | `pictura-render::banded_parallel_effects_equal_serial_and_whole` |
| `pep_scope` | Observe sibling resolution in ordinary whole-canvas, short-region, native-depth, and render-session plan builders; exercise the short-region effect builder. | No new parallel layer traversal outside banding; short-region pass bounds remain local and unretained. | `pictura-render::non_banded_plans_keep_sequential_resolution` |
| Synthetic performance report | Run the ignored generated two-heavy-layer profile alone; select sequential versus concurrent preparation per call on the same build, pool, and per-effect kernel settings with a purge before each cold sample. Do not add the overlap test's rendezvous to measured work. | Reports actual per-layer intervals and whether they overlap, cold and warm composite timings, plan time, worker count, and observed speed ratio; pixel equality remains asserted. | `pictura-render::banded_effect_preparation_profile` |
| Reported workload performance | Locally profile the supplied workload before/after, with identical worker counts and #1178 status. Compare cold and warm timings, actual build intervals, and peak memory. | Record whether independent builds overlap before bands start; investigate missing overlap or a missed approximate 2x target and report observed speed and memory changes using generic workload descriptions only. | manual: the supplied workload is not a committed CI fixture and performance depends on the profiling machine |

Existing regression coverage to run in addition: `banded_compositing_equals_one_canvas`,
`banded_equals_whole_with_object_layers`, `banded_sub_region_reuses_one_pass`,
the composite-plan suite, effect-cache tests, and source-cache worker reentrancy
and panic recovery tests. None of their baselines should change.

Copy the single manual row into the eventual PR's reviewer checklist without
including input names, paths, or layer names.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `pep_cold_goal` | Run the opt-in profiling acceptance on the workstation with the supplied input after decode and a source-cache purge; compare first and repeated cold results against the original reference outside timing. | First and median cold composite each at most 1000 ms, exact bytes, generic stage/timing/memory output. | `pictura-render::effect_cold_latency` |

Dependency PR #1221 provides scalar blur, distance-field, SIMD, and complete
bevel composite reference tests. Run those tests when integrating it.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `pep_matte_exact` | Compare prepared matte processing with the original quantization and per-pixel masks across generated fields and raster/vector refinements. | Every sample identical. | `pictura-render::prepared_effect_mattes_match_scalar` |
| `pep_replay_exact` | Compare row replay with the former scalar function across clipped passes, colors, blend modes, effect opacity, and knockout tracking. | All component bits and coverage flags match. | `pictura-render::effect_row_replay_matches_scalar` |

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `pep_gaussian_tiles` | Blur multi-plane fixtures at tile edges and with kernel support beyond image dimensions; compare against the original scalar loop. | Every output byte matches. | `pictura-filters::gaussian_tiles_match_scalar_at_block_edges` |

## Review corrections

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `pep_feather_shared` | Prepare plain pixel-layer stacks sharing cold raster or vector feather masks, using one and multiple workers; compare with sequential preparation and a single canvas. | Bounded completion and identical bytes, without requiring effects to trigger the mask build. | `pictura-render::banded_shared_feathered_masks_complete_exactly` |
| `pep_feather_reentrant` | On one Rayon worker, yield from a cold feather-cache build so a queued same-key scalar lookup executes inside it. Bound completion. | No worker wait or thread-local borrow panic; the first published result remains authoritative. | `pictura-render::feather_cache_same_thread_reentry_completes` |
| `pep_feather_stolen` | Force a second worker to enter the same cold feather key from a stolen half while the first build waits for that half. Bound completion. | Both callers complete with identical coverage; first-publication reuse applies while resident, without assuming unrelated cache activity cannot evict the key. | `pictura-render::feather_cache_stolen_half_completes` |
| `pep_reference_required` | Read an absent reference path and an existing generated byte reference through the profiling reader. | Missing reference fails without creating it; existing reference bytes are read unchanged. | `pictura-render::profile_reference_requires_existing_bytes` |
