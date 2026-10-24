# Tasks: add-vanishing-point-planes

## Tests

- [x] Rust: every `pictura-filters` test named in `test-plan.md` (`distort/vanishing_point/tests.rs`) and the `pictura-app` mapping and gate tests
- [x] Qt: new `tst_vanishing_point` suite with every case named in `test-plan.md`, registered in `cpp/tests/CMakeLists.txt`

## Engine

- [x] `pictura-core`: `homography::Homography`; `pictura-render` and the app transform preview use it
- [x] `pictura-filters`: `distort::vanishing_point` (camera and lift, planes and related planes, editor, Render Grids kernel); `Filter::VanishingPoint`, variant row, props, apply, exports
- [x] Test fixtures: `instances.rs`, `kernel_digests.rs`, oracle mapping, `pictura-render` moving-kernel fixture

## App

- [x] Bridge: `filter_tools/vanishing_point.rs` (session, kept edits), the shared 8-bit gate, planes on the document session, the descriptor row and mapping, `build.rs`
- [x] C++: `vp_canvas.{h,cpp}`, `vp_dialog.{h,cpp}`, `FilterDialogKind::VanishingPoint`, the depth gate on the menu row, CMake, the `tool.createplane` icon

## Close-out

- [x] `ROADMAP.md`, `docs/dev/doc-code-parity-gaps.md`, `docs/dev/STATE.md`
- [x] `openspec validate --all --strict`, `scripts/check-test-plan.py`, then archive the change
