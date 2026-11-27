# Tasks

## 1. Tests first

- [x] 1.1 Add native codec tests for a Tools entry carrying an opaque option snapshot and an unknown opaque field.
- [x] 1.2 Add `tst_tool_presets_panel` Qt Test cases for the panel command, Painting workspace, create/apply, filters, display modes, rename/delete, reorder, library commands, discovery, invalid import, the options-bar picker, and native round-trip.
- [x] 1.3 Register `tst_tool_presets_panel` in `crates/pictura-app/cpp/tests/CMakeLists.txt`.

## 2. Rust bridge

- [x] 2.1 Add `preset_make_tool(name, tool, options)` to the adjustment-presets bridge.
- [x] 2.2 Expose `tool`, `toolId`, mapped `.tpl` values, and the opaque `options` object from `preset_row_values`.

## 3. C++ panel and helpers

- [x] 3.1 Add `tool_presets.{h,cpp}` with the tool-name mapping, match, and apply helpers.
- [x] 3.2 Add `ToolPresetsPanel` with the list, footer, display modes, create/rename/delete, filter/sort, reorder, and library commands.
- [x] 3.3 Add the panel menu rows, hook, and `toolPresets.*` dispatch.

## 4. Wiring

- [x] 4.1 Register the panel in `PanelHost`, `panelWidgets`, the overflow group, and the Painting workspace.
- [x] 4.2 Wire the controller and active tool in `buildTools`.
- [x] 4.3 Enable the catalogue command, add the const name, regenerate `command_ids.h`, update the golden files and counts, and classify it non-recordable.
- [x] 4.4 Add the panel icon and qrc/provenance entries.
- [x] 4.5 Generalize the options-bar picker to every tool.
- [x] 4.6 Extend the Preset Manager Tools library actions.

## 5. Verification

- [x] 5.1 Run `cargo fmt --all`, clippy, and the Rust tests.
- [x] 5.2 Run the Qt test suites and the fast gate.
- [x] 5.3 Update `ROADMAP.md`, `PARITY.md`, the parity gap register, `public_docs/menu-window.md`, and `docs/dev/STATE.md`.
- [x] 5.4 Archive the OpenSpec change.
