# Tasks: add-lens-correction

## Tests

- [x] Rust: every `pictura-filters` and `pictura-app` test named in `test-plan.md`
- [x] Qt: new `tst_lens_correction` suite with every case named in `test-plan.md`, registered in `cpp/tests/CMakeLists.txt`; `tst_filter_menu` knows the new dialog kind

## Engine

- [x] `pictura-filters`: `distort::lens_correction` (map, sampler, edge fill, profiles); `Filter::LensCorrection`, variant row, props, apply, exports
- [x] Test fixtures: `instances.rs`, `kernel_digests.rs`, oracle mapping, `pictura-render` moving-kernel fixture

## App

- [x] Bridge: `filter_tools/lens_correction.rs`, the depth gate, the descriptor row and mapping, `build.rs`
- [x] C++: `lens_correction_canvas.*`, `lens_correction_panels.*`, `lens_correction_dialog.*`, `FilterDialogKind::LensCorrection`, the menu gate, CMake
- [x] Catalogue: `Shift+Ctrl+R`, implemented, golden

## Close-out

- [x] `ROADMAP.md`, `docs/dev/doc-code-parity-gaps.md`, `docs/06-filters/lens-correction.md`
- [x] Follow-up issues for the deferred parts
- [x] `openspec validate --all --strict`, `scripts/check-test-plan.py`, then archive the change
