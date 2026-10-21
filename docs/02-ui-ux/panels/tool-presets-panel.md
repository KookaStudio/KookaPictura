# Tool Presets Panel

- **Spec ID:** `PAN-020`
- **Status:** `Draft`
- **Parity tier:** `Core` — the Tool Presets panel, the options-bar Tool Preset picker, and tool presets inside the Preset Manager ship in both CS6 editions.
- **New in CS6:** `Changed` — the panel UI is carried from CS5, but the CS6 JDI list records **"Reorganized tool presets remain after restart"** and the CS6 SDK adds scripting access to the tool name associated with a tool preset. No new panel controls are documented in the fetched CS6 Help text.
- **Depends on:** `BRU-006` `07-color-painting/brush-presets.md` (brush vs. tool preset boundary), `10-workflow-io/presets-manager.md` (Preset Manager / library I/O), `02-ui-ux/toolbox-and-options-bar.md` (options bar host), `01-architecture/qt6-ui-design.md` (`ARCH-003`, dock/model-view), `01-architecture/rust-core-design.md` (`ARCH-002`), `03-tools/type-tools.md` (`TOOL-050`, type tool presets).

> All crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Behavior is taken from the fetched CS6 Help PDF;
> anything inferred is marked *(inferred)*. The tool-preset file extension is
> **`.tpl`** and the library folder is **`Presets/Tools`** (confirmed by CS6-era
> secondary sources; the CS6 Help PDF itself documents only the Save/Load
> commands).

## CS6 behavior

A **tool preset** stores a tool plus its options-bar settings so the combination can be recalled without reconfiguring the tool. In CS6 Help terms, tool presets save and reuse tool settings; presets can be loaded, edited, and organized into libraries from the options-bar Tool Preset picker, the Tool Presets panel, and the Preset Manager.

**Default placement.** The Tool Presets panel is not in the default Essentials workspace; it is part of the **Painting** workspace, in the narrower secondary (icon) column alongside Brush, Clone Source, and History (`02-ui-ux/workspace-and-docks.md`, `UI-003`).

Three surfaces expose the same preset list:

1. **Tool Preset picker** — the options-bar pop-up, left of the tool icon.
2. **Tool Presets panel** — `Window > Tool Presets`.
3. **Preset Manager** — `Edit > Presets > Preset Manager`, Preset Type = Tools (`10-workflow-io/presets-manager.md`).

**Choosing a preset.** Click the Tool Preset picker in the options bar and pick a preset from the pop-up panel, or choose `Window > Tool Presets` and select a preset in the Tool Presets panel. Selecting a preset replaces the active tool's options with the preset, and the preset stays applied each time you select the tool until you choose Reset Tool from the panel menu. Selecting a preset for a tool other than the active one also switches the active tool (documented picker behavior; the panel's all-presets list is used this way). *(The exact wording for tool switching is from the panel documentation and evident behavior; mark inferred.)*

**Creating.** Choose a tool and set its options bar, then either click the **Tool Preset button** at the left of the options bar, use `Window > Tool Presets` → **Create New Tool Preset** button, or **New Tool Preset** from the panel menu; enter a name and click **OK**.

**Current tool vs. all tools.** The pop-up panel menu offers **Show All Tool Presets**, **Sort By Tool**, and **Show Current Tool Presets**; the panel's bottom-left **Current Tool Only** checkbox is the same filter. Display modes are **Text Only**, **Small List**, and **Large List** in the pop-up panel (the Preset Manager adds thumbnail modes; see `10-workflow-io/presets-manager.md`).

**Managing.** **Rename Tool Preset** and **Delete Tool Preset** are pop-up-panel menu commands; the Preset Manager can rename/delete/reorder items. Library commands are **Load Tool Presets** (append), **Replace Tool Presets**, **Reset Tool Presets** (replace or append the defaults), and **Save Tool Presets** (write the current list to a file). Placing a saved `.tpl` library in the default `Presets/Tools` folder makes it appear at the bottom of the panel menu after restart (CS6 installs ship `Presets/Tools`).

**Tool preset vs. brush preset.** Help distinguishes the two: a **tool preset** is for storing customized brush tip characteristics together with options-bar settings such as opacity, flow, and color. Brush presets and their `.abr` libraries are a separate namespace (`BRU-006`); this panel owns the tool-plus-options bundle only. A tool preset can capture **type tool settings** (font, size, attributes, color) as well.

