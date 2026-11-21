# Tasks

## 1. Planning gate

- [x] 1.1 Write `test-plan.md` with one row per spec scenario and verify by inspection that every `#### Scenario:` in the delta has a row; `python3 scripts/check-test-plan.py` passes after tasks 2.1, 4.1, and 7.1 add the named tests.

## 2. Engine tests (`pictura-select`)

- [x] 2.1 Add `RefineDab` and the dabs parameter scaffold plus `brush_stroke_changes_only_its_disk`, `brush_restore_undoes_refinement_in_order`, `brush_radius_bounds_changed_pixels`, `brush_later_refine_dab_replaces_the_radius`, `brush_invalid_dabs_are_ignored`, and `edge_band_matches_the_brush_expanded_radius` in `crates/pictura-select/src/refine.rs`; verify `cargo test -p pictura-select --no-run` compiles the new API and the tests are registered with `cargo test -p pictura-select -- --list`.

## 3. Engine implementation (`pictura-select`)

- [x] 3.1 Implement the ordered signed brush field and the local band radius, including restore-before-globals. Disable the identity early-return when any dab is present and run the band step when `settings.radius > 0 || !dabs.is_empty()`; `refine` keeps its signature and passes no dabs. Each dab overwrites the local refine/radius state in its disk in order (a later refine dab replaces an earlier radius, a later restore clears refinement including band expansion); `edge_band_from` does not mark a pixel last restored. Verify `cargo test -p pictura-select brush_` passes (including `brush_stroke_changes_only_its_disk`).
- [x] 3.2 Ignore dabs with non-finite coordinates or radius, a negative radius, a centre outside the coverage, or a radius larger than the coverage's longer side; bound the band skip by the largest valid dab radius and keep unbrushed pixels byte-identical. Verify `cargo test -p pictura-select brush_radius_bounds_changed_pixels` and `cargo test -p pictura-select brush_invalid_dabs_are_ignored` pass.
- [x] 3.3 Implement `edge_band_from` on the same per-pixel radius rule and export it with `RefineDab` from `lib.rs`; verify `cargo test -p pictura-select edge_band_matches_the_brush_expanded_radius` passes.
- [x] 3.4 Export the delta through the existing tests and check the module stays within the file-size cap; verify `cargo clippy -p pictura-select --all-targets -- -D warnings` and `bash scripts/check-file-size.sh` pass.

## 4. Bridge tests

- [x] 4.1 Add `tst_refine_edge::brushDabsApplyThroughTheBridge`, covering direct preview/apply with a dab and proxy/commit agreement, and `tst_refine_edge::brushBandPreviewMatchesTheEngine`, comparing the band preview externs with `edge_band_from`; verify the slots are registered in `crates/pictura-app/cpp/tests/tst_refine_edge.cpp`.

## 5. Bridge implementation (`cxxqt_object`)

- [x] 5.1 Extend the preview and apply externs with `dabs: &[f64]`; update `crates/pictura-select/src/tiled/tests.rs:372`, `crates/pictura-app/cpp/tests/tst_refine_edge.cpp:177,223,229`, `crates/pictura-app/cpp/refine_edge_dialog.cpp:214,216`, `crates/pictura-app/cpp/menu_select.cpp:221-224`, and `crates/pictura-app/cpp/panels/layers_panel_mask_menu.cpp:170-172` with an empty slice. Verify `cargo test -p pictura-app --no-run` and `cmake --build build --target pictura` (cargo --no-run does not build the C++ call sites). `lib.rs` exports `RefineDab` and `edge_band_from`.
- [x] 5.2 Parse and validate dabs, scale them by `preview_scale` for the proxy, and pass them through `apply`; verify `tst_refine_edge::brushDabsApplyThroughTheBridge` passes.
- [x] 5.3 Add the `refine_edge_band_preview` / `refine_mask_band_preview` externs and their proxy scaling; verify `tst_refine_edge::brushBandPreviewMatchesTheEngine` passes.

## 6. Canvas plumbing

