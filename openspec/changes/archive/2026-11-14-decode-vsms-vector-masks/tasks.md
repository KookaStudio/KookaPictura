# Tasks

Tests and fixtures are written before the production edits; compilation and test
execution happen in group 6, following the compile-last rule.

## 1. Test plan and fixture

- [x] 1.1 Write `openspec/changes/decode-vsms-vector-masks/test-plan.md` with one row per scenario tag in the five deltas (codec, layer-compositing, clipping, vector-masks, layer-management), each row naming its planned test (`pictura-codec::<fn>` or `pictura-render::<fn>`) or a justified `manual:` reason; verify every tag is covered and `python3 scripts/check-test-plan.py` reports only the not-yet-written test names.
- [x] 1.2 Add a `vector_mask_vsms()` builder to `scripts/generate-fixtures.py`: a base pixel layer plus a `vscg` solid-fill shape whose closed-rectangle mask is written under `Tag.VECTOR_MASK_SETTING2` and which has no `vmsk`; no clipped layer belongs in the fixture. Register it in `FIXTURES` and document it in `crates/pictura-codec/tests/fixtures/README.md`; verify `python3 scripts/generate-fixtures.py` writes `vector_mask_vsms.psd`, leaves every existing fixture byte-identical, and psd-tools reports `has_vector_mask()` true with one closed path.

## 2. Codec decode

- [x] 2.1 Add codec unit tests in `crates/pictura-codec/src/vector_mask.rs`: `a_vsms_block_decodes_when_vmsk_is_absent`, `vmsk_wins_when_both_blocks_exist`, `a_malformed_vsms_block_is_unset_and_preserved`, `a_valid_vmsk_wins_over_a_malformed_vsms`, `a_malformed_vmsk_does_not_fall_back_to_vsms`, and `a_vsms_block_round_trips_verbatim`; verify each builds the version-3 body under `b"vsms"`/`b"vmsk"` and asserts the derived view, precedence, and preserved raw blocks.
- [x] 2.2 Extend `crates/pictura-codec/tests/vector_mask_oracle.rs` so its psd-tools script falls back to `Tag.VECTOR_MASK_SETTING2`, and add the non-skipping `vsms_fixture_decodes` and `vsms_fixture_round_trips` plus the oracle `vsms_fixture_matches_psd_tools`; verify only the oracle self-skips without psd-tools and the fixture read/round trip asserts the authored rectangle and byte-identical `vsms` block.
- [x] 2.3 Change `resolve_vector_masks` in `crates/pictura-codec/src/vector_mask.rs` to select `b"vmsk"` by presence and otherwise `b"vsms"`, leaving the decode, flags, placeholder density/feather merge, and raw-block preservation untouched; update the derivation comments to say `vmsk`/`vsms`. Verify the tests from 2.1–2.2 pass once the final build runs.

## 3. Vector-mask authoring on `vsms`

- [x] 3.1 Add authoring tests in `crates/pictura-render/src/document_ops/layer_ops/vector_masks.rs`: `a_vsms_mask_deletes_and_round_trips`, `a_vsms_mask_flags_and_link_edit`, `a_vsms_mask_rasterizes_without_a_reload_change`, and `vsms_read_predicates_track_the_effective_block`; verify delete/rasterize remove every vector-mask block, flags and link patch only the `vsms` word, and a write/re-read preserves the result.
- [x] 3.2 Add one effective-block helper in `vector_masks.rs` and use it in `set_vmsk_block`, `patch_flag`, `sync_vector_mask_block`, `delete_vector_mask`, `rasterize_vector_mask`, and `vector_mask_linked`; delete and rasterize remove both `vmsk` and `vsms` blocks, and `rotate_in.rs` inherits the sync fix. Verify the tests from 3.1 pass and the existing `vmsk` authoring tests still pass.

## 4. Shape identity on `vsms`

- [x] 4.1 Add the render test `a_vsms_shape_layer_is_recognized_and_rasterizes`; verify a `vsms`-only fill shape reports `is_shape_layer`, exposes its outline through `layer_shape_paths`, succeeds through `rasterize_shape`, leaves no `vmsk`/`vsms`/`vscg` block, and keeps the composite unchanged after a save and reload.
- [x] 4.2 Make `is_shape_layer` and `layer_shape_paths` accept the effective vector-mask key, make `set_layer_shape_paths` write it, and make `rasterize_shape` drop both keys; verify the test from 4.1 and the existing shape-rasterize tests pass once the final build runs.

## 5. Render and clipping coverage

- [x] 5.1 Add render tests in `crates/pictura-render/src/tests/vector_mask.rs`: `a_vsms_shape_fill_renders_inside_its_path` and `an_inverted_vsms_mask_hides_its_inside`; verify they read `vector_mask_vsms.psd` and assert fill inside/backdrop outside for both the normal and inverted flags.
- [x] 5.2 Add clipping tests in `crates/pictura-render/src/tests/clipping.rs`: `a_clipped_layer_follows_a_vsms_shape_base`, `a_clipped_layer_follows_a_vmsk_shape_base`, and `a_disabled_vsms_mask_leaves_the_base_alpha`; verify the `vsms` and `vmsk` composites are byte-equal and the disabled case matches the fill base's rendered alpha. Append the full-canvas clipped layer in memory; the fixture has none.
- [x] 5.3 Verify the compositor needs no production edit: confirm both clipping coverage and the base render already sample `Layer.vector_mask`, and record that the behavior change is the codec key fallback plus the effective-block document ops.

## 6. Integrated verification and delivery

- [x] 6.1 After all source and test edits are complete, run `cargo fmt --all` and build once with at most 80% of available CPUs; verify the workspace compiles and rebuild only when a compiler fix changes code.
- [x] 6.2 Run `cargo test -p pictura-codec vector_mask`, `cargo test -p pictura-render vector_mask`, `cargo test -p pictura-render clipping`, and `cargo test -p pictura-render rasterize`, then `python3 scripts/check-test-plan.py` and `openspec validate --all --strict`; verify every plan row names an existing test, CI installs psd-tools so the oracle does not count a local self-skip as acceptance, and the full `bash scripts/verify-fast.sh` gate passes.
- [ ] 6.3 Record the manual acceptance row from `test-plan.md` in the implementation PR body against the reported reference document; confirm the affected shape layer renders its letterforms and the clipped layer is confined to them; keep the reference document out of committed fixtures. The manual row is recorded in `test-plan.md` and belongs in the implementation PR's reviewer checklist.
- [x] 6.4 Archive `decode-vsms-vector-masks` after validation, confirm `openspec/changes/` holds only `archive/`, and reference both fixed issues (#1284, #1114) in the PR; verify no open change remains and `ROADMAP.md` needs no edit because no roadmap item changes.
