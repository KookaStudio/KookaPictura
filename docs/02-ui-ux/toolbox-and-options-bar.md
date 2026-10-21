# Toolbox and Options Bar

- **Spec ID:** `UI-004`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `Changed` — the **Application bar was removed** and its functions redistributed: the **Screen Mode button moved to the bottom of the Tools panel**, the **workspace switcher moved to the options bar**, and window-layout options moved to `Window > Arrange`. The Tools panel defaults to a **single column** (double arrow toggles two). The Line and Shape tools became fully vector-based (stroke/fill/dashed via the options bar, plus `Align Edges` / `Constrain Path Dragging`); the retouch group gained **Content-Aware Move** and **Content-Aware Patch**; the Crop tool group gained **Perspective Crop** and a redesigned Crop options bar (Aspect Ratio menu, swap W/H); brush/erodible-tip and static-tip projection additions; maximum brush size raised to 5,000 px; the brush HUD can also change opacity; decimal Feather for Marquee/Lasso/Mask panel; tool presets stay reorganized after restart; new document presets. Flyout grouping mechanism itself is unchanged from CS5.
- **Depends on:** `UI-001` application-frame, `UI-002` menus, `UI-003` workspace-and-docks, `02-ui-ux/keyboard-shortcuts.md`, `03-tools/*`, `07-color-painting/brush-presets.md`

> All module and widget names below are **design proposals**. The repository
> contains a working implementation; this file stays the behavioural contract
> for the feature, and `docs/dev/doc-code-parity-gaps.md` records what shipped
> and what did not. Tool names, groups and shortcuts come from the fetched CS6
> Help reference; secondary facts are cited in `## Sources`. Anything still
> uncertain is marked *(inferred)* or flagged in `## Open questions`.

## CS6 behavior

### The Tools panel (toolbox)

- Starts docked at the **left of the screen**. Some tools have options that
  appear in the context-sensitive **options bar**.
- CS6 defaults to **one column**; click the **double arrow at the top of the
  Tools panel** to show side-by-side **two columns**, click again to return.
- **Hidden tools / flyouts:** a small **triangle at a tool's lower-right corner**
  means hidden tools are available. Hold the mouse button on the tool to reveal
  them, then click one. `Alt`/`Option`-click the tool cycles the hidden tools
  (except Add Anchor Point, Delete Anchor Point, Convert Point).
- **Tool tips** show the tool name below the pointer; the tool's **keyboard
  shortcut** is shown in the tool tip. With `Interface > Show Tool Tips` off,
  tool tips are hidden.
- **Temporary tool switch:** pressing and holding a tool shortcut switches to that
  tool only while held; releasing returns to the previous tool.
- **Cycle hidden tools:** by default `Shift` + repeatedly pressing the shortcut
  cycles the group. Disable `General > Use Shift Key For Tool Switch` to cycle
  without `Shift`.
- **Tool pointer** is configurable per `Cursors` preference: Standard, Precise,
  Normal Brush Tip, Full Size Brush Tip, Show Crosshair In Brush Tip, Show Only
  Crosshair While Painting. `Caps Lock` toggles standard/precise for some tools.
  Painting-cursor size/hardness is adjusted on-canvas with `Alt`+right-drag
  (`Ctrl`+`Option` on Mac): horizontal drag = size, vertical drag = hardness.
  `Ctrl+Alt`-drag (Windows; `Cmd`+`Option` on Mac) changes opacity instead, and
  requires deselecting `General > Vary Round Brush Hardness Based on HUD Vertical
  Movement`.
- At the bottom of the Tools panel, top to bottom: the **foreground/background
  colour** swatches, the **Edit in Quick Mask Mode** toggle, and the **Screen
  Mode** button. The Screen Mode button has a click-and-hold fly-out listing the
  three screen modes and calls the same controller as the `F` key.
