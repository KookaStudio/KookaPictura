# Tasks: add-vanishing-point-retouch

## Tests

- [x] Rust: every `pictura-filters` test named in `test-plan.md` (`distort/vanishing_point/tests_retouch.rs`)
- [x] Qt: `tst_vanishing_point` updated for the nine tools; new `tst_vanishing_point_retouch`, registered in `cpp/tests/CMakeLists.txt`

## Engine

- [x] `space` (flat editing space), `canvas` (working copy, pixel undo), `heal`, `float` (floating selection, marquee), `stroke` (Stamp, Brush), `tools`, `view` (preview, Eyedropper)
- [x] `VpEdit` carries the changed-pixel patch; the kernel writes it before the grid lines

## App

- [x] Bridge: `filter_tools/vanishing_point_retouch.rs`, the session's canvas and backdrop, the commit patch, `clipboard_pixels`
- [x] C++: `vp_options.{h,cpp}`, the canvas's dirty-rect preview and tool gestures, the dialog's tools and shortcuts, CMake

## Close-out

- [x] `ROADMAP.md`, `docs/dev/doc-code-parity-gaps.md`, `docs/dev/STATE.md`
- [x] `openspec validate --all --strict`, `scripts/check-test-plan.py`, then archive the change
