# Tasks

## 1. Tests

- [x] 1.1 Add `light_bokeh_does_not_rim_a_blurred_edge` in
  `crates/pictura-filters/src/blur/field/tests.rs`: a dark subject on a bright
  background keeps the unlit blur's edge ratio, with no brightened rim.
- [x] 1.2 Add `light_range_gate_is_continuous` in the same module: a tonal ramp
  across the upper bound gains no step.

## 2. Engine

- [x] 2.1 Split the Blur Effects pass (`apply_bokeh`) from the Noise pass
  (`apply_noise`).
- [x] 2.2 Apply the bokeh to the source inside `pyramid_blur`, before the
  pyramid, gated by the field weight.
- [x] 2.3 Soften the Light Range gate with a smoothstep taper at both bounds.
- [x] 2.4 Route Field, Iris, Tilt-Shift, Path, Spin, and the combined gallery
  through the split passes.

## 3. Gates

- [x] 3.1 Update the Field, Iris, Tilt-Shift, and combined gallery kernel
  digests for the intentional render-order change.
- [x] 3.2 `cargo nextest run -p pictura-filters`, then
  `cargo clippy --workspace --all-targets -- -D warnings` and
  `cargo fmt --all --check`.
- [x] 3.3 `python3 scripts/check-test-plan.py`, `openspec validate --all --strict`.
- [x] 3.4 Archive with `openspec archive blur-gallery-soft-edges`.
