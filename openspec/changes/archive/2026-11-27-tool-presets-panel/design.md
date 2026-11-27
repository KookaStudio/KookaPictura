# Design

## Context

`PresetStore` already owns the eight application-wide preset lists, including
`PresetType::Tools`, and publishes them to the current-list store and the
settings backup. `PresetManager` already lists and reorders the Tools page.
The codec decodes `.tpl` v3 `tptp` records for `paintbrushTool` and
`pencilTool` into a `Payload::Tool` descriptor and exposes `Opct`, `Flw `/`flow`,
and `Brsh`/`Dmtr` as row values. The options bar builds one Tool Preset button
per tool page but only Brush and Pencil map to a preset list.

The options seam (`tool_options_json.{h,cpp}`) already serializes every tool's
option group to one JSON object and patches it back without touching document
state. That is the capture/apply contract for a tool preset.

## Goals / Non-Goals

**Goals:**

- One dock panel over the current Tools list, with the CS6 create/apply/edit
  surface and the Current Tool Only / Sort By Tool / display modes.
- Presets for every tool, round-tripping the full options snapshot.
- The options-bar picker and the panel share one list and one apply path.
- `.tpl` import at its existing Brush/Pencil coverage; native save.
- Persisted order through the existing current-list store.

**Non-Goals:**

- Extending the `.tpl` decoder to arbitrary tools or writing `.tpl` files.
- Per-tool thumbnail rendering; the panel uses the tool's icon.
- Session-persisting the Current Tool Only or display-mode toggles.
- The CS6 scripting API that reads a preset's tool name.

## Decisions

### Tool rows store the options snapshot as native opaque data

`Preset.opaque` already survives native decode/encode and is written by
`encode_native`. A new `preset_make_tool(name, tool, optionsJson)` builds a
`Payload::Tool` descriptor whose class id is the owning tool string and stores
the options JSON in `opaque`. `preset_row_values` returns:

```json
{
  "tool": "paintbrushTool",
  "toolId": "brush",
  "opacity": 37,
  "flow": 58,
  "size": 12,
  "options": { "brush": { ... }, "type": { ... } }
}
```

`tool` keeps the Photoshop class id for `.tpl` compatibility; `toolId` is the
Pictura catalogue name. Imported `.tpl` rows have no `options`, so applying
them falls back to the mapped opacity, flow, and size. Unknown option fields
inside `options` are preserved because the JSON object is opaque to the codec
and patched field-by-field by the shell.

### The panel is a QListWidget over the shared store

The existing panel architecture hosts plain `QWidget` content panels, not
`QDockWidget`s. `ToolPresetsPanel` owns a `QListWidget`, a footer with Current
Tool Only and New/Delete buttons, and a panel-menu hook. Each item carries the
stable `PresetRow.id` in `Qt::UserRole`, the owning `ToolId` name, and the
display name. The list rebuilds from `PresetStore::changed(Tools)`, preserving
the selected id.

Current Tool Only filters by the active tool. Sort By Tool is a stable sort on
the owning tool name. Display modes change the list view mode and icon size;
Text Only hides icons. Reordering is enabled only when neither filter is active
and writes the new id order back through `PresetStore::replace`. A filtered or
sorted view never changes the stored order.

### Create and apply go through the tool-options seam

Create reads `toolOptionsToJson(*controller)` for the active tool and appends a
native row made by `preset_make_tool`. Apply resolves the owning `ToolId` from
`toolId`/`tool`, calls `ToolController::setActiveTool` when it differs, then
`patchToolOptionsJson` with the stored `options`. Imported `.tpl` rows without
`options` set the Brush/Pencil opacity, flow, and size directly. Neither path
records document history.

### One helper table maps presets to tools

`tool_presets.{h,cpp}` holds `toolPresetToolName`, `toolPresetToolId`,
`toolPresetMatches`, and `applyToolPreset`. The panel, the options-bar picker,
and the tests use the same mapping:

- Brush -> `paintbrushTool`, Pencil -> `pencilTool` (the `.tpl` class ids).
- Every other tool -> its catalogue name (`toolIdName`).

### Library commands reuse the Preset Manager

The panel menu routes Load, Replace, Reset, Save, and discovered libraries to
`openPresetManager(window(), PresetType::Tools, action)`, which already owns the
file dialog, async import, partial-import confirmation, and atomic publish.
`PresetManager::runLibraryAction` gains `tools.load`, `tools.replace`,
`tools.save`, `tools.reset`, and `tools.manager`. Save uses the native `.kpset`
writer; Load and Replace accept `.kpset` and `.tpl`.

### The panel menu is a panel hook plus preference dispatch

New, Rename, Delete, Current Tool Only, Sort By Tool, display modes, and library
commands are rows in `rowsForPanel("toolPresetsPanel")`. The panel installs a
menu hook to sync check state and enablement, and `PanelGroup` routes
`toolPresets.*` ids to `ToolPresetsPanel::runPresetCommand`. Discovered
libraries append as submenus on the same hook.

### Options-bar picker

`OptionsBar::toolButton` keeps the Brush/Pencil brush-preset entries, then
dynamically appends every Tools row matching the page's tool. When none match,
it shows a disabled `No Tool Presets` entry. Applying uses the same
`applyToolPreset` helper.

## Risks

- Native `opaque` is a free-form byte field; a malformed or non-JSON opaque
  payload must be ignored rather than fail the row.
- The panel rebuild must not write back while the store is emitting `changed`.
  A scoped guard mirrors the Swatches panel.
- Enabling the catalogue command changes the frozen catalogue counts, the
  golden TSV, and `command_ids.h`; all three move together.
- The Painting workspace is seeded only on a fresh session; an existing user
  layout keeps its current groups.
