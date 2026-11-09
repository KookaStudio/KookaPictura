# Tasks

## 1. Engine geometry

- [x] 1.1 Update `crates/pictura-select/src/geometry.rs`: Fixed Ratio fits the non-degenerate drag box with one-axis handling, and Fixed Size's top-left uses `to`; verify `cargo test -p pictura-select --lib geometry::` passes.
- [x] 1.2 Add the Rust tests `marquee_rect_fixed_ratio_fits_drag`, `marquee_rect_fixed_ratio_narrow`, `marquee_rect_fixed_ratio_one_axis`, `marquee_rect_fixed_ratio_reverse`, `marquee_rect_fixed_ratio_ignores_modifiers`, `marquee_rect_fixed_size_follows_the_pointer`, and `marquee_rect_fixed_size_ignores_modifiers`; verify the same run passes.

## 2. Alt and Polygonal close radius

- [x] 2.1 Change `MarqueeToolHandler::trackAltEdge` so any Alt edge during the drag mirrors, and drop `altArmed_`; verify `cargo build -p pictura_app` and `ctest --test-dir build -R '^tst_selection_alt$'` pass.
- [x] 2.2 Change the Polygonal close/double-click radius to `kPolygonCloseRadius / max(zoom, 1e-6)`; update the `selection_geometry.h` comment; verify the same build and `ctest --test-dir build -R '^tst_selection_preview$'`.
- [x] 2.3 Add `tst_selection_alt::altPressedDuringDragMirrors` and `tst_selection_alt::altWithNoSelectionDrawsFromTheCentre`; verify the suite passes.
- [x] 2.4 Retire the `tsc_fixed_size` self-test check (244) now covered by `tst_selection_preview::fixedSizeFollowsTheCursor`, and refresh the Alt comments; verify `./build/pictura --headless --self-test`.

## 3. Elliptical options

- [x] 3.1 Give `ToolId::EllipticalMarquee` its own `SelectionOptions` in `tools.cpp`; verify `ctest --test-dir build -R '^tst_options_bars_parity$'` passes.
- [x] 3.2 Persist the elliptical group as `ellipticalSelection` in `tool_options_json.cpp` (save, load, patch); verify `ctest --test-dir build -R '^tst_tool_options_persistence$'` passes.
- [x] 3.3 Add `tst_selection_preview::fixedRatioFitsTheCursorBox`, `fixedSizeFollowsTheCursor`, `fixedSizeClipsAtTheCanvasEdge`, and `ellipticalStyleIsIndependent`; verify `tst_selection_preview` passes.
- [x] 3.4 Add `tst_tool_options_persistence::ellipticalOptionsRoundTrip`; verify the suite passes.

## 4. Docs and integration

- [x] 4.1 Update `docs/03-tools/marquee-selection.md`, `docs/03-tools/lasso-selection.md`, `docs/08-selection/selection-tools-overview.md`, and `docs/dev/STATE.md`; verify the wording matches the new behavior and the docs commit carries `TASK-ALLOWS-DOCS`.
- [x] 4.2 Run `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `openspec validate --all --strict`, and `TASK_ALLOWS_DOCS=1 bash scripts/verify-fast.sh`; verify they pass.
- [x] 4.3 Run `openspec archive selection-marquee-feedback`; verify the change moves under `openspec/changes/archive/`.