- **Application bar removal (CS6).** The CS6 Application bar is gone. The
  workspace switcher now lives at the **right end of the options bar**; document
  layout options are under `Window > Arrange`; and the Screen Mode control moved
  to the bottom of the Tools panel. (The CS6 Help PDF still says "click the Screen
  Mode button in the Application bar" — stale text shared with CS5.)

### Tool groups and verified shortcuts

From the CS6 Help "Keys for selecting tools" table. In a row with multiple
tools, repeatedly press the shortcut (with `Shift` by default) to cycle. The
table order below is the toolbox slot order (the Help table omits unlettered
slots, see the Blur row).

| Group (flyout) | Tools | Shortcut | Notes |
|---|---|---|---|
| Move | Move | `V` | |
| Marquee | Rectangular, Elliptical | `M` | Help marks `†` (same key used in Liquify) |
| Lasso | Lasso, Polygonal, Magnetic | `L` | |
| Selection | Magic Wand, Quick Selection | `W` | |
| Crop / Slice | Crop, Perspective Crop, Slice, Slice Select | `C` | Perspective Crop is new in CS6 |
| Sampling / Measure | Eyedropper, Color Sampler, Ruler, Note, Count | `I` | `†`; Count is Extended-only |
| Retouch | Spot Healing Brush, Healing Brush, Patch, Content-Aware Move, Red Eye | `J` | Content-Aware Move new in CS6; Patch gained a Content-Aware option |
| Paint | Brush, Pencil, Color Replacement, Mixer Brush | `B` | |
| Clone | Clone Stamp, Pattern Stamp | `S` | |
| History | History Brush, Art History Brush | `Y` | |
| Erase | Eraser, Background Eraser, Magic Eraser | `E` | `†` |
| Fill | Gradient, Paint Bucket | `G` | |
| Toning (blur) | Blur, Sharpen, Smudge | — | Own slot; **no letter** in the CS6 shortcut table |
| Toning | Dodge, Burn, Sponge | `O` | |
| Pen | Pen, Freeform Pen | `P` | + Add/Delete Anchor Point, Convert Point |
| Type | Horizontal Type, Vertical Type, Horizontal Type Mask, Vertical Type Mask | `T` | |
| Path select | Path Selection, Direct Selection | `A` | |
| Shape | Rectangle, Rounded Rectangle, Ellipse, Polygon, Line, Custom Shape | `U` | CS6: fully vector |
| 3D object | Object Rotate, Roll, Pan, Slide, Scale | `K` | Extended only |
| 3D camera | Camera Rotate, Roll, Pan, Walk, Zoom | `N` | Extended only |
| Navigate | Hand | `H` | `†` |
| Rotate | Rotate View | `R` | |
| Navigate | Zoom | `Z` | `†` |

`†` The CS6 Help footnote marks these rows (Rectangular Marquee, Eyedropper,
Eraser, Hand, Zoom) as reusing the same key inside the Liquify dialog. The Help
notes that "in rows with multiple tools, repeatedly press the same shortcut to
toggle through the group" — with `Use Shift Key For Tool Switch` on (the default)
this is `Shift`+shortcut, otherwise the bare shortcut.

The Help's galleries also name the same tools by role (Move; Marquee; Lasso;
Quick Selection; Magic Wand; Crop; Slice; Spot Healing Brush; Healing Brush;
Patch; Red Eye; Clone Stamp; Pattern Stamp; Eraser; Background Eraser; Magic
Eraser; Blur; Sharpen; Smudge; Dodge; Burn; Sponge; Brush; Pencil; Color
Replacement; Mixer Brush; History Brush; Art History Brush; Gradient; Paint
Bucket; Path Selection; Type; Type mask; Pen; Shape; Line; Custom Shape; Hand;
Rotate View; Zoom; Note; Eyedropper; Color Sampler; Ruler; Count; 3D object and
camera tools).

The **Blur, Sharpen, Smudge** tools are a single flyout group of their own (the
teardrop slot), separate from Dodge/Burn/Sponge, and the CS6 tool-shortcut table
gives them **no default letter** — they are reachable only via the flyout or
`Alt`-click. Their panel position is between the Gradient/Paint-Bucket slot and
the Dodge/Burn/Sponge slot.

### The options bar

- Appears **below the menu bar** at the top of the workspace. It is
  **context-sensitive**: it changes with the selected tool. Some settings
  (painting mode, opacity) are shared by several tools; some are tool-specific.
- **Layout.** Left end: the active tool's icon and the **Tool Preset picker**.
  Middle: the tool-specific controls. Right end: the **workspace switcher** (in
  CS6, after the Application bar was removed).
- Can be moved using its **gripper bar** and docked at the top or bottom of the
  screen. Show/hide with `Window > Options`.
- **Reset a tool:** right-click (`Ctrl`-click on Mac) the tool icon in the
  options bar, then choose `Reset Tool` or `Reset All Tools`.
- **Brush Preset picker** (painting tools) lets you choose a preset and
  temporarily modify the brush **size and hardness**; the change is temporary and
  reverts on the next preset selection.
