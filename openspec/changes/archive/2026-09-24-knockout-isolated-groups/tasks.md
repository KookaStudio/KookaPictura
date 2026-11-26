# Tasks: knockout-isolated-groups

## 1. CPU compositor

- [x] 1.1 In `composite_layer_inner`'s isolated branch, when `has_knockout(layer)` build an empty `Canvas::new_region(...)` (`ko_inner`, the group's initial backdrop) and pass `Some(&ko_inner)` as the children's base; otherwise `None`.
- [x] 1.2 Keep the pass-through branch and the top-level path unchanged; the innermost enclosing group decides the base.
- [x] 1.3 Update the `ponytail:` ceiling comment (isolated groups now handled; only nested-shallow/clipping/Transparency-Shapes remain).

## 2. Fixture and reference

- [x] 2.1 `scripts/generate-fixtures.py`: add `knockout_isolated_group()` — red Background, a yellow layer, then an isolated group (explicit non-pass blend, `BlendMode.NORMAL`) containing green and half-fill blue with `knko = Deep`; register as `knockout_isolated_group.psd`.
- [x] 2.2 Verify `read_psd` decodes the group as a non-`PassThrough` blend and the child as `Knockout::Deep`; the psd-tools composite must be `(126,127,128)` at a covered pixel.
- [x] 2.3 `python3 scripts/psd_knockout_reference.py gen --out crates/pictura-render/tests/fixtures/knockout_isolated_group.rgba`; commit it.

## 3. Tests

- [x] 3.1 `crates/pictura-render/tests/oracle/knockout_oracle.rs`: add the isolated-group test (self-skip, tolerance 1, assert the group is not `PassThrough` and the child `Deep`, compare, assert the layer below the group contributes).
- [x] 3.2 Unit tests in `crates/pictura-render/src/tests/composite.rs`: isolated-group knockout punches the group's green through; a knockout-free isolated group is byte-identical.
- [x] 3.3 README notes for the new fixture/reference.

## 4. Gates

- [x] 4.1 `cargo nextest run -p pictura-render -p pictura-codec`, `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `openspec validate knockout-isolated-groups --strict`, `bash scripts/check-file-size.sh` (composite.rs must stay ≤ its recorded ceiling — trim comments, do NOT raise the allowlist).
- [x] 4.2 If the isolated oracle disagrees beyond tolerance 1, STOP and report the per-pixel diff.