- [x] 6.1 Add `CanvasBrushPhase` plus `setCanvasBrush` / `canvasBrushActive` to `ToolController`, dispatching hover, press, drag, and release in image space, with the crosshair cursor and pan disabled while armed; verify `cmake --build build --target pictura` and `tst_refine_edge` pass.
- [x] 6.2 Add the `setDialogCanvasBrush` input-blocker exception and the `ToolController*` / `ImageView*` dialog host provider installed from `menu_edit.cpp`; verify the existing dialog sampling tests and a `tst_refine_edge` canvas-hover case reach the brush path.
- [x] 6.3 Add `ImageView::setRefinePreview` / `clearRefinePreview` plus `refinePreviewForTest()` / `hasRefinePreviewForTest()` and paint it above the document overlays; verify `cmake --build build --target pictura` and a `tst_refine_edge` overlay case.

## 7. Dialog tests

- [x] 7.1 Add `tst_refine_edge::canvasDragAddsBrushDabs`, `brushKeysToggleResizeAndShowEdge`, `brushAltRestoresMomentarily`, and `cancelDiscardsBrushStrokes`; extend `optionsBarButtonOpensIt` to assert the `Select ▸ Refine Edge…` command enablement; add `tst_layers_mask_commands::refineMaskBrushes` calling `refine_mask_apply` with a dab. Verify the slots are registered in `tst_refine_edge.cpp` and `tst_layers_mask_commands.cpp`.

## 8. Dialog implementation

- [x] 8.1 Add the brush controls (size as a diameter in document pixels, default 30, 1–1000; refine/restore modes; Alt momentary restore via per-dab `restore = latched || QGuiApplication::keyboardModifiers().testFlag(Qt::AltModifier)`; `Shift+E` latch; bracket sizing; `J` Show Edge) and remove the stale no-brush note. Dab radius is size / 2, the hover ring diameter is size, and dabs space at radius / 4. Verify `tst_refine_edge::brushKeysToggleResizeAndShowEdge` and `brushAltRestoresMomentarily` pass.
- [x] 8.2 Arm the canvas, draw the hover ring, collect dabs (spacing a quarter of the dab radius, radius = size / 2), compose the red contour plus green band overlay, expose `dabsForTest()`, and clear all of it on close; verify `tst_refine_edge::canvasDragAddsBrushDabs` and `cancelDiscardsBrushStrokes` pass.
- [x] 8.3 Pass the dabs from `menu_select.cpp` and `panels/layers_panel_mask_menu.cpp` into apply; verify `tst_refine_edge` and `tst_layers_mask_commands` (including `refineMaskBrushes`) pass offscreen.

## 9. Documentation and parity

- [x] 9.1 Update `docs/08-selection/refine-edge.md` (UI surface, keyboard, parameters, and Rust module mapping) with the CS6 and CC 2015.5+ brush behavior, Alt restore, Show Edge alongside the CS6 Show Radius name, the `RefineDab` / `refine_from` / `edge_band_from` mapping, the estimator/geometry ceiling, and the deferred Select and Mask features; update `docs/dev/STATE.md` lines 5220-5223 to the same ceiling. Verify the new section names both generations, every deferred control, and the ceiling.
- [x] 9.2 Mark SEL-4 SHIPPED in `docs/dev/doc-code-parity-gaps.md` with the new tests as evidence and record the geometry/approximation ceiling; do not claim the docs' strand-recovery criterion. Tick the brushes item in `ROADMAP.md`; verify the rows point at the shipped files and the ceiling.
- [x] 9.3 Verify the docs commit carries `TASK-ALLOWS-DOCS` and the conventional `(#612)` suffix.

## 10. Integration verification

- [ ] 10.1 Run `cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings`
- [x] 10.2 Build the app and run `QT_QPA_PLATFORM=offscreen ctest --test-dir build -R 'tst_(refine_edge|layers_mask_commands)' --output-on-failure`
- [ ] 10.3 Run `bash scripts/verify-full.sh`, `python3 scripts/check-test-plan.py`, and `bash scripts/check-file-size.sh`
- [x] 10.4 Run `openspec validate refine-edge-brushes --strict`, archive the change, and verify `openspec validate --all --strict` and `bash scripts/check-openspec-archived.sh`