- CS6-specific option changes sourced from the Help: vector shape tools expose
  **stroke/fill/dashed-line** controls plus `Align Edges` and `Constrain Path
  Dragging`; the Eyedropper Sample menu gained "current layer and below" and
  "ignore adjustment layers"; Marquee/Lasso/Mask **feather accepts decimal
  values** (CS6, 0–250 px range); `Free Transform` gained an **Interpolation**
  menu and the Crop / Perspective Crop **Front Image** shortcut changed from `F`
  to `I`; the Crop options bar gained an **Aspect Ratio** menu and a double-arrow
  that swaps Width/Height (replacing the Rotate Crop Box button); the brush HUD
  can change **opacity** with `Ctrl+Alt`+drag.

### Tool presets

- A **tool preset** saves a tool's options for reuse. Access via the **Tool
  Preset picker** in the options bar (leftmost, immediately right of the tool
  icon), the **Tool Presets panel** (`Window > Tool Presets`), or the Preset
  Manager. CS6 keeps the picker in the options bar — it was **not** removed.
- The picker's **Current Tool Only** checkbox filters to the active tool;
  deselect to show all presets.
- **Create:** set options → click the Tool Preset button / `Window > Tool
  Presets` → **Create New Tool Preset** → name → OK.
- **Manage:** panel menu offers Show All Tool Presets, Sort By Tool,
  Show Current Tool Presets, Text Only / Small List / Large List, and
  Rename/Delete Tool Preset, Reset/Load/Replace/Save Tool Presets. `Alt`/`Option`
  -click an item deletes it.
