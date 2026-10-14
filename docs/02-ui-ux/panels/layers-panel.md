# Layers Panel

- **Spec ID:** `PAN-001`
- **Status:** `Draft`
- **Parity tier:** `Core` (ships in Standard and Extended).
- **New in CS6:** `Changed` — CS6 adds the **filter/search row** at the top of the panel, right-click **color labels**, a **Properties panel** that edits the components selected here, simultaneous lock/blend/color edits across a multiple selection, the **Blend If** badge, an `FX` toggle, correct opacity/blend readout on hidden layers, `00`/`Shift+00` opacity shortcuts, `Tab`/`Shift+Tab` rename navigation, layer-name tooltips, and tool-reflecting shape-layer names. The panel chrome (thumbnail column, eye column, opacity/fill/blend header, bottom button strip, panel menu) is otherwise CS5-era.
- **Depends on:** `LAY-001` layers-overview, `LAY-002` layer-management-ui, `LAY-003` layer-groups, `LAY-004` layer-masks, `LAY-005` vector-masks-and-clipping-masks, `LAY-010` blend-modes, `LAY-011` layer-styles, `LAY-012` adjustment-layers, `LAY-013` fill-layers, `LAY-020` smart-objects, `LAY-021` smart-filters, `LAY-031` merge-and-flatten, `LAY-032` layer-filtering-and-search, `ARCH-003` qt6-ui-design, `ARCH-008` document-model, `ARCH-009` undo-history, `PAN-006` properties-panel.

> All crate, module, widget, and type names below are **design proposals**. No code exists in this repository. This file specifies the **panel widget behavior**; layer semantics (what visibility, locks, blend modes, masks, clipping and merge actually do) are owned by the `05-layers/` specs and are cross-referenced rather than restated.

## CS6 behavior

The Layers panel (`Window > Layers`, `F7`) is the dock that enumerates every layer, layer group, and layer effect in the document. It is the primary navigation surface for the layer stack: it shows/hides layers, creates layers and groups, and exposes the panel menu for the remaining commands. CS6 Help anatomy: **A** panel menu, **B** layer group, **C** layer, **D** expand/collapse layer effects, **E** layer effect, **F** layer thumbnail.

**Default placement (CS6 Essentials workspace).** The panel sits at the **bottom of the right-hand main panel column**, grouped as a tab set with **Channels** and **Paths**; Layers is the default-active tab. It is not open by default in any other group. (Source: Photoshop Essentials, *Managing Panels In Photoshop CS6*.)

This spec covers the widget: the header controls, the row delegate, the effects disclosure, the indicators, the filter row, the context menus, and the panel options. The command semantics behind every click live in `LAY-002` (create/duplicate/delete/rasterize/merge/select/link), `LAY-003` (groups), `LAY-010` (blend), `LAY-011` (effects), `LAY-012`/`LAY-013` (adjustment/fill content) and `LAY-032` (filtering).

### Header controls

The panel header carries, left to right: the **blend-mode popup**, the **Opacity** control, the **Fill** control, and — new in CS6 — the **filter/search row**. Below these sit the **lock toggles** (see below); they are not part of the blend/opacity/fill strip. Blend mode, Opacity and Fill are editable only when the selection permits; `LAY-002` records the rules (a group exposes Opacity only; Background and locked layers expose neither; type/shape layers force certain locks).

**Lock strip.** CS6 Help does not diagram the lock controls, but the CS6-era reference (Design Shack) places four lock icons at the **upper-left of the panel body**, in this left-to-right order: **Lock transparent pixels**, **Lock image pixels**, **Lock position**, **Lock all** (the `Lock All` toggle is bound to `/`). On a fresh CS6 Essentials install the panel opens with the blend-mode/opacity/fill strip at the top, the lock row beneath it (acting on the selected layer), the filter row, then the layer list and the bottom button strip.

### Row anatomy and indicators

Each row is drawn by a custom delegate. Left to right:

