# Tasks

## 1. Parameterised filter mapping

- [x] 1.1 Add `filter_from_kind_params(kind: &str, params: &[f64]) -> Option<pictura_filters::Filter>` in `crates/pictura-app/src/cxxqt_object/filter_map.rs`, with each kind's fixed slot order and an empty slice producing the documented defaults; make `filter_from_kind` delegate to it with `&[]`. Verify with the `filter_map.rs` guard test: every supported kind builds its default from an empty slice and rejects a wrong-length slice.
- [x] 1.2 Extend the existing mapping guard test to cover all newly parameterised kinds (not just the six from `port-missing-filter-kernels`) and assert their slot arities. Verify `cargo nextest run -p pictura-app` passes.
- [x] 1.3 Add a `Source:` provenance header to the parameterised mapping, mirroring the prior photorust ports. Verify by inspecting the file header.

## 2. Preview, commit, and last-filter state

- [x] 2.1 Add the `PictureView` bridge methods `apply_filter_params(kind, params)`, `filter_preview(kind, params)`, `filter_preview_cancel()`, and `filter_last()`, declared in `cxxqt_object.rs` and implemented in `impl_filters.rs`. Preview snapshots the active layer before the first parameter change, filters a clone, swaps the result into the document, and refreshes the region without `record`; cancel restores the snapshot; commit records exactly one `"Filter"` state and stores `(kind, params)` as the last filter. Verify with Rust tests in `tests_impl.rs`: wrong-arity refusal, preview leaves the document base and history unchanged, cancel is bit-identical, commit records one state and populates the last filter.
- [x] 2.2 Gate preview and commit on the active layer being an unlocked normal pixel layer, reusing the existing active-layer resolver and visible-layer check. Verify a Rust test that a locked or non-pixel active layer refuses preview and commit without modifying pixels.

## 3. Generic filter parameter dialog

- [x] 3.1 Port photorust `shell/src/dialogs/FilterPreviewDialog.{h,cpp}` (with `FilterPreviewPane`, `BlurCenterWidget`, `PlacementPreviewWidget`, `DistortGridWidget`, `ShearCurveWidget`, and the kernel grid) into `crates/pictura-app/cpp/filter_preview_dialog.{h,cpp}`, split across translation units to stay under the file-size cap; standard `QDialog` chrome. Verify the dialog compiles in the CMake build and each file is under its cap.
- [x] 3.2 Add every new `.cpp`/`.h` to `CMakeLists.txt` explicitly and wire the dialog's Preview checkbox, zoom, OK/Cancel to the bridge preview methods. Verify the build succeeds and a manual/offscreen open of a Gaussian Blur dialog renders.
- [x] 3.3 Add the Qt Test suite `crates/pictura-app/cpp/tests/tst_filter_menu.cpp` covering dialog construction and parameter collection for a representative kind of each control type (slider, angle, choice, radio, checkbox, colour, shear curve, distort grid, kernel grid, Blur Center, placement). Register it in `crates/pictura-app/cpp/tests/CMakeLists.txt` and verify `ctest --test-dir build -R '^tst_filter_menu$'` passes offscreen.

## 4. Filter command table and menu wiring

- [x] 4.1 Add `crates/pictura-app/cpp/filter_commands.{h,cpp}` with one row per implemented filter (stable id, label, menu path, kind, parameter descriptors). Verify with the `tst_filter_menu` all-rows test that every row's kind resolves to an implemented menu id and that stub leaves stay disabled.
- [x] 4.2 Keep the hand-written CS6 `Filter` tree in `command_tree.cpp`; in a new `frame_menus_filter.cpp`, mark each table row's menu id implemented and register its handler plus the "unlocked normal pixel layer" enablement provider, and add both new `.cpp` files to `CMakeLists.txt`. Verify `tst_command_tree` still passes and stub leaves (e.g. Filter Gallery, Reduce Noise) stay disabled.
- [x] 4.3 Port the per-filter parameter descriptors from photorust `MainWindow.cpp` (the `applyFilterWith` block) into the table so each implemented kind opens the correct controls; parameterless kinds apply directly. Verify by extending `tst_filter_menu`: choosing a parameterised kind opens a dialog, a parameterless kind applies immediately, and an unimplemented entry stays disabled.
- [x] 4.4 Add the `Source:` provenance trailer/headers for the ported dialog and descriptor mapping.

