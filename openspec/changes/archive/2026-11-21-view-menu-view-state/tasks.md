# Tasks

## 1. Catalogue, menu layout, goldens and docs

- [x] 1.1 Add the new command identifiers (`ViewExtras`, `ViewShowAll`, `ViewShowNone`, `ViewShowExtraOptions`, `ViewShowTargetPath`, `ViewShowLayerEdges`, `ViewShowGrid`, `ViewShowCount`, `ViewShowSlices`, `ViewShowNotes`, `ViewShowPixelGrid`, `ViewLockSlices`, `ViewClearSlices`, `View200`, `ViewPrintSize`) to `crates/pictura-script/src/command_catalogue/const_names.rs`, regenerate `command_ids.h`, and verify `python3 scripts/gen-command-ids.py --check` passes.
- [x] 1.2 In `entries.rs`, flip `implemented` for the wired rows and set `checkable` only for Extras, the ten Show entries and Lock Slices; add `view.clear.slices` and `view.show.extraOptions`; delete the `View > Align` and `View > Align To` rows. In `commands.cpp:110`, delete only the Align separator. The zoom presets, Show All, Show None, Clear Slices and Show Extra Options stay non-checkable. Verify `cargo test -p pictura-script` and `tst_command_tree::cr_menu_catalogue` after the golden update.
- [x] 1.3 Update `crates/pictura-script/tests/golden/catalogue.tsv`, `const_names.tsv`, and the expected counts in `catalogue.rs` (679 rows, 483 implemented, 403 const names; verify the arithmetic first), then verify both golden tests and `tst_command_tree::cr_menu_catalogue` pass.
- [x] 1.4 Update `docs/02-ui-ux/menus.md` with `Clear Slices`, the `Show Extra Options` semantics, and the edition-dependent window list; verify `bash scripts/guard.sh` accepts the docs change with `TASK-ALLOWS-DOCS`.

## 2. View state, Extras, Selection Edges and global commands

- [x] 2.1 Add the `tst_view_menu` suite with `defaultsAndPersistence`, `extrasMembership`, `extrasOptionsWindow`, `selectionEdgesScope`, `colorSamplers` and `globalCommands`, set `PICTURA_EDITION=extended` in `initTestCase` for Count, register it in `crates/pictura-app/cpp/tests/CMakeLists.txt`, and verify those six slots exist.
- [x] 2.2 Add the persisted View state to `SessionState` and `session.cpp` (per-item flags, color-sampler flag, Extras master, membership, grid settings, selection edges), apply it to every canvas and in the canvas setup path while skipping loading documents with no host/canvas, and verify the defaults, persistence, apply-to-all, overlay apply, no-document and later/loading scenarios through `tst_view_menu::defaultsAndPersistence`.
- [x] 2.3 Add `frame_view.cpp` (with its `cmake/sources/frame.cmake` entry) and register the Extras, Show, All and None handlers with checked and enable providers plus `has_document()` guards, then verify the membership, per-item-state, All/None and global-command scenarios through `tst_view_menu::extrasMembership` and `tst_view_menu::globalCommands`.
- [x] 2.4 Add `show_extras_dialog.{h,cpp}`, its `cmake/sources/dialogs.cmake` entry, the edition-aware list (Standard nine, Extended ten; Count only in Extended; no 3D Axis), the global command wiring, and Cancel handling; verify the window, Cancel and edition cases through `tst_view_menu::extrasOptionsWindow` and an extended `tst_edition_gating::editionGating`.
- [x] 2.5 Add a replacement-only selection generation bumped only by `set_selection`/`take_selection`, track the last-seen generation per document view in `refreshSelectionOverlay()`, move the Selection Edges handler and provider to the app-wide state, and verify the new-selection, in-place-edit/no-reset, two-document and Extras cases through `tst_view_menu::selectionEdgesScope`.
- [x] 2.6 Add the persistent color-sampler flag, set it from Show > All, clear it from Show > None, and gate it through Extras; verify `tst_view_menu::colorSamplers`.

## 3. Grid, Pixel Grid, Layer Edges and Target Path