- **Eye column** — visibility toggle. `Alt`/`Option`-click shows only that layer/group and remembers prior visibility; a second `Alt`-click restores it. Dragging through the eye column toggles successive rows. Right-click (Windows) / Control-click (Mac) the eye opens a menu to show or hide either that single layer/group or every layer/group. Only visible layers print.
- **Thumbnail** — panel-wide size and content are set in `Panel Options` (`None` / small / medium / large; `Entire Document` or `Layer Bounds`). Clicking the thumbnail, as opposed to the row, selects the layer's non-transparent pixels as a document selection (`LAY-002`).
- **Name** — inline rename on double-click; `Tab`/`Shift+Tab` in CS6 move to the next/previous layer while renaming. Shape-layer names reflect the tool used (e.g. "Rectangle 1"). Tooltips include the layer name.
- **Color label** — CS6 sets it from the row's right-click context menu (CS5 used layer properties).
- **Mask thumbnails** — a layer mask thumbnail and/or a vector mask thumbnail, with a **link icon** between the layer and its mask when linked. The link icon is the click target for unlink/relink.
- **Clip indicator** — a layer clipped to the layer below is indented, and its base layer's name is underlined.
- **Style / effects badge** — the layer-effects (`fx`) expander; a **Blend If** badge appears when the layer's blending options have been customised. In CS6 the effects are ordered as applied (e.g. Drop Shadow below other effects).
- **Fill indicator** — the adjustment/fill/smart-object content thumbnail occupies the layer-thumbnail slot (see `LAY-012`, `LAY-013`, `LAY-020`).

### Effects disclosure

The triangle left of the folder icon expands/collapses a group. The `fx` arrow expands/collapses layer effects and smart filters; `Layers panel Options > Expand New Effects` controls whether newly added effects start expanded. `Alt`/`Option`-clicking the `FX` toggle arrows in CS6 shows/hides all layer effects on that layer.

### Filter / search row (CS6)

A popup selects the **filter dimension** — name, kind, effect, mode, attribute, or color label, plus the `Selected` isolation type — followed by a criterion editor, and a **toggle switch** turns filtering on/off. The default dimension is **Kind**, whose criterion editor is a row of layer-type icons (pixel, adjustment, type, shape, smart object); the `name` dimension shows a text field, the remaining dimensions show value lists, and `Selected` has no criterion (it shows the isolated set). Filtering shows a subset of rows; it is a transient view and must not mutate the document or clear the underlying active layer. The full per-dimension criterion matrix is owned by `LAY-032`; this spec owns the bar's layout and the on/off state machine.

### Panel menu, panel options, context menus

- **Panel menu** (top-right triangle): `Panel Options`, `New Layer`/`New Group`, `Duplicate Layer`/`Duplicate Group`, `Delete Layer`/`Delete Group`, `Merge Down`/`Merge Visible`/`Flatten Image`, `Blending Options`, lock and select entries, mask/vector-mask/clipping entries, `Convert to Smart Object`, and the remaining `Layer`-menu commands.
- **Panel Options**: thumbnail size (`None`/small/medium/large), thumbnail contents (`Entire Document` / `Layer Bounds`), `Expand New Effects`, `Add "copy" to Copied Layers and Groups`, and `Use Default Masks on Fill Layers`. CS6 Help names only the first three; the CS6-era sources show the copy-name and default-mask options in the same dialog (defaults: `Entire Document`, `Expand New Effects` on, `Add "copy"` on, `Use Default Masks` on).
- **Row context menu**: CS6 color label, layer-type-specific actions, `Copy CSS`, group commands, and the merge/rasterize/clipping entries valid for the row. Right-clicking the eye is the solo/all visibility menu noted above.

### Multi-select behavior

