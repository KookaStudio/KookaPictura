# Tasks

## 1. Tests

- [x] 1.1 Add `penModeBar`, `penShapeMode`, `penPathMode`, and `penPixelsMode` to `crates/pictura-app/cpp/tests/tst_pen_tools.cpp`; keep the existing Pen checks on the Work Path by selecting Path mode.
- [x] 1.2 Select Path mode in `tst_path_selection_tools` and `tst_type_on_path` so their Pen-drawn fixtures still edit the Work Path.

## 2. Pen modes

- [x] 2.1 Add `PenOptions.mode` (0 Shape / 1 Path / 2 Pixels, default Shape), clamp it, and persist it.
- [x] 2.2 Add the `optionsPenMode` combo to the Pen and Freeform Pen pages and sync the two pages.
- [x] 2.3 Land a finished path: Shape adds a shape layer from the Work Path, Path saves a copy, Pixels strokes the active layer.

## 3. Close-out

- [x] 3.1 Run the targeted Qt suites and `scripts/check-test-plan.py`.
- [x] 3.2 Run `bash scripts/verify-fast.sh` and fix any fmt, clippy, file-size, guard, or OpenSpec failure.
- [x] 3.3 Run `openspec validate --all --strict`, then `openspec archive pen-shape-path-pixels`; verify `python3 scripts/check-openspec-archived.sh` passes.