- **Library format:** CS6 tool-preset libraries are `.TPL` files, loaded from the
  picker's pop-up panel menu (`Load Tool Presets`) or `Edit > Presets >
  Export/Import Presets`. `.ABR` files are brush presets, not tool presets.
- CS6: reorganized tool presets **remain after restart**; the SDK can read the
  tool name associated with a preset.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Tools panel | Dock (left) | tool letters | 1-column default; 2-column optional |
| Double arrow (top of Tools) | Toggle | n/a | Switch 1- vs 2-column |
| Hidden-tool triangle | Flyout | hold click / `Alt`-click | Cycle with `Shift`+shortcut |
| Options bar | Tool bar | `Window > Options` | Context-sensitive; gripper; top/bottom dock |
| Tool Preset picker | Popup | n/a | Left end of options bar; Current Tool Only checkbox |
| Workspace switcher | Popup | n/a | Right end of options bar (CS6, post-Application-bar) |
| Tool icon in options bar | Context menu | right-click (`Ctrl`-click) | `Reset Tool`, `Reset All Tools` |
| Brush Preset picker | Popup | n/a | Temp size/hardness override |
| Tool Presets panel | Dock | `Window > Tool Presets` | Create/manage presets |
| Foreground/Background colour | Swatch | `X` swap, `D` defaults | Bottom of Tools panel |
| Quick Mask toggle | Button | `Q` | Bottom of Tools panel |
| Screen Mode button | Button + flyout | `F` / `Shift+F` | Bottom of Tools panel (CS6 move) |
| Tool tip | Overlay | n/a | Name + shortcut; `Show Tool Tips` |
| Painting cursor resize | On-canvas | `Alt`+right-drag | H size / V hardness; `Ctrl+Alt` opacity |

Bottom-of-Tools order (top to bottom): foreground/background swatches → Quick
Mask toggle → Screen Mode button.

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Column layout | enum | 1-column | 1 / 2 columns | Double-arrow toggle |
| Tool tooltip | bool | On | on / off | Interface preferences |
| Use Shift Key For Tool Switch | bool | On | on / off | General preferences |
| Show Tool Tips | bool | On | on / off | Interface preferences |
| Vary Round Brush Hardness Based on HUD Vertical Movement | bool | On | on / off | General preferences; off enables `Ctrl+Alt` HUD opacity |
| Cursor style | enum | Standard | Standard / Precise | Cursors preferences |
| Painting cursor | enum | Normal Brush Tip | Normal / Full Size / Crosshair variants | Cursors preferences |
| Max brush size | px | 5,000 (CS6) | 1–5,000 | Increased in CS6 |
| Feather value | decimal | 0 | 0–250 px; decimals in CS6 | Marquee/Lasso/Mask panel |
| Shape stroke/fill | control set | — | colour / gradient / pattern; dashed | CS6 vector shape options |
| Tool preset list | enum | default library | Text Only / Small / Large | Per panel menu |
| Tool preset library | file | default `.TPL` | `.TPL` tool presets / `.ABR` brushes | Load from picker menu |
| Current Tool Only | bool | Off | on / off | Tool Preset picker |

## Algorithms & pipeline

Design proposal.

- **Tools panel = icon grid model.** Each tool is a `ToolId` with `{icon,
  shortcut, group, flyoutIndex, cursorDefaults, optionsFactory}`. Rendering is a
  vertical grid (1 or 2 columns) of `QToolButton`s with a flyout (`QMenu`) where
  `group` has >1 member and a corner triangle.
- **Single active tool.** A `ToolController` owns `active_tool: ToolId` and a
  `temporary_tool: Option<ToolId>` stack for spring-loaded switching (hold
  shortcut; release restores). Esc cancels a drag without changing the tool.
- **Cycle logic.** `Shift`+shortcut (when `Use Shift Key For Tool Switch`) or
  `Alt`-click advances the group; cycle wraps and is deterministic.
- **Slot order.** The toolbox is a fixed list of slots in the Help
  shortcut-table order; unlettered slots (Blur/Sharpen/Smudge) still participate
  in flyout and `Alt`-click cycling. The visible tool of a slot is the last one
  used, and CS6 starts on a single column.
- **Options bar = per-tool widget factory.** Each `ToolId` supplies an
  `OptionsWidget` (Qt widget) built from a declarative option schema so
  common controls (mode, opacity, feather) can be reused. Rebuilding on tool
  change must preserve focus-safe state and be cheap (`QStackedWidget` of cached
  option widgets rather than reconstructing each time *(inferred optimisation)*).
- **Reset.** `Reset Tool` restores the factory options for the active tool;
  `Reset All Tools` restores every tool. Presets are not deleted by reset.
- **Tool presets.** A preset is `{ToolId, options snapshot, name}`. The picker
  reads from a `ToolPresetStore` with a `current_tool_only` filter; `Alt`-click
  deletes; load/replace/save use `.tpl`-style libraries managed by the common
  Preset Manager.
- **Screen-mode integration.** The Tools-panel Screen Mode button and the `F`
  key drive the same `ScreenModeController` as `UI-001`.

## Rust module mapping

Proposals.

- `pictura_tools::registry` — `ToolId`, `ToolGroup`, `ToolDescriptor`
  (`icon`, `shortcut`, `options_schema`, `cursor_defaults`), built-in table.
- `pictura_tools::controller` — `ToolController` (active/temporary tool,
  spring-load stack, cycle/reset).
- `pictura_tools::options` — option schemas and value structs per tool group
  (common: `mode`, `opacity`, `flow`, `feather`; shape: `fill`, `stroke`,
  `align_edges`, `constrain_path_dragging`).
- `pictura_tools::presets` — `ToolPreset { tool, name, values }`,
  `ToolPresetStore`, library load/save/replace, `current_tool_only` filter.
- `pictura_tools::cursors` — cursor style resolution and on-canvas size/hardness
  HUD math.

Crossing types: `ToolId`, `ToolPreset`, small option value structs, `f32`
brush size/hardness/opacity. The Tools panel itself is Qt and consumes this via
the bridge; no pixels cross.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `ToolsPanel` | `QWidget` dock | Icon grid (1/2 col), flyout triangles, bottom strip (colour swatches, Quick Mask, Screen Mode) |
| `ToolButton` | `QToolButton` | Icon + `QMenu` flyout; `InstantPopup`/`DelayedPopup` per group |
| `ToolController` | `QObject` | Active/temporary tool, shortcut and cycle handling, reset |
| `OptionsBar` | `QToolBar` | Gripper, top/bottom docking, `QStackedWidget` of cached `OptionsWidget`s; left Tool Preset picker, right workspace switcher |
| `OptionsWidget` (per tool) | `QWidget` | Declarative controls; shared sub-widgets for mode/opacity/feather |
| `ToolPresetPicker` | `QToolButton` + `QMenu` | Popup preset list, Current Tool Only |
| `ToolPresetsPanel` | `QWidget` dock | Full preset management |

Tool buttons need `setPopupMode(QToolButton::MenuButtonPopup)` to show the
corner triangle for groups. The options bar is a `QToolBar` so `QMainWindow`
already handles gripper and top/bottom docking; a `QStackedWidget` switches the
active tool's option widget. The Screen Mode button is a `QToolButton` with a
`MenuButtonPopup` fly-out for the three modes; the workspace switcher sits at the
far right of the options bar (CS6, replacing the removed Application bar).

## Data-model impact

- **Tool state is session state**, not document data: active tool, temporary
  tool, column layout, options-bar position, last-used options, tool presets.
  Persisted in the preference/session store; never in PSD/XMP.
- **Tool selection produces no history state.** Tool *actions* produce history
  via `ARCH-007`; merely choosing a tool does not.
- **Options feed commands.** Tool option values are inputs to the command layer;
  a preset is a saved options snapshot, not a document node.
- **Shortcut mapping** of tool letters feeds `02-ui-ux/keyboard-shortcuts.md` and the Keyboard Shortcuts
  dialog (`UI-002`).
- CS6 persists reorganized tool presets across restarts; our preset store must
  therefore be durable, with library load/save via the Preset Manager.

## Edge cases

- **Standard vs Extended.** 3D tools (`K`, `N`) are absent in Standard; the
  Tools panel and shortcut table must reflect the edition.
- **No flyout room.** In 1-column narrow layouts the flyout menu must still open
  on-screen; clamp to screen geometry.
- **Shortcut collision.** A tool letter that clashes with a menu or custom
  shortcut must resolve deterministically and warn in the Keyboard Shortcuts
  dialog.
- **Spring-loaded + focus loss.** Releasing a held shortcut after the window
  loses focus must not leave the temporary tool active.
- **Options rebuild cost.** Switching tools rapidly must not flicker or leak;
  cache per-tool option widgets.
- **GPU-unavailable cursor previews.** On-canvas brush-size previews "require
  OpenGL"; without a GPU the cursor must degrade to a non-preview form rather
  than fail.
- **Huge brushes.** 5,000 px brushes at high zoom must not stall the HUD or the
  cursor outline.
- **Preset library missing/corrupt.** Fall back to the default library and keep
  user presets.
- **Unicode/localised tool names.** Tool tips and the presets panel must use the
  translation catalogue; ids stay stable.
- **Accessibility.** Icon-only tool buttons need accessible names, focus order
  and keyboard reachability (`02-ui-ux/accessibility.md`).

## Parity acceptance criteria

- Given the Tools panel, it starts as **one column**; clicking the top double
  arrow toggles to two columns and back, and the choice survives restart.
- Given CS6, the Application bar is absent: the Screen Mode button sits at the
  bottom of the Tools panel (with a three-mode hold fly-out) and the workspace
  switcher is at the right end of the options bar.
- Given a tool with hidden tools, holding the mouse reveals the flyout; `Alt`
  -click cycles the group; `Shift`+shortcut cycles when `Use Shift Key For Tool
  Switch` is on and not when it is off.
- Given a tool shortcut is pressed and held, the temporary tool is active;
  releasing returns to the previous tool, including after an Esc-cancelled drag.
- Given each verified tool, pressing its shortcut (row above) selects it; the
  tool tip shows the name and shortcut.
- Given a tool is selected, the options bar shows that tool's controls; moving it
  by the gripper and docking top/bottom works; `Window > Options` hides/shows it.
- Given right-click on the options-bar tool icon, `Reset Tool` restores the
  active tool's defaults and `Reset All Tools` restores every tool.
- Given a created tool preset with Current Tool Only on, only that tool's presets
  appear; with it off, all presets appear; `Alt`-click deletes a preset.
- Given reorganized presets, restart the app and the order persists (CS6
  behavior).
- Given Standard edition, 3D tools are absent; given Extended, `K`/`N` groups
  appear.
- Given the Eyedropper Sample menu, CS6 options "current layer and below" and
  "ignore adjustment layers" are present.
- Given a vector Shape tool, the options bar exposes fill/stroke/dashed controls
  and `Align Edges`/`Constrain Path Dragging`.
- Given `devicePixelRatio = 2`, tool icons render crisp and the flyout triangles
  are correctly placed.

## Sources

Fetched/opened for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  official Photoshop CS6 Help reference (downloaded and text-extracted).
  Establishes: Tools panel location, hidden tools and the corner triangle, tool
  tips and shortcuts, temporary tool switching, `Use Shift Key For Tool Switch`;
  cursor settings (Standard/Precise/Normal Brush Tip/Full Size/Crosshairs/Caps
  Lock, on-canvas size & hardness with `Alt`+right-drag H/V); the complete tool
  gallery (selection, crop/slice, retouching, painting, drawing/type,
  navigation/notes/measure, 3D); the "Keys for selecting tools" shortcut table
  (`V M L W C I J B S Y E G O P A T U K N H R Z`, plus `[`/`]`/`Shift+[`/`Shift+]`
  brush size & hardness under "Keys for painting"); the options bar description
  (context-sensitive, gripper, top/bottom docking, `Window > Options`) and
  `Reset Tool`/`Reset All Tools`; tool presets (picker, Current Tool Only,
  create/manage, Text/Small/Large, Alt-delete, picker shown in the options bar);
  CS6 changes — "functions formerly in the application bar have moved… click the
  button at the bottom of the toolbar" (screen mode), vector shapes with
  stroke/fill/dashed and `Align Edges`/`Constrain Path Dragging`, decimal feather
  0–250 px for Marquee/Lasso/Mask, Free Transform interpolation, Eyedropper
  "ignore adjustment layers" and "current layer and below", brush HUD opacity via
  `Ctrl+Alt`, 5,000 px max brush, Crop Aspect Ratio menu and Front Image `F`→`I`,
  Content-Aware Move/Patch and Perspective Crop, reorganized presets persisting.
  Caveat: the Help body text still says the Screen Mode button is "in the
  Application bar"; this contradicts its own What's New page and is stale CS5
  text.
- `https://www.photoshopforphotographers.com/pscs6/downloads/Photoshop-interface.pdf`
  (Martin Evening, *Adobe Photoshop CS6 for Photographers*, Focal Press sample
  chapter) — "the Application bar is gone, the Workspace options can be accessed
  via the Options Bar"; document layout via `Window > Arrange`; tools panel on
  the left, options bar across the top.
