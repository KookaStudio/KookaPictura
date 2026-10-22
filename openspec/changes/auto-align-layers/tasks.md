# Tasks

## 1. Registration engine

- [ ] 1.1 Add failing `pictura-render` tests for `reposition_only_recovers_translation`, `locked_layer_is_the_reference`, `composition_centre_picks_the_reference`, `perspective_recovers_a_synthetic_quad`, `scene_collage_applies_a_similarity`, `auto_picks_the_better_supported_mode`, `unsupported_modes_are_refused`, `ineligible_layers_are_skipped`, and `auto_align_is_deterministic`.
- [ ] 1.2 Add `document_ops/layer_ops/auto_align/registration.rs` with the grayscale pyramid and deterministic multi-scale NCC translation; make the translation and determinism tests pass.
- [ ] 1.3 Add Harris corners, patch descriptors, deterministic RANSAC with the four-point `solve_homography` minimal solver, an inlier least-squares/DLT refit, and a similarity estimator; make the Perspective and Scene Collage tests pass.
- [ ] 1.4 Add the eligibility/reference rules, the reduced-Auto residual selection, `can_auto_align`, and explicit unsupported-mode errors; make the remaining tests pass.
- [ ] 1.5 Verify `cargo nextest run -p pictura-render` and `cargo clippy -p pictura-render --all-targets -- -D warnings`.

## 2. Atomic document application

- [ ] 2.1 Add `document_ops/layer_ops/auto_align/mod.rs` that validates eligibility and applies computed placements to a cloned `Document`, swapping it in only after every placement succeeds; add `mid_apply_refusal_leaves_document_unchanged` and a cancel/no-placement test.
- [ ] 2.2 Re-export the auto-align API from `pictura-render` and verify the crate tests pass.

## 3. Dialog, bridge, and cancel

- [ ] 3.1 Add failing `tst_auto_align` cases `menuEnabledWithTwoEligibleLayers`, `unsupportedModesDisabled`, `menuDisabledWithoutTwoEligibleLayers`, `ineligibleKindsSkipped`, `cancelRecordsNothing`, and `oneHistoryStateAndUndo`; register the suite in `crates/pictura-app/cpp/tests/CMakeLists.txt`.
- [ ] 3.2 Add `Edit > Auto-Align Layers…`, the command id, `AutoAlignDialog` (Auto default, unsupported modes disabled, Lens Correction in its own disabled option group), and enablement; add the new files to `CMakeLists.txt`; make the dialog cases pass.
- [ ] 3.3 Add the auto-align job state, progress/cancel signal, and apply bridge as free functions in a new module (do not grow `cxxqt_object.rs`); apply one `Auto-Align Layers` history state; make the history/undo and cancel cases pass.
- [ ] 3.4 Verify `ctest --test-dir build -R '^tst_auto_align$' --output-on-failure`.

## 4. Integration and records

- [ ] 4.1 Update `docs/dev/doc-code-parity-gaps.md` (LAY-4), `docs/02-ui-ux/menus.md` (Edit tree), `ROADMAP.md` (tick only the #582 half of the #582/#583 line), and `docs/dev/STATE.md`; open/record the unsupported-mode and reduced-Auto follow-up. Carry `TASK-ALLOWS-DOCS`.
- [ ] 4.2 Run `openspec validate --all --strict`, `python3 scripts/check-test-plan.py`, and `bash scripts/verify-full.sh`; fix every failure.
- [ ] 4.3 Archive the change with `openspec archive auto-align-layers` and verify `scripts/check-openspec-archived.sh` passes.
