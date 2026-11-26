# Tasks: knockout-shallow-base

## 1. Compositor

- [x] 1.1 Extract the knockout entry points (`composite_layers`, `knockout_base`, `has_knockout`, `composite_layer`, `composite_knockout`) into `crates/pictura-render/src/composite_knockout.rs`; make `composite_layer_inner` `pub(crate)` and keep `composite.rs` at or under its 1217 allowlist ceiling.
- [x] 1.2 Replace the single knockout `base` with `deep: Option<&Canvas>` and `shallow: Option<&Canvas>`; select by mode (`Deep → deep.or(shallow)`, `Shallow → shallow`, `None → none`).
- [x] 1.3 Root: pass the document-background canvas as both bases. Pass-through group: forward `deep`, pass a snapshot of the running canvas at group entry as `shallow` (only when `has_knockout`). Isolated group: forward `deep = None` and `shallow` = the group's empty initial backdrop.
- [x] 1.4 Unit/composite tests: Shallow inside a pass-through group reveals the intervening layer below the group; the same stack with Deep reveals the document background; root Shallow == Deep.

## 2. Fixtures and oracle

- [x] 2.1 Add `knockout_shallow_group` to `scripts/generate-fixtures.py`: red Background, an opaque intervening layer, then a pass-through group of green and a half-fill blue with `Tag.KNOCKOUT_SETTING = 1`; regenerate `crates/pictura-codec/tests/fixtures/knockout_shallow_group.psd`.
- [x] 2.2 Add the fixture to `scripts/psd_knockout_reference.py` and generate `crates/pictura-render/tests/fixtures/knockout_shallow_group.rgba` with `psd-tools` 1.19.
- [x] 2.3 Add a test to `crates/pictura-render/tests/oracle/knockout_oracle.rs` that decodes the fixture (asserting the group is `PassThrough` and the child is `Shallow`), composites it, and diffs against the reference at the documented tolerance; self-skip when `psd_tools` is unavailable.
- [x] 2.4 Pin `psd-tools>=1.19` in `.github/workflows/ci.yml`.

## 3. Verification

- [x] 3.1 `cargo nextest run --workspace` green, including the existing Deep root/group/isolated knockout oracles.
- [x] 3.2 `cargo test -p pictura-render --test oracle knockout_oracle::` green (all four fixtures).
- [x] 3.3 `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `bash scripts/check-file-size.sh`, `openspec validate knockout-shallow-base --strict`.
