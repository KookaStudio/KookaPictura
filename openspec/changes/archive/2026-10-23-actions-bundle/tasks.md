# Tasks

## 1. Planning and fixtures

- [x] 1.1 Create `openspec/changes/actions-bundle/` with proposal, capability deltas, design, tasks, and `test-plan.md`; verify `openspec validate --all --strict` passes.
- [ ] 1.2 Capture the pre-change golden `list_commands` snapshot and the frozen `command_ids` list from the current build; verify the snapshots are committed and reproduce the current output exactly.
- [ ] 1.3 Add the self-skipping `.atn` corpus harness (`PICTURA_ATN_CORPUS`) plus `scripts/fetch-atn-corpus.sh` with SHA-256-pinned sources and provenance notes; do not commit any fetched fixture; verify the test self-skips when the variable is unset and round-trips the corpus when it is set.
- [ ] 1.4 Add `serde` and `serde_json` as direct `pictura-script` dependencies for the versioned app-native `TEXT` payload and text listing, state why in the commit, and regenerate third-party notices; verify `cargo deny check` and `python3 scripts/third-party-licenses.py` pass.
- [ ] 1.5 Add the `pictura-script` crate skeleton and workspace member; verify `cargo metadata --no-deps` lists it and `cargo check -p pictura-script` succeeds.

## 2. #273 — Rust command catalogue

- [ ] 2.1 Write the catalogue tests before the implementation: golden equality, id uniqueness, frozen-id `const_names`, path-derived ids, `cr_menu_catalogue`, `cr_identifiers_stable`, `cr_rust_only`, `cr_id_drift`, and `cr_list_commands`; register any new Qt slot in `tst_command_tree`, and add the Rust `cr_id_drift` header parser; verify they fail against the empty catalogue.
- [ ] 2.2 Implement `pictura_script::command_catalogue` with all 589 entries and the 229 frozen const names, giving `WindowPanelsActions` its documented F9 default shortcut; verify every catalogue-only test in 2.1 passes; header drift is verified in 2.3; split the table along menu seams if a file would exceed the 1200-line cap.
- [ ] 2.3 Add `scripts/gen-command-ids.py`, generate the checked-in `crates/pictura-app/cpp/command_ids.h`, replace the inline `command_ids` namespace in `commands.h` with that header, add `pictura-script` to the `pictura-app` dependencies, and add the `pictura-app` cxx-qt bridge exposing `CommandSpecRow`/`command_catalogue()`; verify `cr_id_drift` passes and the header is included by `commands.h`.
- [ ] 2.4 Replace `addDefaultCommands` with the Rust rows, delete `command_tree.cpp`, and update `CMakeLists.txt`; verify `tst_command_tree` passes and `cr_list_commands` matches the golden snapshot.
- [ ] 2.5 Run the #273 gate (`cargo nextest -p pictura-script`, `ctest -R '^tst_command_tree$'`, `bash scripts/check-file-size.sh`, `openspec validate --all --strict`) and commit `refactor(command): move the command catalogue into Rust (#273)`.

## 3. #616 — ActionDescriptor and recording bus

- [ ] 3.1 Write the `pictura-codec` tests before the implementation for every `af_atn_*`, `af_psp_*`, `af_desc_*`, and codec-level `af_malformed_*` row in `test-plan.md`, plus the optional corpus test; verify they fail against the absent codec.
- [ ] 3.2 Implement the exact Action Manager descriptor/reference codec in `pictura-codec`, keeping `DescValue` unchanged; verify all codec `af_*` tests pass.
- [ ] 3.3 Write the `pictura-script` engine tests before the implementation for every `ae_*`/`af_*` row whose coverage names `pictura-script`, plus the `tst_command_bus` tests; register `tst_command_bus` in `crates/pictura-app/cpp/tests/CMakeLists.txt`; verify they fail. The catalogue-only `cr_*` rows are written in 2.1; the `tst_actions_panel` rows are written in 4.1.
- [ ] 3.4 Implement the command definitions/classification, action model, library, recorder, and player in `pictura-script` over the `pictura-codec` container (adding the crate dependency); verify the `ae_*` and `pictura_script` `cr_*` tests pass.
- [ ] 3.5 Add `CommandRegistry` executors, recording sink, `recordCommand`, and `executeRecorded`; record only after a handler reports success, route control-server dispatch through the same path, and instrument every command classified recordable, starting with the Adobe-verified families and all parameterless commands; verify every `tst_command_bus` row passes.
- [ ] 3.6 Update the command-recording coverage matrix and `docs/dev/doc-code-parity-gaps.md`; correct `docs/09-automation/actions.md` for the exact `.atn`/`.psp` layout (`hasDescriptor`, byte-string names, `ObAr`/`UnFl`/`Pth `, `save` not `SveA`), the five dialog-option values, F1–F12, the CC-only conditional feature, and Action Options including the target set; verify `scripts/guard.sh` accepts the commit with `TASK-ALLOWS-DOCS`.
- [ ] 3.7 Run the #616 gate (`cargo nextest -p pictura-codec -p pictura-script`, `ctest -R '^tst_command'`, `bash scripts/check-file-size.sh`, `openspec validate --all --strict`) and commit `feat(automation): add the action descriptor and recording bus (#616)`.