- `https://community.adobe.com/t5/photoshop-ecosystem-discussions/mini-bridge-panel-in-cs6/m-p/10118252`
  — Adobe Support Community: "the Application bar was removed in CS6", confirms
  the removal and the vertical-space rationale.
- `https://www.photoshopessentials.com/basics/managing-panels-in-photoshop-cs6/`
  — CS6 interface: `Tab` hides all panels including the Tools panel and options
  bar; `Shift+Tab` hides only the right-hand panels; panel columns.
- `https://www.psdvault.com/basics/photoshop-toolbar-customisation` — "By
  default, the Toolbar is presented as a one column panel"; double arrow toggles
  two columns; `Shift+Tab` hides all panels except Toolbar and Options Bar,
  `Tab` hides everything; bottom strip = foreground/background colour, Quick
  Mask, Screen Mode; slots share a shortcut and `Shift`+shortcut cycles.
- `https://enviragallery.com/guide-to-the-blur-tool-in-photoshop` — Blur is
  grouped in the toolbar with Sharpen and Smudge; "you cannot press a letter on
  the keyboard to select one of these three".
- `https://phdigitalarts.blogspot.com/2014/10/photoshop-screen-modes-and-interface.html`
  (Steve Patterson / Photoshop Essentials, CS6) — Screen Mode icon at the very
  bottom of the Tools panel; click-and-hold opens a fly-out listing the three
  screen modes; `F`/`Shift+F` cycle, `Esc` exits.
