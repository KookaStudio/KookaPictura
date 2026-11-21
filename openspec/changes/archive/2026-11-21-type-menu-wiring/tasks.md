# Tasks

## 1. Planning and engine outlines

- [x] 1.1 Write `openspec/changes/type-menu-wiring/test-plan.md` with one row per spec scenario and a Coverage cell naming a test (`tst_*::*` or `<crate>::*`) or a justified manual check; the final verification task runs `python3 scripts/check-test-plan.py` once the named tests exist.
- [x] 1.2 Add `crates/pictura-render/src/type_outline.rs` with curve-preserving outline extraction for unwarped/on-path/area type and flattened warped contours, plus the outline refusal; verify unit tests cover rendered alpha-ink bounds (not the `type_placement` layout box), a transformed point/paragraph case, contour counts, warped, on-path, area, and an outline-less face.
- [x] 1.3 Expose the warp geometry helper to `type_outline` and register the module and exports in `crates/pictura-render/src/lib.rs`; verify `cargo nextest run -p pictura-render type_outline` passes.
- [x] 1.4 Add `convert_type_to_shape` in `crates/pictura-render/src/document_ops/layer_ops/` as an in-place layer mutation that clears `TySh`, the type tool, type styles, pixel channels, `raw_channels`, and `source_channels`; keeps the name, blend, opacity, raster layer mask, and effects; makes the glyph outline the vector mask (replacing an existing vector mask); and writes the type fill colour; verify tests cover the shape kind, ink bounds, retained effects and raster mask, cleared type data, the fill colour, save/reopen as a shape, and undo-driven restore through the bridge.
- [x] 1.5 Verify the engine group with `cargo nextest run -p pictura-render -p pictura-core` and `cargo clippy -p pictura-render --all-targets -- -D warnings`.

## 2. Catalogue and identifiers

- [x] 2.1 Update `entries.rs`: remove the Type Check Spelling, Find And Replace, and Extrude to 3D rows; add `type.panels.glyphs`, `type.language.options.default.features`, and the ten `type.openType.*` rows; set `checkable` on the radio/toggle rows and `implemented` on every wired row; update `golden/catalogue.tsv`; remove the removed Extrude id from `command_edition` and the `extended_only_command_ids` expected set in `command_catalogue.rs`, and from `tst_edition_gating.cpp`; verify `cargo nextest run -p pictura-script` and the edition-gating test pass.
- [x] 2.2 Add the new constants to `const_names.rs`, then regenerate `const_names.tsv` and `command_ids.h` with `scripts/gen-command-ids.py` (do not hand-edit the header); verify the `cr_id_drift` and catalogue golden tests pass.
- [x] 2.3 Add `edit.find.and.replace.text` to `modal_ids()` and extend `tst_command_tree::modalMetadata` to assert it is modal; verify the test passes.
- [x] 2.4 Add `TypePanelsGlyphs` to `commands.cpp`'s `panelToggles()` and its checked provider; verify the Glyphs row opens and closes the panel and follows its visibility.

## 3. Bridge and structural Type commands

- [x] 3.1 Create `crates/pictura-app/cpp/tests/tst_type_menu.cpp` with a smoke case and register it in `crates/pictura-app/cpp/tests/CMakeLists.txt`; verify the target builds and the smoke case passes offscreen.
- [x] 3.2 Add `type_create_work_path`, `type_convert_to_shape`, `type_can_outline`, and the Update All text-layer bridge to `type_tools` and `cpp/api/type_api.{h,cpp}`; verify Rust or Qt tests create a work path, convert a shape, and update all layers under one state each.
- [x] 3.3 Wire `Type > Create Work Path` and `Type > Convert to Shape` in `menu_type.cpp`, including the editable/outline-font/session gates, one-state history, active work-path selection, and the canvas overlay; add the outline cases (`createWorkPath`, `createWorkPathReplaces`, `outlineRefusals`, `convertToShape`, `convertToShapeKeepsAppearance`, `convertToShapeUndo`, `convertToShapeRoundTrip`, `warpedOutlines`, `onPathOutlines`, `areaOutlines`, `sessionGate`) to `tst_type_menu` and verify they pass.
- [x] 3.4 Replace the two `addUnimplemented` rows in `panels/layers_panel_type_menu.cpp` with registry-dispatched `type.create.work.path` and `type.convert.to.shape` rows, absent for non-type rows; update `tst_layers_row_menu::typeRows` and `rowActionsCarryCommandIds`, and add `tst_type_menu::layersRowParity`.

## 4. Type menu option rows