- [x] 3.1 Add `tst_view_menu::gridOverlay`, `gridPreferences`, `pixelGrid`, `layerEdges` and `targetPath` to the suite and verify those five slots exist.
- [x] 3.2 Add the effective-visibility plumbing and the Grid overlay (`image_view_grid.cpp` with its `cmake/sources/image_view.cmake` entry), then verify the interval, minor lines, toggle and Extras behavior through `tst_view_menu::gridOverlay`.
- [x] 3.3 Replace the Grid group's value labels with numeric inputs, add object names and signals, default to Light Gray/Lines/1 inch/4, persist the keys, apply changes to every canvas, and verify `tst_view_menu::gridPreferences` (including percent spacing and `grid_prefs_group`) and `tst_rulers_guides::guidePreferences`.
- [x] 3.4 Draw the Pixel Grid above 500% zoom and verify the threshold, toggle and Extras behavior through `tst_view_menu::pixelGrid`.
- [x] 3.5 Compute the active layer's content rect through `transform_api::contentRect`, cache it until the target or `canvas_revision()` changes, draw the outline, and verify the initial target, target-change and document-edit invalidation cases through `tst_view_menu::layerEdges`.
- [x] 3.6 Wire `workPathOverlay(*v, -2)` into the frame-level refresh, define the shape-layer path read, suppress the base curve while a path-editing tool is active, add the refresh hook, and verify the curve, toggle/Extras, path-edit and path-tool cases through `tst_view_menu::targetPath`.

## 4. Persistent overlays, slice commands and Extras gating

- [x] 4.1 Add `tst_view_menu::sliceVisibility`, `sliceLock`, `clearSlices`, `countVisibility`, `noteVisibility`, `smartGuidesExtras` and `guidesExtras`, plus `pictura-render::clear_slices_clears_user_slices_and_keeps_one_auto_slice` and `pictura-render::clear_slices_refuses_without_user_slices`, and verify those slots exist.
- [x] 4.2 Move the base slice and annotation refresh to a frame-level apply that iterates every canvas, keep tool-owned decorations separate, pass the effective visibility mask and tool override, and verify the slice, count, note and two-canvas overlay cases.
- [x] 4.3 Add the persisted `Lock Slices` flag and make the Slice and Slice Select tools inert while it is on; verify the lock, persistence and clear-while-locked cases through `tst_view_menu::sliceLock` and `tst_view_menu::clearSlices`.
- [x] 4.4 Implement `pictura_render::clear_slices`, add `user_slice_count` (or scan `slice_at` for `userIndex >= 0`), expose the clear bridge through `crop_group`, add the disabled state and the one-state handler, and verify both Rust cases plus `tst_view_menu::clearSlices`.
- [x] 4.5 Extend the guides and smart-guides Extras gating, make placing a hidden guide turn Extras on, and verify `tst_view_menu::guidesExtras` and `tst_view_menu::smartGuidesExtras`.
- [x] 4.6 Make Extras win over the active Count and Note tools and verify `tst_view_menu::countVisibility` and `tst_view_menu::noteVisibility`.
- [x] 4.7 Run the existing `selftest_tools_selection`, `tst_edition_gating`, `tst_rulers_guides`, `tst_smart_guides`, `tst_save_for_web` and annotation/slice self-test coverage and fix any regression without weakening a scenario.

## 5. Zoom presets

- [x] 5.1 Add `tst_view_menu::zoomPresets` and verify the slot exists.
- [x] 5.2 Add the shared `ImageView` print-size helper and the View > 200% / Print Size handlers with document guards and enable providers, then verify `tst_view_menu::zoomPresets`.
- [x] 5.3 Point the zoom tool context menu at the shared print-size helper and verify `tst_command_tree::toolboxZoomMenu` still passes.

## 6. Integration and closeout

- [x] 6.1 Update `ROADMAP.md`, the UI-7 row in `docs/dev/doc-code-parity-gaps.md`, and `docs/dev/STATE.md`, and verify `bash scripts/guard.sh` accepts the docs change with `TASK-ALLOWS-DOCS`.
- [x] 6.2 Verify every `test-plan.md` slot exists, then run `openspec validate --all --strict`, `cargo nextest run -p pictura-render -p pictura-script -p pictura-app`, the offscreen `tst_view_menu`, `tst_edition_gating`, `tst_rulers_guides`, `tst_smart_guides` and `tst_command_tree` suites under the CPU budget, and `bash scripts/verify-fast.sh`; fix every failure.
- [x] 6.3 Archive the change with `openspec archive view-menu-view-state` and verify `bash scripts/check-openspec-archived.sh` passes.
- [x] 6.4 List the test plan's manual rows in the PR body as the reviewer checklist and verify the PR body contains each one.