## 4. #301 — Actions engine and panel

- [ ] 4.1 Write the Qt Test suites before the implementation for every `ap_*`, `pr_*`, and panel-integrated `ae_*` row in `test-plan.md`, including docked/flyout/float hosts, the accessibility model roles (`ap_a11y_model`), and the hover/shortcut cases; register `tst_actions_panel` in `crates/pictura-app/cpp/tests/CMakeLists.txt`; verify they fail.
- [ ] 4.2 Implement the Rust `ActionsEngine` bridge: library snapshot rows, default set, recording state, playback cursor, and function-key reservation; verify the engine and host tests start passing.
- [ ] 4.3 Implement `ActionsPanel`, the custom `QAbstractItemModel`/`QTreeView`, and the Button-mode `QListView` with stable node ids and derived check/modal states, adding every new source and header to `CMakeLists.txt`; verify all `ap_tree_*`, `ap_check_*`, `ap_modal_*`, `ap_select`, `ap_rename`, `ap_overwrite`, `ap_reorder`, `ap_duplicate`, `ap_delete_fast`, `ap_button_*`, and `ap_hosts` tests pass.
- [ ] 4.4 Add the `PanelGroup` dynamic menu hook, add `Insert Conditional…` and update the Actions menu table, and implement `performPanelMenuAction` plus installed-set discovery with a test-injectable presets root; verify `ap_menu_*` tests pass.
- [ ] 4.5 Implement the bottom toolbar, New Action/Set, Action/Set Options, Playback Options, Insert Stop, Insert Menu Item, Insert Path, and Insert Conditional dialogs; verify `ap_toolbar_*`, `ap_dialog_*`, and `ap_insert_*` tests pass.
- [ ] 4.6 Implement recording (menu, panel operations, tool strokes, Escape stop, Record Again) and the playback loop with speeds, set/action/step play, modal and specific-dialog pauses, stop, path, conditional, recursion, cancellation, errors, and the tool-stroke bound; verify `ae_*`, `ap_play_*`, `ap_record_again`, `ap_record_escape`, and `ap_allow_tool` tests pass.
- [ ] 4.7 Implement function-key assignment/precedence with the injectable Windows seam, session persistence, atomic `.psp` save/load, Save/Load/Replace/Reset/Clear, and the text dump; verify `ap_key_*`, `ap_state_restore`, `ap_library_restore`, `ap_save_load`, `ap_replace`, and `ap_text_dump` tests pass.
- [ ] 4.8 Add the `actions.*` cases to `tst_icon_assets` before the assets, then add the new panel glyphs with provenance, `lucide-map.json` entries, and `pictura.qrc` registration; verify `tst_icon_assets` and `scripts/check-icon-provenance.py` pass.
- [ ] 4.9 Correct `docs/02-ui-ux/panels/actions-panel.md` for the `Window > Panels > Actions` path, the content-panel host, the dynamic menu, the project-authored default set, the conditional extension, F1–F12, Action Options including the target set, the Insert Menu Item specific modal control, the whole-set Play app decision, `.psp` write timing/atomicity, and the `Close Tab Group` label; update `docs/dev/m42-panel-menus.md` with the implemented Actions rows (including the Insert Conditional extension); update `docs/09-automation/actions.md` if 3.6 left any panel-facing text; verify `scripts/guard.sh` accepts the `TASK-ALLOWS-DOCS` commit.
- [ ] 4.10 Run the #301 gate (`cargo nextest -p pictura-script`, `ctest -R '^tst_actions'`, `openspec validate --all --strict`, `bash scripts/check-file-size.sh`) and commit `feat(actions): implement the Actions panel and engine (#301)`.

## 5. Integration, manual checks, and archive

- [ ] 5.1 Run the full gate: `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run --workspace`, `cargo test --workspace --doc`, `ctest --test-dir build -R '^tst_'`, `openspec validate --all --strict`, `bash scripts/check-file-size.sh`, `bash scripts/guard.sh`, `bash scripts/verify-fast.sh`.
- [ ] 5.2 Perform the manual rows from `test-plan.md`: Step By Step visual redraw between commands and screen-reader announcement of inclusion/modal states; record the results for the PR body.
- [ ] 5.3 Update `docs/dev/STATE.md` and `ROADMAP.md` for the shipped Actions panel/engine, the Rust command catalogue (#273), and the command bus (#616); verify the docs commit carries `TASK-ALLOWS-DOCS`.
- [ ] 5.4 Archive the change with `openspec archive actions-bundle`; verify only `archive/` remains under `openspec/changes/`, `openspec validate --all --strict` passes, and the archived capability specs are correct.
- [ ] 5.5 Create the PR with title `feat(actions): Actions panel, action engine and command bus (#301)`, reference #301/#616/#273, list the manual-check results, and state the app-native tool/conditional/path limits and the whole-set Play decision; verify CI gates pass.