## 5. Last Filter commands

- [x] 5.1 Wire `Last Filter` (`Ctrl+F`) and `Last Filter Settings` (`Alt+Ctrl+F`) in `frame_menus_filter.cpp` using the bridge last-filter state: `Last Filter` re-applies with no dialog, `Last Filter Settings` reopens the filter's dialog prefilled. Verify in `tst_filter_menu`: after committing Gaussian Blur, `Last Filter` applies the same radius without a dialog and `Last Filter Settings` opens prefilled.
- [x] 5.2 Add a label provider naming the last filter and an enabled provider disabling both commands until a filter has been committed. Verify in `tst_filter_menu` that both are disabled before the first commit and the `Last Filter` label matches after.

## 6. Integration verification and docs

- [x] 6.1 Run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run --workspace`, and `cargo test --workspace --doc`; verify all green.
- [x] 6.2 Build with CMake and run `ctest --test-dir build -R '^tst_' --output-on-failure` offscreen; verify `tst_filter_menu` and the existing suites pass and the self-test budget is unchanged (no new `runSelfTest` checks).
- [x] 6.3 Update `docs/dev/STATE.md` with the filter-menu resume note and run `openspec validate --all --strict`; verify it is green.
- [x] 6.4 Run `bash scripts/verify-fast.sh` and verify it passes.

## 7. Manual-test review fixes

- [x] 7.1 Grayscale: teach `pictura-render::apply_filter` to filter a single-channel (Grayscale) layer by replicating channel `0` across the three working planes and writing the filtered plane back to channel `0`; refuse a missing channel `0` or a mixed `1`/`2` layout. Update `docs/dev/m6c-filter-integration.md` and the `imaging/filter-application` delta. Verify with Rust tests (grayscale changes channel `0` only; missing/mixed channels still error) and a Qt test that a Grayscale document commits a filter through the dialog.
- [x] 7.2 Dialog layout: match CS6 — thumbnail top-left, OK/Cancel/Preview stacked right, a centered zoom row with magnifier icons and a percentage label, and value box on the label line with the slider beneath. Verify with Qt tests that the zoom label steps and that zoom leaves the document image unchanged.
- [x] 7.3 Preview toggle: verify with a pixel-level Qt test that unchecking Preview restores the pre-filter pixels and re-checking re-applies.
- [x] 7.4 Menu ellipsis: add a label provider in `frame_menus_filter.cpp` so every parameterised entry ends with `…` and parameterless entries do not. Verify with a Qt test over all rows.
- [x] 7.5 No compositor parent-dim: add `runDialog(QDialog&, QWidget*)` and route every app dialog's `exec()` through it; add the `ui/dialog-presentation` delta and the `docs/02-ui-ux/application-frame.md` note. Update the filter Qt tests to find the non-modal dialog. Verify `ctest -R '^tst_'` and the manual dialog paths.
- [x] 7.6 Filter transparency for unlocked layers: run the same kernel over a grey copy of the alpha plane so blur/noise affect a layer's edges; a transparency lock still preserves alpha and skips clear pixels. Update `docs/dev/m6c-filter-integration.md` and the `imaging/filter-application` delta. Verify with Rust tests (unlocked alpha changes; locked stays bit-identical).
- [x] 7.7 Preview the current section at the canvas zoom: the dialog thumbnail crops the visible document section seeded from the canvas zoom, and the live preview filters only that section expanded by `preview_apron` (the commit still filters the whole layer). Add `apply_filter_region`. Verify with a Rust test (region interior matches a full apply; outside is untouched) and the Qt zoom test.