Click selects; `Shift`-click extends contiguously; `Ctrl`/`Cmd`-click toggles non-contiguously. In CS6 a multiple selection can change locking, blend mode, or color label in one action. `LAY-002` owns selection semantics (thumbnail-vs-row click, `Select > All Layers` / `Similar Layers`, linked layers, and how the active target is chosen for commands).

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Window > Layers` | Menu → panel | `F7` | Opens/focuses the dock. |
| Panel header | Blend-mode popup | — | Modes filtered by document color mode/bit depth (`LAY-010`). |
| Panel header | Opacity / Fill | `00`, `Shift+00` (0%) | Fill hidden/disabled for groups. |
| Panel header | Lock strip | `/` (Lock Transparency) | All / Transparent / Image / Position; type-shape force. |
| Panel header | Filter popup + criterion + toggle | — | CS6 six-dimension filtering plus the `Selected` isolation type (`LAY-032`). |
| Row → eye column | Toggle | `Alt`-click (solo) | Drag to toggle many; right-click for solo/all. |
| Row → thumbnail | Select pixels | `Ctrl`/`Cmd`-click | Loads layer transparency as a selection. |
| Row → name | Inline edit | double-click; `Tab`/`Shift+Tab` | CS6 next/prev while renaming. |
| Row → `fx` expander | Disclosure | `Alt`-click (show/hide all) | Effects/smart filters. |
| Row → context menu | Menu | right-click | Color label, type-specific actions, Copy CSS. |
| Panel bottom | Buttons | various | Link, Layer Style (`fx`), Layer Mask, Adjustment, New Group, New Layer, Delete. |
| Panel menu | Menu | — | Commands + `Panel Options`. |
| `Layer` menu | Menu | various | Full layer command set (`LAY-002`). |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Thumbnail size | enum | medium *(inferred)* | None / small / medium / large | `Panel Options`; `None` improves performance. Exact CS6 first-run default not stated in Help. |
| Thumbnail contents | enum | Entire Document | Entire Document / Layer Bounds | `Panel Options`. |
| Expand New Effects | bool | on | on / off | `Panel Options`. |
| Add "copy" to copied layers/groups | bool | on | on / off | `Panel Options`. |
| Use Default Masks on Fill Layers | bool | on | on / off | `Panel Options`. |
| Filter dimension | enum | kind | name / kind / effect / mode / attribute / color label / selected | CS6 plus the `Selected` isolation type; criteria in `LAY-032`. |
| Filtering enabled | bool | off | on / off | Toggle in the filter row. |
| Blend mode | enum | Normal | 27 layer modes + group `Pass Through` | `LAY-010`. |
| Opacity | percent | 100 | 0–100 | Groups: available; Background/locked: not. |
| Fill | percent | 100 | 0–100 | Groups: not available. |
| Lock flags | bool set | none | All / Transparent Pixels / Image Pixels / Position | Solid = full, hollow = partial, dimmed = locked group. |
| Visibility | bool | on | — | Eye column. |
| Color label | enum | none | preset palette | CS6 right-click. |

Panel Options defaults are those shown by the CS6-era Layers Panel Options dialog (`Entire Document`, `Expand New Effects` on, `Add "copy"` on, `Use Default Masks` on); the thumbnail size default is not stated in the fetched Help text.

## Algorithms & pipeline

The panel is a **view + controller** over the document node tree; it owns no authoritative state.

1. **Projection.** Flatten the `ARCH-008` node arena into display order (topmost first), emitting one row per node plus child rows per effect/smart filter. Group and artboard rows are expandable.
2. **Roles.** Emit id, name, kind, visibility, lock flags, blend mode, opacity, fill, clipping, color label, mask presence, blend-if badge, and effect list as item roles; the delegate paints from roles only.
3. **Filtering.** A proxy model filters projected rows by the CS6 dimensions; the toggle enables/disables the proxy. Filters never mutate document state.
4. **Commands.** Every edit (visibility, lock, opacity/fill, blend, color, rename, reorder, create/duplicate/delete, rasterize, link, filter off) is a command with an undo record (`ARCH-009`). The GUI thread queues commands; the model/view updates on completion. Multi-select edits are one command over multiple node ids.
5. **Drag-and-drop.** Dragging a row reorders the stack (or moves a row into a group); the drop target and legality are decided by the command layer, and the proxy must not present illegal targets.
6. **Solo visibility.** `Alt`-click snapshots prior per-layer visibility so a second `Alt`-click restores it exactly.

## Rust module mapping

Design proposal; the panel reuses the document/command surface rather than defining a parallel model.

- `pictura_core::document::layer_ops` — command constructors (`NewLayer`, `DuplicateLayer`, `DeleteLayer`, `Reorder`, `SetVisibility`, `SetLock`, `SetOpacity`, `SetFill`, `SetBlend`, `SetColorLabel`, `Rename`, `Rasterize`, `Link`, `LayerViaCopy`, `LayerViaCut`), as in `LAY-002`.
- `pictura_core::layer_filter` — `LayerFilter { dimension, criterion, enabled }` and the view projection (`LAY-032`).
- `pictura_ui_bridge::LayersViewModel` — projected row records for the Qt model: `{ node_id, parent, depth, roles… }`; reconstructed from the document, never the source of truth.
- `pictura_ui_bridge::LayerCommandBus` — enqueue/observe commands; carries `NodeId`, `LockFlags`, `BlendMode`, `ColorLabel`, `Option<&str>` name diffs.

Data crossing the boundary: `NodeId`, small scalar/enum diffs, projected row structs. No Qt types in `pictura_core`; no pixel data crosses for panel operations.

## Qt6 component mapping

Widgets (not QML), consistent with `ARCH-003` for dense, keyboard-centric dock chrome.

| Proposal | Base | Responsibility |
|---|---|---|
| `LayersPanel` | `QDockWidget` | Host; header, tree, bottom strip. |
| `LayersModel` | `QAbstractItemModel` | Node rows; roles for every column + expandable effect/smart-filter child rows. |
| `LayersFilterProxyModel` | `QSortFilterProxyModel` | CS6 six-dimension filtering plus the `Selected` isolation type; enable/disable toggle. |
| `LayerRowDelegate` | `QStyledItemDelegate` | Eye, thumbnails, name, color label, mask link icon, clip indent/underline, `fx` expander, Blend If badge. |
| `BlendModeComboBox` | `QComboBox` | Mode list, filtered by color mode/bit depth (`LAY-010`). |
| `OpacityFillWidget` | `QWidget` | Scrubby/label + slider; enable state from the selection. |
| `LockStrip` | `QToolButton` group | Lock All + partial toggles; tri-state (on/off/forced). |
| `LayerFilterBar` | `QWidget` | Filter-dimension popup, criterion editor, on/off switch (`LAY-032`). |
| `LayersPanelMenu` | `QMenu` | Panel menu + row context menu; built per row kind. |
| `LayerPropertiesDialog` / `GroupPropertiesDialog` | `QDialog` | New/rename dialogs (`LAY-002`). |

The model is GUI-thread-only; mutations go through the command bus. Drag-and-drop is implemented with `QAbstractItemModel` MIME data plus command validation, not direct model surgery.

## Data-model impact

- **No new persistent fields.** Visibility, lock flags, opacity, fill, blend, clipping, color label, link state and masks already exist on `Node` (`ARCH-008`); the panel is a pure view over them.
- **Filter/search is transient UI state**, never serialised (`LAY-032`).
- **Color label** maps to the existing PSD layer-record color field; CS6's right-click path only changes the input affordance.
- **Undo granularity:** one state per user action; a multi-select edit is one command spanning several node ids; live scrubby-slider drags coalesce into one state (`ARCH-009`).
- **Row order** is the document stack order; the panel must render the PSD order exactly.

## Edge cases

- **Background / locked layers:** opacity, fill, blend and reorder controls must reflect refusal (disabled/dimmed) rather than silently no-op.
- **Group selected:** Fill is disabled; only Opacity is offered.
- **Type/shape layers:** Transparent and Image locks are forced on in the strip and cannot be toggled off.
- **Hidden layers:** the header must still report correct opacity and blend mode (CS6 behavior).
- **Filter with no matches:** the tree shows an empty view; the active layer is retained.
- **Empty document / single background row:** header controls and delete/merge must disable cleanly.
- **PSB / thousands of rows:** projection and filtering stay O(rows); the delegate must not rasterise pixel data per row.
- **Undo/redo:** reversing a delete restores node id, order, children, masks, styles, link membership and color.
- **GPU unavailable:** panel operations are CPU/command-path only and unaffected.

## Parity acceptance criteria

1. Given a document, rows render top-to-bottom in PSD stack order with groups expandable and clipped layers indented under an underlined base name.
2. Given `Alt`-click on an eye, only that layer shows; a second `Alt`-click restores the exact prior per-layer visibility.
3. Given a group selected, Fill is disabled and Opacity is editable; given a Background layer, opacity/blend/reorder are refused.
4. Given a type or shape layer, Lock Transparency and Lock Image are shown locked and cannot be deselected in the strip.
5. Given each of the filter dimensions (the six CS6 ones plus the `Selected` isolation type), only matching rows show; toggling filtering off restores all rows without changing document state.
6. Given multiple selected layers, one blend/lock/color change applies to all in a single undo step.
7. Given `00` and `Shift+00`, layer opacity and fill opacity become 0%.
8. Given a layer with customised blending options, the Blend If badge shows; with no customisation, it is absent.
9. Given `Panel Options > None`, thumbnails are hidden and the row layout stays legible.
10. Given an undo of a delete, the restored row reproduces the original id, order, masks, styles and link state.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` (downloaded, `pdftotext -layout`) — official CS6 Help. Sections used: "Layers panel overview" (panel anatomy A–F; `Window > Layers`; `Panel Options` thumbnail size/contents; `Expand New Effects`); "Filter layers (CS6)" (six dimensions + toggle); "Show or hide a layer, group, or style" (eye, `Alt`-click solo, drag-through, right-click solo/all); "Layer opacity and blending" (Opacity vs Fill; group restriction; Background/locked restriction); "Moving, stacking, and locking layers" (lock icon solid/hollow/dimmed; type/shape forced locks); "Managing layers" (rename, color, delete, merge); "What's new in CS6 > Layers enhancements" (Properties panel edits selected layer components; JDI Layers list: color label via right-click, multi-select edits, `Rasterize Layer Style`, style order, Blend If badge, FX toggle, hidden-layer readout, `00`/`Shift+00`, `Ctrl/Cmd+J` group duplicate, `Tab`/`Shift+Tab`, tooltips, shape naming).
- `https://jkost.com/blog/2012/06/the-properties-panel-in-photoshop-cs6.html` — Adobe's Julieanne Kost: CS6 Properties panel shows the selected layer's properties and a mask icon at the top.
- `https://designshack.net/articles/software/the-master-guide-to-the-photoshop-layers-panel` (fetched) — CS6-era Layers-panel deep dive: the seven bottom buttons (link, layer styles, add layer mask, new fill/adjustment, new group, new layer, delete); the lock section (four locks, upper-left, listed transparent/image/position/all); `Panel Options` default state including `Add "copy" to Copied Layers and Groups`; search defaults to **Kind** with type-icon criteria and filters by name/kind/effect/mode/attribute/color-label; the `00` = 0 % opacity shortcut is CS6-only and `Shift` applies the number shortcuts to Fill.
- `https://www.photoshopessentials.com/basics/managing-panels-in-photoshop-cs6` (fetched) — CS6 Essentials default workspace: the Layers panel is the bottom group of the right-hand main column, tabbed with Channels and Paths.
- `https://www.photoshopessentials.com/basics/layers/essential-layers-panel-preferences` (fetched) — `Panel Options` thumbnail size / `Use Default Masks on Fill Layers` / `Add "copy"` controls.

Consulted as search-result snippets only (not individually fetched; community/third-party):

- SearXNG query for "Photoshop CS6 Properties panel shape layer live properties type contextual" — corroborated that the Properties panel is CS6-new and context-sensitive; per-corner "Live Shapes" and full Character/Paragraph integration are later CC revisions (see `PAN-006`).

Not used in this pass:

- `helpx.adobe.com` (HTTP 403) — modern Layers-panel help pages inaccessible; the archived CS6 Help PDF was used instead.

## Open questions

- **Panel Options thumbnail-size default** on a fresh CS6 install is still not stated in the fetched text (the other three defaults are now sourced). *Resolves with:* a default-preferences dump or CS6 capture.
- **Blend If badge artwork/trigger.** The badge is sourced; its precise trigger threshold (any customisation vs only certain tabs) is not. *Resolves with:* a CS6 test.
- **Exact criterion editor per filter dimension** beyond `Kind` (type icons) and `name` (text field) — the Help names the dimensions and the toggle but not each editor's form. *Resolves with:* a CS6 capture or the `LAY-032` panel-options detail.
- **Link-set persistence.** How linked-layer relationships serialise in PSD belongs to `LAY-002`/`ARCH-008`; the panel only renders it.
