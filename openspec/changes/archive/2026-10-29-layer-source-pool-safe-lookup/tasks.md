# Tasks

## 1. Regression test

- [x] 1.1 Add `pictura-render::same_key_lookup_stolen_mid_build_does_not_deadlock`: on a one-thread pool, a build that yields runs a queued lookup of the same key, under a 30 s watchdog.
- [x] 1.2 Add `pictura-render::same_key_lookup_in_a_stolen_half_of_the_build_does_not_deadlock`: on a two-thread pool, the other worker steals half of a build and looks up the same key, under a 30 s watchdog.
- [x] 1.3 Add `pictura-render::panicking_build_releases_its_claim`: a panicked build gives its claim back.
- [x] 1.4 Run both tests against the unfixed `lookup` and confirm each fails on its watchdog.

## 2. Fix

- [x] 2.1 Replace the cache's `OnceLock` cell with a `Slot` (empty / building / ready): a lookup on a rayon worker never waits on an in-flight build and builds its own copy; a lookup outside the pool waits; the first published artifact wins; a panicked build releases its claim.
- [x] 2.2 In `composite_region::composite_banded`, composite band 0 before the remaining bands run in parallel, so each whole-layer pass is built once.

## 3. Verify

- [x] 3.1 `cargo fmt --all`, `cargo clippy -p pictura-render --all-targets -- -D warnings`, `cargo nextest run -p pictura-render`.
- [x] 3.2 Run `huge_inner_glow_is_bounded` and the two regression tests alone 20 times each under `timeout 120`.
- [x] 3.3 `python3 scripts/check-test-plan.py`, `openspec validate --all --strict`, then archive the change.
