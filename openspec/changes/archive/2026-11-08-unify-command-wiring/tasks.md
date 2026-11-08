# Tasks

## 1. Catalogue and id freeze

- [x] 1.1 Freeze the static leaves that still derive their id at runtime: Color
  Range, Refine Edge, Warp Text, Convert To Paragraph/Point Text, Replace All
  Missing Fonts, the 19 layer-style leaves, Apply Image, Image Size, Canvas
  Size, Print, Print One Copy, Filter Gallery, Read Watermark, the
  Align/Distribute leaves, Auto-Blend Layers, Lens Correction, the Type leaves,
  and the `frame_document_menus.cpp` Image Size, Canvas Size, and Print call
  sites. Add them to `const_names.rs`, regenerate `command_ids.h`, and verify
  `scripts/gen-command-ids.py --check` passes and no static `commandIdForPath`
  call remains in `frame_menus*.cpp`, `frame_build.cpp`, or
  `frame_document_menus.cpp`.
- [x] 1.2 Reject a derived id the catalogue does not contain in the Rust
  path-to-id bridge by returning an empty identifier and emitting a warning
  naming the path, and verify a new Rust test asserts the empty result on an
  unknown path and a non-empty result on a catalogue path.
- [x] 1.3 Register the remaining static handlers against `command_ids::`
  constants; update `tst_command_tree_cases.h` to skip empty-path rows in the
  `path.size() >= 2` and menu-presence checks and to compute the
  `crListCommands` count from the catalogue instead of hard-coding 597; verify
  `cargo nextest run -p pictura_app` and `tst_command_tree` pass.
- [x] 1.4 Fix the stale "generated from" headers in `entries.rs` and
  `const_names.rs`.

## 2. Modal command metadata

- [x] 2.1 Audit every handler that opens a dialog (`runDialog`, `QFileDialog`,
  `QInputDialog`, `QMessageBox`, and the unsaved prompt) and add the modal id
  list to the Rust catalogue, including Image Size, Canvas Size, Fill, Stroke,
  every Adjustments dialog, and the other `runDialog` users; verify the list
  contains every current denylist id plus the missing ones. Panel commands added
  in 5.1 are audited in 5.4.
- [x] 2.2 Expose the modal list through the bridge and `CommandRegistry::isModal`,
  delete `isModalCommand` from `control_server.cpp`, and verify
  `tst_control_modal_guard` passes with a new assertion that a known modal id
  and a known non-modal id report correctly.

## 3. Targeting anchor

- [x] 3.1 Add a `makeRowAnchor(path)` helper that writes the Rust target with
  the current selection and the clicked path as the anchor, call it from
  `showContextMenu` and `showMaskThumbMenu`, and verify a new Qt test keeps a
  two-layer selection while making the right-clicked row the anchor.
- [x] 3.2 Add panel-only commands for the rows whose effect differs from the
  menu-bar command (New Layer, New Group, Move Up, Move Down) and verify each
  creates or moves exactly as before through `tst_layers_panel` and
  `tst_layers_panel_bundle` cases.

## 4. Layer row, mask, and vector menus

- [x] 4.1 Replace the `RowSpec::Enable` enum with registry enablement, map each
  row to a command id, build actions with
  `CommandRegistry::detachedAction(id, parent)`, and delete `performRowAction`;
  verify `tst_layers_panel*`, `tst_layers_row_menu`, `tst_smart_object_actions`,
  and the new `lpr_row_action_registry` case pass.
- [x] 4.2 Replace the single Add Layer Mask and Add Vector Mask rows with their
  CS6 submenus, resolve the dynamic rows (`clippingToggle`, `link`,
  `rasterizeLayer`) by state, and add state-specific enable providers for
  `LayerMaskEnable` (mask present and disabled) and `LayerMaskDisable` (mask
  present and enabled); verify `tst_layers_mask_commands`,
  `tst_layers_row_menu`, and the new submenu scenarios pass.
- [x] 4.3 Move the Duplicate, Delete, and Merge labels into registry label
  providers and verify the menu bar, row menu, and flyout agree in
  `tst_command_parity` and the existing label tests, including
  `tst_merge_commands::layersPanelContextSensitiveLabel`.
- [x] 4.4 Delete `performMaskAction` and the vector-mask switch, and verify
  `tst_layers_mask_commands` and `tst_smart_object_actions` pass.

