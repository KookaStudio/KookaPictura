# Tasks: knockout-groups

## 1. CPU compositor

- [x] 1.1 Add `base: Option<&Canvas>` to `composite_layer_inner`; update its call sites (`knockout_base` and `composite_knockout` pass `None`).
- [x] 1.2 In `composite_layer_inner`'s pass-through branch, recurse children with the incoming `base` instead of `None`. The isolated branch keeps `None`.
- [x] 1.3 Replace `knockout_base`'s top-level presence check with a recursive `has_knockout(layer)` walk over `layer.knockout` and `layer.children`.
- [x] 1.4 `ponytail:` comments for the isolated-group and nested-shallow ceilings.

## 2. Fixture and reference

- [x] 2.1 `scripts/generate-fixtures.py`: add a `knockout_group()` builder — red Background, a pass-through group containing green and a half-fill blue with `knko = Deep`; register it as `knockout_group.psd`.
- [x] 2.2 Confirm the written group is pass-through in both psd-tools and `read_psd` (`BlendMode::PassThrough`); if psd-tools defaults the group to an isolated mode, set its `blend_mode` to pass-through explicitly.
- [x] 2.3 `python3 scripts/psd_knockout_reference.py gen --out crates/pictura-render/tests/fixtures/knockout_group.rgba`; commit the reference.

## 3. Tests

- [x] 3.1 `crates/pictura-render/tests/oracle/knockout_oracle.rs`: add a test for the group fixture (self-skip guard, tolerance 1, assert `PassThrough` + `Deep`, compare, assert green punched to 0).
- [x] 3.2 Unit tests in `crates/pictura-render/src/tests/composite.rs`: a pass-through-group knockout punches green through; an isolated-group knockout leaves the output byte-identical to no knockout.
- [x] 3.3 README notes for the new fixture/reference.

## 4. Gates

- [x] 4.1 `cargo nextest run -p pictura-render -p pictura-codec`, `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `openspec validate knockout-groups --strict`.
- [x] 4.2 If the group oracle disagrees beyond tolerance 1, STOP and report the per-pixel diff (the group semantics are the whole point of the change).
