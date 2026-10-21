# Adjustments Panel

- **Spec ID:** `PAN-005`
- **Status:** `Draft`
- **Parity tier:** `Core` (individual adjustment types have their own tiers; 32-bit adjustment layers are Extended-only — `LAY-012`).
- **New in CS6:** `Changed` — CS6 keeps the adjustment **icons always visible** in the Adjustments panel and moves the **adjustment presets** out of the panel into a **Presets menu in the Properties panel** (presets for Levels, Curves, Exposure, Hue/Saturation, Black & White, Channel Mixer, Selective Color). Clicking an icon still both selects the adjustment and creates an adjustment layer; the actual parameter controls now live in the Properties panel, not the Adjustments panel.
- **Depends on:** `LAY-012` adjustment-layers, `LAY-013` fill-layers, `ADJ-000` adjustments-overview, the per-adjustment specs under `04-image-ops/adjustments/`, `LAY-010` blend-modes, `ARCH-008` document-model, `ARCH-009` undo-history, `ARCH-003` qt6-ui-design, `PAN-006` properties-panel.

> Crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Adjustment math is owned by `04-image-ops/`; layer
> semantics are owned by `LAY-012`; this file specifies the **panel widget and the
> create/animate flow**.

## CS6 behavior

The **Adjustments panel** (`Window > Adjustments`) gathers the color and tonal adjustments. Clicking a tool icon selects an adjustment and creates an adjustment layer in one step. The adjustments made through the panel are therefore **non-destructive** — they live on an adjustment layer and can be re-edited or discarded without changing underlying pixels (`LAY-012`).

**Default placement (CS6 Essentials workspace).** The panel is in the **middle of the right-hand main panel column**, grouped with the **Styles** tab; Adjustments is the default-active tab. (Source: Photoshop Essentials, *Managing Panels In Photoshop CS6*.)

In **CS5** the panel had a presets list and showed a separate icon screen; in **CS6**:

- the **adjustment icons are always visible**, so the panel is a launch grid;
- the **parameter controls are shown in the Properties panel** (CS5 showed them in the Adjustments panel);
- the **presets list moved to a Presets menu in the Properties panel** for the adjustment selected;
- the panel can be widened by dragging a bottom corner (CS5 had an Expand View button).

Clicking an adjustment icon (or choosing an adjustment from the panel menu) creates the adjustment layer and opens its settings in the Properties panel. The panel then exposes the same per-layer affordances as `LAY-012`:

- **Toggle Layer Visibility** — show/hide the adjustment layer.
- **Reset** — return the adjustment to its original settings.
- **Delete This Adjustment Layer** — discard the adjustment.
- **Clip to Layer** — confine the adjustment to the layer below (click again to apply to all layers below); equivalently `Layer > Create Clipping Mask` / `Ctrl`/`Cmd`+`Alt`+`G`.

The panel menu also offers `Auto-Select Parameter` and `Auto-Select Targeted Adjustment Tool`, which make text fields / the on-image targeted adjustment tool the default focus (`LAY-012`). Adjustment-panel channel shortcuts mirror the Layers/Channels conventions: `Alt`/`Option`+`2` selects the composite channel and `Alt`/`Option`+`3/4/5` red/green/blue, with the legacy `Ctrl`/`Cmd`+`1/2/3` scheme available through `Edit > Keyboard Shortcuts > Use Legacy Channel Shortcuts`; `Delete`/`Backspace` deletes the adjustment layer.

### Adjustment type grid

The visible adjustments (16 types; PSD keys and parameters in `LAY-012`):

`Brightness/Contrast`, `Levels`, `Curves`, `Exposure`, `Vibrance`, `Hue/Saturation`, `Color Balance`, `Black & White`, `Photo Filter`, `Channel Mixer`, `Color Lookup`, `Invert`, `Posterize`, `Threshold`, `Gradient Map`, `Selective Color` — plus the three **fill-layer** types `Solid Color`, `Gradient`, `Pattern` (`LAY-013`), which share the create flow but add content instead of transforming the backdrop. The three fill-layer icons appear **first** in the grid (above the adjustment icons), mirroring their position at the top of the `Layer > New Fill Layer` submenu and the Layers-panel New Fill/Adjustment menu (community/observed; the Help does not enumerate the grid).