**CS6 scripting note.** The CS6 SDK adds the ability to access the tool name associated with a tool preset name via scripting — relevant to `09-automation/` and to how the model identifies the owning tool, not to visible panel UI.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Options bar — Tool Preset picker | Pop-up panel button | — | Left of the tool icon; primary entry point |
| Options bar — Tool Preset button | Button | — | Opens/creates a preset next to the tool |
| `Window > Tool Presets` | Menu → dock panel | — | Tool Presets panel |
| Panel bottom-left | Checkbox | — | `Current Tool Only` / `Show Current Tool Presets` |
| Panel pop-up menu | Menu | — | New/Rename/Delete; Show All / Sort By Tool / Show Current Tool; Text Only / Small List / Large List; Load/Replace/Reset/Save Tool Presets |
| Panel bottom-left | Create New Tool Preset button | — | Opens the name dialog |
| Preset Manager | Dialog | `Edit > Presets > Preset Manager` | Preset Type = Tools; load/replace/rename/delete/reorder |
| Filesystem | Files | — | `Presets/Tools/*.tpl`; a library here appears at the bottom of the panel menu after restart |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Preset name | string | tool-derived | — | New Tool Preset dialog |
| Owning tool | tool ref | active tool | CS6 tool set | Set at creation; drives Current Tool filter |
| Captured options | tool config | current options bar | tool-dependent | Mode/opacity/flow/color/brush tip, crop size, type attributes, etc. |
| List filter | bool | off | Current Tool Only / Show All | Panel checkbox and pop-up label are the same state |
| Sort | enum | list order | Sort By Tool / manual | Preset Manager also supports drag-reorder |
| Display mode | enum | Small List *(assumed)* | Text Only / Small List / Large List (+ Preset Manager thumbnails) | Pop-up panel; CS6 default not stated |
| Function/shortcut | — | — | — | Not documented for tool presets (actions own function keys) |

## Algorithms & pipeline

The panel is a **model/view over a preset library**; it owns no image algorithms.

1. **Capture** — read the active tool id plus the current options-bar state and serialize it into a `ToolPreset { tool, name, options }`. *(inferred)* Options are an enum-tagged parameter block keyed by tool kind, so a type-tool preset and a crop-tool preset share no fields.
2. **Apply** — on selection, resolve the owning tool, switch the active tool if necessary, and push the stored option block into the tool's state. The preset stays applied every time the tool is selected until Reset Tool is chosen, i.e. applying a preset is sticky until reset or another preset is chosen.
3. **Filter/sort** — `current_tool_only` filters by owning tool; `sort_by_tool` groups by tool id.
4. **Persist** — session-created presets live in the preferences store; durable libraries are written by **Save Tool Presets** and read by **Load/Replace/Reset**. *(inferred: the CS6 text describes the commands but not the container format.)*

Tool switching and option application must go through the same command/active-tool service the options bar uses, so the panel cannot leave the toolbar and options bar out of sync.

## Rust module mapping

Proposals, consistent with `BRU-006`:

- `pictura_presets::tool::ToolPreset` — `{ id: ToolPresetId, tool: ToolId, name: String, options: ToolOptions }`.
- `pictura_presets::tool::ToolOptions` — enum-tagged per-tool parameter block (crop, type, brush, marquee, eyedropper, …).
- `pictura_presets::tool::ToolPresetLibrary` — ordered list; `filter_current_tool(tool)`, `sort_by_tool()`, `create/rename/delete`, `load/replace/reset/save`.
- `pictura_presets::prefs::PreferencesStore` — session persistence of newly created presets (shared with `BRU-006`).
- `pictura_ui_bridge::ActiveTool` — read/push the options-bar tool state; the panel applies presets through this service.

Crossing types: `ToolPresetId`, `ToolId`, `ToolOptions`, and small option structs. No Qt types cross into `pictura_presets`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `ToolPresetsPanel` | `QDockWidget` | Host; list display, Current Tool Only, bottom buttons, panel menu |
| `ToolPresetModel` | `QAbstractListModel` | Presets + owning tool + display name; filter/sort proxy |
| `ToolPresetProxy` | `QSortFilterProxyModel` | Current Tool Only / Sort By Tool |
| `ToolPresetPicker` | `QComboBox` pop-up / `QToolButton` | Options-bar picker; shared model with the panel |
| `NewToolPresetDialog` | `QDialog` | Name entry; create/rename |
| `PresetManagerDialog` | `QDialog` | Tools tab: load/replace/rename/delete/reorder (shared with `10-workflow-io/presets-manager.md`) |

Widgets over QML (dense docked list, model/view; `ARCH-003`). Preset thumbnails are rendered by the Rust tool/brush renderer and surfaced as `QImage`; list and picker share one model so they stay in sync.

## Data-model impact

- **No PSD fields.** Tool presets are application/preset-library state and are never serialized into the document. No PSD or XMP impact.
- **Preferences.** Newly created presets are stored in the preferences file; a preferences reset drops unsaved presets (CS6 behavior), so the UI should warn before a reset/replace.
- **Undo.** Preset create/rename/delete/reorder are not document history states; they are app-state operations. Applying a preset that changes tool options is also not an undoable document edit.
- **Namespaces.** Tool presets, brush presets (`BRU-006`), and other preset kinds are separate libraries and must not be conflated in storage or in the Preset Manager tabs.
- **Forward compatibility.** Preserve unknown per-tool option fields on read so a preset authored by a newer build survives a re-save *(inferred)*.

## Edge cases

