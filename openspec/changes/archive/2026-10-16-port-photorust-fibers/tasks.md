# Tasks: port-photorust-fibers

## 1. Engine

- [x] 1.1 Port photorust's `fibers` into `render/fibers.rs` on the planar buffer; `color_a` → foreground, `color_b` → background, seed folded to `u32`.
- [x] 1.2 Narrow the ranges to Variance `0..=64`, Strength `1..=64`; Variance 0 is the even blend.
- [x] 1.3 Leave Clouds and Difference Clouds unchanged.

## 2. Verification

- [x] 2.1 Port photorust's Fibers property tests: determinism per seed, colour bounds and alpha, hard strand edges, vertical run, variance breaks up, strength stretches, both colours reached, parameter rejection.
- [x] 2.2 `docs/06-filters/render-filters.md` updated (`TASK-ALLOWS-DOCS`).
- [x] 2.3 `bash scripts/verify-full.sh`; `openspec validate --all --strict`.
