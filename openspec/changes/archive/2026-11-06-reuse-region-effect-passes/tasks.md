# Tasks

## 1. Ready-pass probe and cache behavior

- [x] 1.1 Write `ready_pass_probe_misses_unavailable_entries`, `region_does_not_wait_for_inflight_effect_pass`, and `ready_pass_probe_preserves_cache_accounting` in layer-source tests, splitting a focused test module if needed for file-size caps. Cover absent, Empty, Building, evicted and over-budget slots, metrics, LRU and byte accounting; completion is test source for every listed assertion, with execution deferred until all code is finished.
- [x] 1.2 Implement a private ready-only probe in `layer_source/mod.rs` using existing slot and cache types. Verify through 1.1 tests after the final build that misses neither insert, wait for builds, nor publish, and that hits safely share retained artifacts and update accounting as designed.

## 2. Region reuse and freshness

- [x] 2.1 Write `warm_short_region_reuses_ready_effect_pass`, `cold_short_region_builds_transient_effect_pass`, `full_composite_reuses_ready_effect_pass`, and `region_effect_families_match_cold_and_warm_slices`; add unit-test instrumentation at `build_layer_passes` that counts transient and cached builds by content key, since existing lookup-only counts are insufficient. Assert shared pass identity, no warm all-build-count increase, cold bounded output, full reuse, and exact CPU slices across effects and offset rectangles. Completion is assertion-bearing test source before task 2.3; execute after all code is finished.
- [x] 2.2 Write `edited_styled_region_rebuilds_after_warm`, `cold_overlay_region_sources_stay_bounded`, and `pattern_region_rebuilds_after_pattern_edit`; assert transient bounds, paint/mask freshness after warming, unchanged-layer reuse, bounded overlay buffers and referenced-pattern edits. Completion is assertion-bearing test source before task 2.3; retain existing banded, transient-retention and per-family tests.
- [x] 2.3 Route non-pattern short-region requests through the probe with the existing whole-pass key; return the shared pass on a hit and preserve current transient construction on a miss. Keep no-passes, pattern-driven, full and banded behavior intact. Verify with all task 2.1/2.2 tests and existing cache/region suites after all code is finished; no pixel baseline changes.

## 3. Bevel performance evidence

- [x] 3.1 Write ignored `warm_bevel_region_replay_profile` in `tests/source_cache_profile.rs` using design.md's fixed generated fixture, regions and dedicated one-worker rayon pool. After three warm-up rounds, measure nine alternating-order rounds of the adjacent 128-row pair versus its exact 256-row union, assert the ratio of median summed-pair time to median union time is at most 4.0, and compare all outputs to full slices outside timing. Assert four warm source-cache hits per short call for this fixture, including two pass hits, zero warm source misses and zero effect decodes; report these as reuse signals, with direct zero-build assertions in task 2.1's unit tests. Report cold short and warm full times too. Finish this assertion-bearing test source before any build.

## 4. Integrated verification and shipping

- [x] 4.1 After all code and tests are finished, build once using at most 80% of available CPUs; fix errors and rebuild only after code changes. Run the planned Rust tests, existing region/layer-effect/source-cache suites and relevant GPU parity checks without changing tolerances; verify test-plan.md references resolve with `python3 scripts/check-test-plan.py`.
- [x] 4.2 Run the ignored bevel profile alone with the exact command in design.md. Record cold/warm timings, the single-worker pair-to-union median ratio, cache reuse signals and task 2.1's direct zero-build evidence. Completion requires the profile's ratio assertion (at most 4.0), cache assertions and exact-pixel comparisons to pass; investigate failures instead of treating printed timings as acceptance.
- [x] 4.3 Update ROADMAP.md for the shipped issue as required by the repository; verify issue linkage and scope, then run applicable repository gates and `openspec validate --all --strict`. Change docs/dev/STATE.md only if separately authorized; do not broaden docs scope implicitly.
- [x] 4.4 Archive `reuse-region-effect-passes` in the shipping PR after implementation and checks pass. Verify strict validation, the archived test plan, and the zero-open guard; the PR has no manual test-plan rows because every row is automated.
