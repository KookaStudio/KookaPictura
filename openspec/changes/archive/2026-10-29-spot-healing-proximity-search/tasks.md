# Tasks

## 1. Engine

- [x] 1.1 Add `healing/proximity.rs`: the chamfer distance transform, the margin from the brush core's thickness, and the source search.
- [x] 1.2 Grow the hole by the margin in `Work::new` (a weight per pixel); apply it to Proximity Match and Create Texture; return the grown rectangle.
- [x] 1.3 Factor `poisson_fill` out of `clone_region`; Proximity Match uses it with the searched offset and falls back to the Laplace fill.
- [x] 1.4 Add the engine tests from the test plan; restate the outside-pixels and rectangle assertions for the margin.

## 2. App

- [x] 2.1 Add `tst_retouch_tools::spotHealingClickRemovesADot`.

## 3. Gates

- [x] 3.1 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`.
- [x] 3.2 `cargo nextest run -p pictura-paint`; build and run `tst_retouch_tools`.
- [x] 3.3 `python3 scripts/check-test-plan.py`, `openspec validate --all --strict`.
- [x] 3.4 Archive with `openspec archive spot-healing-proximity-search`.
