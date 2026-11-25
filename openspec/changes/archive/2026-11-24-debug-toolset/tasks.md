# Tasks

## 1. Sampler

- [ ] 1.1 `debug_toolset.{h,cpp}`: `/proc` parsers and `DebugSampler` producing `DebugSnapshot`.
- [ ] 1.2 Name the app job threads (`pictura-open`, `pictura-save`, `pictura-filter`, `pictura-doc`) and the rayon global pool (`pictura-rayon-N`).

## 2. Panel

- [ ] 2.1 `debug_toolset_panel.{h,cpp}`: floating draggable panel, 2 s timer, three groups.
- [ ] 2.2 GPU row from `metrics_snapshot`.

## 3. Wiring

- [ ] 3.1 `command_ids::HelpDeveloperDebugToolset`, handler and checked provider in `frame_commands.cpp`, panel member in `frame.h`.
- [ ] 3.2 `pictura-script` catalogue entries, const names, `command.rs` list, and the two goldens.
- [ ] 3.3 Register sources in `cmake/sources/*.cmake` and the suite in `crates/pictura-app/cpp/tests/CMakeLists.txt`.

## 4. Tests

- [ ] 4.1 `tst_debug_toolset.cpp` per `test-plan.md`.

## 5. Trackers and docs

- [ ] 5.1 `ROADMAP.md` and `PARITY.md` (non-parity developer tool); Help-menu note in `docs/02-ui-ux/menus.md` (`TASK-ALLOWS-DOCS`) if the task allows docs.
- [ ] 5.2 Update `docs/dev/STATE.md`; archive the change.