- [x] 4.1 Wire `Type > Anti-Alias`, `Type > Orientation`, `Type > Rasterize Type Layer`, and `Type > Update All Text Layers` with checked/enabled providers, the editable/session gates, and one-state handlers; verify `tst_type_menu` covers check state, apply, undo, and disabled rows.
- [x] 4.2 Add `MenuRegistrar::Runtime` callbacks for the two type preferences and wire `Type > Language Options` (Default/East Asian/Middle Eastern) to them; verify the menu check states, panel visibility, no history state, and persistence across relaunch.
- [x] 4.3 Add the `fontPreviewSize` Type preference, the `font_combo` preview-size accessor, and the six `Type > Font Preview Size` rows; verify `tst_type_menu` checks None omits the sample, Huge is larger than Small, the row height grows, and the choice survives a reload.
- [x] 4.4 Wire `Type > Paste Lorem Ipsum` for an active type session or a selected type layer, appending at the end of the layer's existing text; verify the session path records no state before commit, the layer path records one `Paste Lorem Ipsum` state with the exact appended text, and the disabled case.
- [x] 4.5 Add `Type > Panels > Glyphs` (already registered in 2.4) and `Type > OpenType` with the ten modelled features; verify the Glyphs toggle follows panel visibility without a type layer, an OpenType toggle records one state with the right checked state, and the Type/Character-panel cross-surface check passes (`tst_type_menu::openTypeCrossSurface`).
- [x] 4.6 Implement `Type > Save Default Type Styles` and `Type > Load Default Type Styles` with a versioned style payload in the Type preferences, a dedicated bridge function for the load state, and the editable/session gates; verify save records no document state, load records one state, undo restores the style, the defaults survive an application relaunch, and a non-editable document keeps Load disabled.
- [x] 4.7 Add the remaining `tst_type_menu` cases (AA, orientation, rows-disabled, rasterize, update-all, language, preview, lorem, glyphs, OpenType, defaults, removed rows) and verify the suite passes offscreen.

## 5. Edit > Find And Replace Text

- [x] 5.1 Create `crates/pictura-app/cpp/tests/tst_find_replace.cpp` with a smoke case and register it in the Qt test CMake list; verify the target builds and the smoke case passes offscreen.
- [x] 5.2 Add the pure `replace_matches` matcher to `pictura-core` with ASCII case folding, whole-word boundaries, and left-to-right/right-to-left non-overlapping scans that do not rescan replacements (so `aa`→`X` in `aaa` gives `Xa` or `aX`); verify unit tests cover case, whole word, forward, and no match.
- [x] 5.3 Add the bridge replacement operation over type layers that are not fully locked, with the editable-document/current-layer/session gates, an empty-Find refusal, a blank-allowed path for a layer emptied by the replacement, and one `Find And Replace Text` state; verify Rust tests cover current-layer and all-layer scope, locked layers, empty Find, blank results, and no match recording nothing.
- [x] 5.4 Add `find_replace_dialog.{h,cpp}` and wire `Edit > Find And Replace Text…` in `menu_edit.cpp`; register the dialog in `cmake/sources/dialogs.cmake`; verify `tst_find_replace` covers the fields, options, exact Forward results, scope, no-op, non-editable/session gating, and round-trip, and `tst_command_tree::modalMetadata` covers the modal declaration.

## 6. Docs, archive, and integration

- [x] 6.1 Update `ROADMAP.md` (tick the Type menu, link #1442 and #1443 for the deferred surfaces) and the parity register UI-6.
- [x] 6.2 Update the affected contract docs: `docs/02-ui-ux/panels/layers-panel.md:57`, `docs/03-tools/type-tools.md:42,79,112,235` (including the opaque fill ceiling), `docs/02-ui-ux/menus.md:202-204,217,220`, and `docs/02-ui-ux/panels/character-and-paragraph.md:74,244`; the docs commit carries `TASK-ALLOWS-DOCS`.
- [x] 6.3 Run `python3 scripts/check-test-plan.py`; every scenario must have a row and every named test must exist.
- [x] 6.4 Run `cargo nextest run --workspace` and the new Qt suites offscreen under the CPU budget; verify every `test-plan.md` automated row has a passing result.
- [ ] 6.5 Run `bash scripts/verify-fast.sh` and `bash scripts/verify-full.sh`; fix any file-size, guard, or documentation failures.
- [ ] 6.6 Archive the change with `openspec archive type-menu-wiring`; verify `bash scripts/check-openspec-archived.sh` passes and only `openspec/changes/archive/` remains.
- [ ] 6.7 Run `bash scripts/verify-fast.sh` once more after archiving to confirm the zero-open policy and the final tree.