**Color Lookup** is CS6-new (community-sourced; the fetched CS6 Help prose does not name it) and the PSD spec carries the `clrL` key. **Invert** has no editable settings; the Properties panel shows a read-only state for it. Availability is gated by document color mode and bit depth (`LAY-012`): bitmap/indexed disallow adjustment layers, and 32-bit adjustment layers are Extended-only.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Window > Adjustments` | Menu → panel | — | Display the panel; icons always visible in CS6. |
| Panel icon grid | Buttons | — | Click creates an adjustment layer and opens Properties. |
| Panel menu | Menu | — | Choose adjustment; `Add Mask by Default`; `Auto-Select Parameter`; `Auto-Select Targeted Adjustment Tool` (`Save Preset` moves to the Properties panel in CS6). |
| Panel corner | Resize | drag | Widen the panel (CS6). |
| Properties panel | Contextual controls | — | Parameter editor + Presets menu + Clip to Layer (`PAN-006`). |
| Layers panel New Adjustment Layer button | Button/menu | — | Alternative create path (`PAN-001`, `LAY-002`). |
| `Layer > New Adjustment Layer` | Menu | — | Submenu of all types; New Layer dialog. |
| `Layer > New Fill Layer` | Menu | — | Solid Color / Gradient / Pattern (`LAY-013`). |
| `Layer > Layer Content Options` | Menu | — | Re-edit the selected adjustment/fill layer. |
| Clip to Layer button | Button | — | Toggles clipping (`LAY-012`). |
| Adjustment channel shortcuts | Keys | `Alt/Option+2`, `Alt/Option+3/4/5` | Legacy `Ctrl/Cmd+1/2/3` via preference. |
| Delete adjustment layer | Key | `Delete` / `Backspace` | Deletes the selected adjustment layer. |

## Parameters & ranges

The panel itself has few controls; the adjustments' parameters are owned per type.

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Adjustment type | enum | none | 16 adjustment types + 3 fill types | Fixed at creation (`LAY-012`). |
| Presets menu | enum | none | Levels, Curves, Exposure, Hue/Saturation, B&W, Channel Mixer, Selective Color | CS6: in the **Properties** panel, not here. |
| Auto-Select Parameter | bool | off | on / off | Focus text fields automatically. |
| Auto-Select Targeted Adjustment Tool | bool | off | on / off | Default to the on-image tool. |
| Clip to Layer | bool | off | on / off | Confine to the layer below. |
| Reset | action | — | — | Restore original settings. |
| Toggle Layer Visibility | bool | on | on / off | Per-layer visibility. |
| Delete | action | — | — | Discard the adjustment layer. |
| Panel width | px | default | draggable | CS6 corner drag. |

Per-adjustment parameters (e.g. Levels input/output/gamma; Curves points; Hue/Saturation ranges; Color Lookup `3DLUT File`/`Abstract`/`Device Link`) are owned by `04-image-ops/adjustments/*` and `LAY-012`.

## Algorithms & pipeline

The panel is a **launcher**: it creates a layer and hands editing to the Properties panel. It owns no adjustment math.

1. **Create.** Clicking an icon sends a `NewAdjustmentLayer { kind }` command; the command layer inserts an `Adjustment { params }` node above the active layer, applies a default mask (built from the selection/path if present, `LAY-012`), and selects it.
2. **Edit.** The Properties panel binds to the selected node and edits `AdjustmentParams`; each parameter change is a command with an undo record (`ARCH-009`). The composition pipeline re-runs the adjustment pass on dirty tiles (`LAY-012`).
3. **Clip/reset/delete/visibility.** Thin wrappers over `LAY-012` commands.
4. **Presets.** CS6 reads/writes presets through the Properties panel's Presets menu; the Adjustments panel has no preset UI. Preset application creates/updates the adjustment layer.
5. **Gating.** The icon grid enables/disables entries by color mode, bit depth and edition (32-bit Extended-only).

## Rust module mapping

Design proposal; adjustment data and kernels already exist (`LAY-012`, `ADJ-000`).

- `pictura_core::node::AdjustmentParams` — enum, one variant per adjustment (`ARCH-008`).
- `pictura_core::document::adjustment_ops` — `NewAdjustmentLayer { kind, clip, mask_from }`, `SetAdjustmentParams`, `ResetAdjustment`, `SetAdjustmentVisibility`, `DeleteAdjustment`, `SetClipping`.
- `pictura_adjust` — adjustment kernels and typed parameter structs (`LAY-012`); `pictura_adjust::preset` for the seven preset-capable types.
- `pictura_core::edition` — `Edition` gate for 32-bit adjustments.
- `pictura_ui_bridge::AdjustmentCatalog` — the ordered icon list with per-entry availability `{ kind, label, icon, enabled, reason }`, crossing to Qt.

Crossing types: `AdjustmentKind`, `AdjustmentParams`, `Edition`, `ColorMode`, `BitDepth`. No Qt types in `pictura_core`.

## Qt6 component mapping

Widgets (consistent with `ARCH-003`).

| Proposal | Base | Responsibility |
|---|---|---|
| `AdjustmentsPanel` | `QDockWidget` | Host; resizable icon grid + panel menu. |
| `AdjustmentIconGrid` | `QListView` (icon mode) / `QToolButton` flow | Always-visible icons; click ⇒ create command. |
| `AdjustmentCatalogModel` | `QAbstractListModel` | Icon/label/enabled roles from `AdjustmentCatalog`. |
| `AdjustmentIconDelegate` | `QStyledItemDelegate` | Icon, hover/press, disabled tooltip. |
| `AdjustmentsPanelMenu` | `QMenu` | Choose adjustment; auto-select options. |
| `PropertiesPanel` (`PAN-006`) | `QDockWidget` | Parameter editor, Presets menu, Clip to Layer, Reset/Delete/Toggle. |
| `FillLayerDialogs` | `QDialog` | Solid Color / Gradient / Pattern create dialogs (`LAY-013`). |

The parameter editors (`LevelsEditor`, `CurvesEditor`, `HueSaturationEditor`, `ColorLookupEditor`, …) live in the Properties panel (`PAN-006`) and are shared with `04-image-ops/`.

## Data-model impact

- **No new persistent fields** beyond `LAY-012`: the panel creates existing `Adjustment`/`Fill` nodes.
- **Adjustment presets** are external assets/preferences, not document data, except the chosen Color Lookup LUT identity inside `clrL` (`LAY-012`).
- **Auto-select preferences** are application preferences.
- **Undo:** creation, parameter edits, clipping, visibility, reset and delete are commands (`ARCH-009`); live slider drags coalesce into one state.
- **Panel state** (last-used adjustment, panel width) is workspace/preference data, not document data.

## Edge cases

- **Unsupported adjustment for the mode:** bitmap/indexed disable all adjustment-layer icons; 32-bit Standard disables 32-bit-capable adjustments; the tooltip must explain why.
- **No layers below / adjustment at bottom:** creation succeeds but has no visible effect (`LAY-012`).
- **Invert:** has no editable settings; the Properties panel must show a read-only state, and the Adjustments panel must not present phantom controls.
- **Smart Object / no selection:** mask creation follows `LAY-012` (white mask, or selection/path-derived).
- **Color Lookup LUT missing:** degrade gracefully and report; do not crash (`LAY-012`).
- **Panel overflow:** a narrow dock must scroll or wrap the icon grid without hiding entries.
- **Undo across panel:** creating then immediately deleting an adjustment must net to no layer with a clean history.
- **GPU unavailable:** adjustments run on the CPU fallback (`LAY-012`).

## Parity acceptance criteria

1. Given CS6, `Window > Adjustments` shows the always-visible icon grid; clicking an icon creates an adjustment layer and opens its settings in the Properties panel.
2. Given each of the 16 adjustment types and 3 fill types, clicking its icon creates the matching layer type with a default mask (`LAY-012`).
3. Given a pixel selection before creating an adjustment, the new layer's mask matches the selection; given a selected path, a vector mask is created (`LAY-012`).
4. Given a CS6 build, no preset list is shown in the Adjustments panel; the seven preset-capable adjustments expose presets through the Properties panel Presets menu.
5. Given `Clip to Layer`, the adjustment affects only the layer below; clicking again applies it to all layers below.
6. Given `Reset`, the selected adjustment returns to its created defaults; given `Toggle Layer Visibility`, the composite toggles; given `Delete`/`Backspace`, the layer is removed.
7. Given `Auto-Select Parameter` / `Auto-Select Targeted Adjustment Tool` on, the corresponding control is focused by default.
8. Given `Alt`/`Option+2` and `Alt`/`Option+3/4/5`, composite/R/G/B channels are selected; with legacy shortcuts enabled, `Ctrl`/`Cmd+1/2/3` behave as R/G/B.
9. Given a 32-bit document in Standard edition, 32-bit adjustment creation is unavailable; in Extended it is available for the supported types.
10. Given an Invert adjustment, the Properties panel presents no editable parameters.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` (downloaded, `pdftotext -layout`) — official CS6 Help. Sections used: "Adjustments panel overview" (icon click selects an adjustment and creates a layer; non-destructive adjustment layers; CS5 presets list vs CS6 Properties Presets menu for Levels/Curves/Exposure/Hue/Saturation/Black&White/Channel Mixer/Selective Color; icons always visible in CS6; panel widen by corner drag); "Apply a correction using the Adjustments panel" (icon/menu create; toggle visibility; Reset; Delete This Adjustment Layer; CS5 arrow vs CS6 always-visible icons); "Apply a correction to only the layer below" (Clip to Layer); "Save and apply adjustment presets" (CS5 vs CS6 preset menu; Save Preset); "Automatically select text fields or the targeted adjustment tool" (Auto-Select Parameter / Auto-Select Targeted Adjustment Tool; `Shift+Enter`); "Using adjustment layers" (adjustment layers auto-created; properties in Properties panel CS6); "Adjustment and fill layers" (create/edit/confine; Color Range masks; Invert has no editable settings — see `LAY-012` for the full list); "Color adjustment commands" (panel is the flexible adjustment-layer path); "Keys for adjustment layers" (`Delete`/`Backspace`); "Keys for the Channels panel" legacy-shortcuts note (Use Legacy Channel Shortcuts); "What's new in CS6" / JDI list (no panel-specific History change; Color Lookup is CS6-new per community).
- `https://jkost.com/blog/2012/06/the-properties-panel-in-photoshop-cs6.html` — CS6 Properties panel shows the selected layer's properties and a mask icon; adjustment layers expose their controls there.
- `https://www.photoshopessentials.com/basics/using-the-enhanced-properties-panel-in-photoshop` — confirms the Properties panel is the contextual editor and that adjustment layers are edited there (article is CC-2020-era; used only for the CS6-era contextual-panel concept, not for CS6 control lists).
- `https://www.carrieacosta.com/class/shared/adjustment_layers_CS6.pdf` (**"Using Adjustment Layers", CS6 class handout**) — enumerates the CS6 Adjustment Layer buttons: Brightness/Contrast, Levels, Curves, Exposure, Vibrance, Hue/Saturation, Color Balance, Black & White, Photo Filters, Channel Mixer, Color Lookup, Invert, Posterize, Threshold, Selective Color and Gradient Map (16, including Color Lookup), and notes the panel-menu list.
- `http://www.photoshopforphotographers.com/3101-1901/Help_guide/tp/Adjustments_palette.html` (fetched) — CS6-era book companion: in CS6 the Adjustments panel only offers buttons for selecting and adding adjustment layers, with the adjustment controls moved to the new Properties panel.
- `https://www.apogeephoto.com/photoshop-cs6-cc-the-adjustments-panel-and-properties-panel` (fetched) — CS6/CC tutorial: pressing an Adjustments icon adds the adjustment layer and opens its controls in the Properties panel; tooltips name the icons.
- `https://www.photoshopessentials.com/basics/layers/essential-layers-panel-preferences` (fetched) — the Layers-panel "New Fill or Adjustment Layer" list leads with the three Fill layers (Solid Color, Gradient, Pattern); the Adjustments panel menu carries `Add Mask by Default`.
- `https://www.photoshopessentials.com/basics/managing-panels-in-photoshop-cs6` (fetched) — CS6 Essentials default workspace: Adjustments is the middle group of the main column, tabbed with Styles.

Consulted as search-result snippets only (not individually fetched; community): SearXNG query for "Photoshop CS6 Properties panel shape layer live properties type contextual".

Not used in this pass:

- `helpx.adobe.com` (HTTP 403) — modern Adjustments-panel help inaccessible; the archived CS6 Help PDF was used instead.

## Open questions

- **Exact icon order/count in CS6.** The Help does not enumerate the grid; the 16+3 list is assembled from the CS6 class handout and community sources, and the fill-first ordering is observed rather than documented. *Resolves with:* a CS6 capture.
- **Color Lookup in the CS6 Help.** The fetched Help prose does not name Color Lookup in the Adjustments panel although it is CS6-new (community) and the PSD spec has `clrL`. *Resolves with:* a CS6 Adjustments-panel screenshot.
- **Properties panel vs Adjustments panel split for presets** is sourced for the named seven types; whether any adjustment also kept an in-panel preset affordance is not. *Resolves with:* a CS6 capture.
- **Default `Auto-Select Parameter` / `Auto-Select Targeted Adjustment Tool` states** are not stated. *Resolves with:* a preferences dump.
- **`Add Mask by Default` presence in CS6.** Sourced from CS6/CC-era references; a CS6 capture would confirm. *Resolves with:* a CS6 screenshot.
- **32-bit availability per adjustment** is delegated to `LAY-012` / `04-image-ops/32-bit-hdr.md`.
