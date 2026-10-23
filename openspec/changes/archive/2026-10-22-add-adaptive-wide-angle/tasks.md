# Tasks: add-adaptive-wide-angle

## Tests

- [x] Rust: every `pictura-filters` test named in `test-plan.md` (`distort/adaptive_wide_angle/tests.rs`)
- [x] Qt: new `tst_adaptive_wide_angle` suite with every case named in `test-plan.md`, registered in `cpp/tests/CMakeLists.txt`

## Engine

- [x] `pictura-filters`: `distort::adaptive_wide_angle` (lens model, base projection, constraints, solver, renderer); `Filter::AdaptiveWideAngle`, variant row, props, apply, exports
- [x] Test fixtures: `instances.rs`, `kernel_digests.rs`, oracle mapping, `pictura-render` moving-kernel fixture

## App

- [x] Bridge: `filter_tools/adaptive_wide_angle.rs` (session, EXIF, kept constraint sets), the descriptor row and mapping, `build.rs`
- [x] C++: `awa_canvas.{h,cpp}`, `awa_dialog.{h,cpp}`, `FilterDialogKind::AdaptiveWideAngle`, the depth gate on the menu row, CMake

## Close-out

- [x] `ROADMAP.md`, `docs/dev/doc-code-parity-gaps.md`, `docs/dev/STATE.md`
- [x] `openspec validate --all --strict`, `scripts/check-test-plan.py`, then archive the change