- **Preset for another tool selected in the all-presets view** — must switch the active tool and update the options bar consistently.
- **Deleted/undefined preset library** — opening a missing library reports an error; never silently empties the list.
- **Preferences reset** — warn that unsaved tool presets will be lost.
- **Duplicate names** — disambiguate on load/create; do not silently overwrite.
- **Tool removed/changed** — a preset whose owning tool/option no longer exists must report the mismatch rather than apply a partial state.
- **Type tool presets** — must carry font, size, attributes, and color (`03-tools/type-tools.md`); a missing font on apply follows the type-tool fallback path.
- **Current Tool Only + no presets for the active tool** — show the documented empty state rather than a stale list.
- **Reordering** — CS6 JDI says reorganized tool presets persist across restart; ordering must be stored, not recomputed.
- **Non-ASCII names** — preset names must round-trip in the preferences/library store.
- **Preset library inside vs. outside the default folder** — loads either way; only the default folder yields a panel-menu restart entry.

## Parity acceptance criteria

1. Given a brush tool configured with a tip, opacity, flow, and color, `Create New Tool Preset` adds a named preset that reappears in both the panel and the options-bar picker.
2. Given a tool preset selected, the options bar shows the captured settings, and after switching to another tool and back the preset's settings are still applied until `Reset Tool`.
3. Given **Current Tool Only** on, the panel lists only presets whose owning tool is active; off, it lists all loaded presets.
4. Given **Sort By Tool**, presets are grouped/ordered by their owning tool.
5. Given `Save Tool Presets`, then `Reset Tool Presets`, then `Load Tool Presets`, the saved presets are restored (append semantics).
6. Given a library placed in the default `Presets` folder, its name appears at the bottom of the panel menu after restart.
7. Given CS6 JDI behavior, reordering presets and restarting the application preserves the new order.
8. Given a renamed or deleted preset, the change is reflected in the panel, the picker, and the Preset Manager.
9. Given a type-tool preset, applying it restores font family/style/size/attributes/color.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — primary CS6 Help corpus (downloaded, SHA-256/MD5 recorded, text-extracted with `pdftotext -layout`). Established: the tool-preset definition; Tool Preset picker / Tool Presets panel / Preset Manager; choosing a preset and the sticky "until you choose Reset Tool" behavior; Create New Tool Preset and New Tool Preset; Show All Tool Presets / Sort By Tool / Show Current Tool Presets / Current Tool Only; Text Only / Small List / Large List; Rename/Delete Tool Preset; Load/Replace/Reset/Save Tool Presets and the `Presets` default-folder menu behavior; the brush-vs-tool-preset distinction and options-bar opacity/flow/color capture; the CS6 JDI "Reorganized tool presets remain after restart" and CS6 SDK "access tool name associated with the tool preset name via scripting".
- `http://www.photoshopforphotographers.com/3101-1901/Help_guide/tp/Tool_Presets_palette.html` — Martin Evening, *Adobe Photoshop CS6 for Photographers* support page. Secondary CS6 corroboration: New Preset button, trash/delete, Current Tool Only, Show All Tool Presets, Save/Load Tool Presets, using tool presets for crop and type settings, and brush tool presets capturing brush + blending mode + color.
- `https://doc.qt.io/qt-6/qtreeview.html` — Qt 6 `QTreeView` (model/view contract, uniform row heights, sorting, drag-reorder) informing the Qt6 proposal.

Consulted as search-result snippets only (not individually fetched; community/current-version):

- SearXNG query "Photoshop CS6 Tool Presets panel Current Tool Only Create New Tool Preset" — corroborating snippets from Adobe/help, CreativePro, Lifewire, and the PSforPhotographers pages; later-version Help titles only, not used as CS6 fact.

Not used in this pass:

- `helpx.adobe.com` tool-preset pages (HTTP 403 / current-version only).

Fetched for this revision:

- `https://www.photoshopessentials.com/basics/photoshop-cs6-workspaces/` — CS6 Painting workspace: Tool Presets (with Brush and Clone Source) in the secondary icon column.

Consulted as search-result snippets only for the `.tpl` extension and `Presets/Tools` folder (not individually fetched): `https://vimeo.com/68231285` (CS6 .tpl import), `https://community.adobe.com/t5/photoshop-ecosystem-discussions/how-do-i-find-a-favorite-tool-in-an-old-version-of-photoshop/m-p/11481193` (CS6 `presets/Tools`), `https://www.grutbrushes.com/install-photoshop-brush-toolsets-tpl-files`, and `https://psfiles.com/how-to-install-photoshop-brush-toolsets-tpl-files`.

## Open questions

- **Internal `.tpl` binary layout and version.** The extension and "only a whole set/library is saved" behavior are documented, but the container format is not (owned by `10-workflow-io/presets-manager.md`). *Resolves with:* a CS6-saved `.tpl` and the file-format/SDK reference.
- **Default display mode** (Small List vs. Small Thumbnail) on a fresh install is not stated. *Resolves with:* a CS6 UI capture.
- **Which options are captured per tool** (the exact capture matrix) is only exemplified (brush, crop, type) in the fetched text. *Resolves with:* per-tool captures on CS6.
- **Does selecting an all-presets list entry always switch tools** in CS6, and does it warn about discarding current tool edits? *Resolves with:* a CS6 test.
- **Serialization of the owning tool id** for scripting (`tool name associated with the preset`). *Resolves with:* the CS6 Scripting/ExtendScript reference and `09-automation/extendscript-api-surface.md`.