- `https://www.grutbrushes.com/install-photoshop-brush-toolsets-tpl-files` —
  `.TPL` is the Photoshop tool-preset library format, installed via the Tool
  Presets panel (`Load Tool Presets`) and filtered by the "current tool only"
  checkbox; `.ABR` files are brush presets, not tool presets.
- `https://itwiki.wpunj.edu/images/e/ee/Photoshop_CS6_Extended_-_Dacier.pdf` —
  secondary student tutorial; corroborates Tools-palette location, flyout access
  by holding the mouse, and foreground/background colour controls.
- SearXNG meta-search queries used to locate secondary sources. Facts are only
  taken from opened pages, not from snippets alone.

Not parsed: `helpx.adobe.com` (HTTP 403 on current help pages); the archive CS6
PDF was downloaded and text-extracted instead.

## Open questions

- **Full options-bar control set per tool.** This spec covers the documented
  shared/CS6-changing controls; a per-tool exhaustive control table belongs in
  `03-tools/*` and is not fully sourced here.
- **Tool-preset binary interoperability.** CS6 uses `.TPL` libraries, but the
  binary layout is undocumented, so Pictura defines its own tool-preset format;
  importing CS6 `.TPL` is out of scope unless required later.
- **Accessibility names/roles for tool buttons.** Unspecified in the CS6 Help;
  resolve in `02-ui-ux/accessibility.md`.
- **`Design` vs. `Typography` workspace** affects which panels a tool preset may
  be expected to accompany. See `UI-003` Open questions.
