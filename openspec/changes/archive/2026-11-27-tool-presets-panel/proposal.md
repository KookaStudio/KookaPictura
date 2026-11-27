# Proposal

## Why

`Window > Panels > Tool Presets` is a disabled catalogue leaf and no Tool
Presets panel exists (parity gap PAN-3, #511). The Preset Manager already
manages a Tools list and the codec already imports the brush/pencil subset of
`.tpl` libraries, but users cannot capture, browse, apply, rename, reorder, or
save a tool preset from a panel, and the options-bar Tool Preset picker refuses
every tool except Brush and Pencil.

## What Changes

- Add a **Tool Presets panel** in the `Window > Panels` menu and the Painting
  workspace. It lists the shared Tools preset list, applies a preset by
  switching to its owning tool and restoring the captured options, and offers
  Create New Tool Preset, Rename, Delete, Current Tool Only, Sort By Tool,
  Text Only / Small List / Large List, and Load / Replace / Reset / Save.
- Capture **every tool's** options-bar snapshot with the existing
  `toolOptionsToJson`/`patchToolOptionsJson` seam. Native `.kpset` tool entries
  carry the full options object as preserved opaque data, so a preset created
  for a Type, Crop, or any other tool round-trips every captured field.
- Keep the codec's existing `.tpl` import coverage (Brush and Pencil). A
  `.tpl` entry applies its mapped opacity, flow, and size; Save writes the
  native `.kpset` format because no `.tpl` encoder is evidenced.
- Generalize the options-bar Tool Preset picker to every tool, sharing the
  current Tools list with the panel. Brush and Pencil keep their separate
  brush-preset entries.
- Extend the Preset Manager's Tools page to run the panel's Load, Replace,
  Reset, and Save actions.
- Persist panel reordering through the current Tools list, so order survives a
  restart.

Out of scope: extending the `.tpl` decoder to arbitrary tools, writing `.tpl`
files, per-tool thumbnail rendering, and the CS6 scripting tool-name API.

## Capabilities

### New Capabilities

- `ui/tool-presets-panel`: the dock panel, its create/apply/edit/library
  surface, the Current Tool Only and Sort By Tool filters, display modes, and
  the shared-list picker behavior.

### Modified Capabilities

- `ui/preset-manager`: the Tools page exposes the panel's library actions and
  the tool-preset picker consumes every tool's entries.
- `codec/preset-libraries`: native Tools entries preserve the full tool-options
  snapshot in addition to the tool identifier and mapped values.

## Impact

- `crates/pictura-app/src/cxxqt_object/adjustment_presets.rs` and
  `adjustment_presets/library.rs`: `preset_make_tool` and full option values on
  Tool rows.
- `crates/pictura-app/cpp/panels/tool_presets_panel.{h,cpp}` (new),
  `tool_presets.{h,cpp}` (new): panel, shared matching/apply helpers.
- `crates/pictura-app/cpp/options_bar.cpp`: every tool's preset picker.
- `crates/pictura-app/cpp/preset_manager.cpp`: Tools library actions.
- `crates/pictura-app/cpp/panel_host*`, `panels/panel_group_menu.cpp`,
  `panels/panel_group.cpp`, `frame_build.cpp`: host, menu, workspace, and
  controller wiring.
- `crates/pictura-script/src/command_catalogue/` and generated
  `command_ids.h`: enable the Tool Presets command.
- `assets/icons/window.panels.tool.presets.svg` and `assets/pictura.qrc`.
- Tests: a new `tst_tool_presets_panel` Qt suite plus native codec tests.
- Docs (separate `TASK-ALLOWS-DOCS` commit): `ROADMAP.md`, `PARITY.md`,
  `docs/dev/doc-code-parity-gaps.md`, `public_docs/menu-window.md`, and
  `docs/dev/STATE.md`.