## 5. Panel flyout and per-panel actions

- [x] 5.1 Add the panel-only catalogue rows (`panel.<panel>.*`) for Layers,
  Actions, History, Character, Paragraph, and Properties. The Adjustments panel
  contributes no panel-only ids: its rows are existing
  `Layer > New Adjustment Layer` ids and preferences that stay local. Add
  `panel.properties.reset`, `panel.properties.toggleVisibility`, and
  `panel.properties.deleteLayer`. Add the new ids to `const_names.rs` and
  regenerate `command_ids.h`. Update the Rust
  `every_path_derived_id_matches_its_path` test to skip
  `spec.path.is_empty()` and add an assertion that empty-path ids are unique
  and start with `panel.`. Register handlers and verify each command dispatches
  from its panel and from the control server's `dispatch_command`.
- [x] 5.2 Rewrite `rowsForPanel` and `populatePanelMenu` to build implemented
  rows from `detachedAction(id, parent)`, delete `runPanelMenuAction` and the
  six `performPanelMenuAction` implementations, keep a small local preference
  path for Add Mask by Default and the Character/Paragraph preferences, and call
  `registry->refresh()` in the flyout's `aboutToShow` (or read the label
  provider directly) so dynamic labels are current. Verify `tst_actions_panel*`,
  `tst_actions_panel_cases`, `tst_type_session_panels`, `tst_adjustment_presets`,
  `tst_layers_panel_bundle`, `tst_command_tree::menusPanel`,
  `tst_properties_panel`, and
  `tst_merge_commands::layersPanelContextSensitiveLabel` pass.
- [x] 5.3 Route the Properties footer buttons through `panel.properties.reset`,
  `panel.properties.toggleVisibility`, and `panel.properties.deleteLayer`; map
  Clip to Create/Release Clipping Mask by `row.clipping`; make
  `panel.properties.deleteLayer` delete only `{path_}` with
  `layers_can_delete({path_})` and `clearSolo`, not the selection. Verify
  `tst_properties_panel`, `tst_last_layer` (flyout Delete, Properties footer
  Delete, and the `ll_can_delete_agrees` selection shapes), and the
  `ll_properties_footer_guarded` scenario pass.
- [x] 5.4 Extend `modal_ids()` for the new panel commands that open dialogs
  (Properties savePreset/loadPreset, History newSnapshot, Actions
  load/replace/save/clear/reset) and cover them in `tst_control_modal_guard`.

## 6. Shortcuts editor and parity

- [x] 6.1 Group empty-path commands by their `panel.<panel>` prefix in the
  Keyboard Shortcuts editor and verify a new
  `tst_command_tree::panelCommandsInShortcuts` case and
  `tst_keyboard_shortcuts::listsPanelCommands` pass.
- [x] 6.2 Add the parity cases to the existing suites (no new suite), scoped per
  command to the surfaces that offer it: Merge Down/Merge Layers and Duplicate = menu + row + flyout; Delete
  = menu + row + flyout + Properties; layer and vector masks = menu + row +
  thumbnail + Properties; Paste/Clear Layer Style = menu + row; Convert to
  Smart Object = menu + row + Background menu. Put the new anchor and
  row-command-id cases in `tst_layers_row_menu`
  (`rightClickAnchorsInsideSelection`, `rowActionsCarryCommandIds`) instead of
  duplicating the parity suite. Verify the suites pass.
- [x] 6.3 Update `docs/dev/STATE.md` and, if the item is listed there,
  `ROADMAP.md`; the commit needs `TASK-ALLOWS-DOCS` for the `docs/` change.

## 7. Integration checks

- [x] 7.1 Run `openspec validate --all --strict`, `cargo nextest run
  --workspace`, `ctest -R '^tst_'` (including `tst_layers_row_menu`,
  `tst_actions_panel_cases`, `tst_type_session_panels`,
  `tst_adjustment_presets`, `tst_layers_panel_bundle`, `tst_last_layer`, and
  `tst_merge_commands`), `scripts/check-file-size.sh`, and `scripts/guard.sh`;
  verify all pass.
- [x] 7.2 Close #277 and #830 in the PR body, list the `test-plan.md` manual
  rows as reviewer checks, and verify the PR body names the change.
