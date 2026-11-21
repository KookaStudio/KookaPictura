# Tasks

## 1. Scaffolding

- [x] 1.1 Add `file.browse` after `file.open`, flip `file.scripts.statistics` to implemented with the `Statistics…` label, add `file.scripts.browse` as a disabled row, add `file.scripts.statistics` to the Extended edition list and `extended_only_command_ids`, add `FileBrowse` and `FileScriptsStatistics` to `const_names.rs`, update `catalogue.tsv` and `const_names.tsv`, and regenerate `command_ids.h`; verify `cargo nextest run -p pictura-script` and `python3 scripts/gen-command-ids.py --check` pass
- [x] 1.2 Add the `statistics_dialog.h` public interface and a minimal `statistics_dialog.cpp` skeleton (constructor, accessors, source-list add methods), and register both files in `cmake/sources/documents.cmake`; verify the shell links

## 2. Tests first

- [x] 2.1 Create `crates/pictura-app/cpp/tests/tst_file_menu.cpp` with the `fb_*` cases and the disabled `file.scripts.browse` check, and register the suite in `crates/pictura-app/cpp/tests/CMakeLists.txt`; verify it configures and the cases are discovered
- [x] 2.2 Create `crates/pictura-app/cpp/tests/tst_statistics.cpp` with the `sd_*` dialog cases and the `iss_*` merge, failure, persistence, and no-document cases, and register the suite; verify it configures and the cases are discovered
- [x] 2.3 Extend `crates/pictura-app/cpp/tests/tst_edition_gating.cpp` with the Statistics Extended/Standard assertions; verify the suite still configures
- [ ] 2.4 Run the new suites and confirm they fail for the missing behaviour, not for compile or link errors

## 3. File > Browse…

- [x] 3.1 Add the private `openExternalUrl` helper to `frame.h`/`frame.cpp` and use it for the menu runtime's URL opener; verify `tst_document_windows`' reveal-in-file-manager cases still pass
- [x] 3.2 Register the `file.browse` handler in `frame_commands.cpp` with the document-folder/home fallback; verify the `tst_file_menu` Browse cases pass

## 4. Statistics dialog

- [x] 4.1 Move the stack source suffixes and file filters from `load_layers_dialog.cpp` to `dialogs.h`/`dialogs.cpp`; verify `tst_load_files_into_stack` still passes
- [x] 4.2 Replace the skeleton from 1.2 with the full dialog controls and source-list behaviour; verify the `sd_*` cases pass

## 5. Statistics command and persistence

- [x] 5.1 Add `statisticsOptions` to `SessionState` and `showStatisticsDialog` plus the `file.scripts.statistics` handler in `frame_commands.cpp`; write the accepted mode/align to the session immediately after the dialog returns Accepted, before loading the stack, and include the two-source failure path and skipped-file report; verify the `iss_*` cases, including `acceptedChoiceIsStored`, pass
- [x] 5.2 Verify the command is enabled with no document via `tst_statistics` and absent in Standard via `tst_edition_gating`

## 6. Documentation and tracking

- [x] 6.1 Update UI-8 to `PARTIAL` with `File > Browse…` shipped as the Linux replacement, `File > Scripts > Browse…` disabled until #620, `Check In…` and `Import ▸ WIA Support` platform-only, and the remaining owners #558/#617/#619/#620/#630; correct the File order (Browse before Open As), the `Statistics…` label, and add `Browse…` to the Scripts list in `docs/02-ui-ux/menus.md`; tick the File-menu item in `ROADMAP.md` with the split and owners; add the `docs/dev/STATE.md` entry; verify `python3 scripts/check-test-plan.py` and the guard accept the docs commit with `TASK-ALLOWS-DOCS`
- [x] 6.2 Add `Crop And Straighten Photos` to the AUTO-4 row in `docs/dev/doc-code-parity-gaps.md` and comment on #619 folding it into scope; verify both records are present

## 7. Integration and archive

- [x] 7.1 Run one build after all code is complete, then the targeted `ctest -R '^tst_(file_menu|statistics|edition_gating|command_tree|menu_registrar)$'`; verify all pass
- [ ] 7.2 Run `bash scripts/verify-full.sh`, `python3 scripts/check-cmake-source-fragments.py`, and `openspec validate --all --strict`; verify all pass
- [x] 7.3 Archive `file-browse-and-statistics` with `openspec archive` and confirm no non-archived change remains
