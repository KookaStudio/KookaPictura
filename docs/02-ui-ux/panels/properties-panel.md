# Properties Panel

- **Spec ID:** `PAN-006`
- **Status:** `Draft`
- **Parity tier:** `Core` (the panel ships in Standard and Extended; its 3D pages are Extended-only).
- **New in CS6:** `Yes` — the **Properties panel** is new in CS6. It is a **contextual** dock that shows settings for the current selection: adjustment-layer settings (moved out of the CS5 Adjustments panel), mask settings (the CS5 **Masks panel** was retired and amalgamated here), and the settings of the selected 3D element (Environment / Scene / Camera / Mesh / Material / Light). It also hosts the adjustment **Presets menu** and the `Clip to Layer` control. The old CS5 **Masks** and **Adjustments** panels no longer exist as separate editors.
- **Depends on:** `LAY-012` adjustment-layers, `LAY-004` layer-masks, `LAY-005` vector-masks-and-clipping-masks, `LAY-013` fill-layers, `LAY-011` layer-styles, `TOOL-061` 3d-panel, `TOOL-052` pen-and-path-tools, `SEL-003` refine-edge, `SEL-005` color-range, `ARCH-003` qt6-ui-design, `ARCH-008` document-model, `ARCH-009` undo-history, `PAN-001` layers-panel, `PAN-005` adjustments-panel.

> Crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. This file specifies the **contextual panel widget**;
> the semantics of each setting are owned by the layer, mask and 3D specs it
> cross-references.

## CS6 behavior

In CS6 the properties of some layer types appear in the Properties panel. It is a dock (`Window > Properties`) that swaps its content to match the selected layer/element. It combines settings that lived in separate CS5 panels and dialogs into one place.

**Default placement (CS6 Essentials workspace).** The panel sits in the **secondary (narrower) panel column to the left of the main one**, below the **History** panel, and opens in **icon view**. Double-clicking a layer opens the panel if it is hidden (Properties is not the default-active tab of any group). (Source: Photoshop Essentials, *Managing Panels In Photoshop CS6*; PFP book companion, *Properties panel*.)

### Adjustment settings

For an **adjustment layer**, the panel hosts the adjustment's parameter controls — the same controls CS5 showed in the Adjustments panel (`LAY-012`). Fill layers use their own content editors (the Solid Color picker, Gradient Fill dialog, or Pattern Fill dialog) instead of the Properties panel. It also carries the adjustment **Presets menu** for the seven preset-capable types (Levels, Curves, Exposure, Hue/Saturation, Black & White, Channel Mixer, Selective Color); clicking a preset applies it via an adjustment layer, and `Save Preset` adds user presets. The panel provides **Reset** (restore original settings), **Toggle Layer Visibility**, and **Delete This Adjustment Layer**, plus the **Clip to Layer** toggle (confine to the layer below; click again for all layers below), a **Previous State / before-after** toggle (press-and-hold the button or the `\` key to preview the pre-edit image), and the `Layer > Layer Content Options` / double-click-thumbnail reopen path. When nothing is selected the panel header reads **No Properties**. **Invert** adjustments have no editable settings.

The panel menu offers `Auto-Select Parameter` and `Auto-Select Targeted Adjustment Tool`, matching the CS6 Adjustments panel behavior (`PAN-005`).

### Mask settings (the retired Masks panel)

The CS5 **Masks panel** controls were amalgamated into the Properties panel. With a layer mask selected, the panel exposes:

- **Density** — mask opacity (100 % = fully blocking; lower reveals more).
- **Feather** — softens mask edges (CS6-era reference: up to a 1000 px radius).
- **Invert** — reverses masked/unmasked areas (also available in 32-bit in CS6).
- **Mask Edge** — edge-refinement controls such as Smooth and Contract/Expand (`SEL-003`).
- **Color Range** — build/refine the mask from sampled colors (`SEL-005`).
- **Disable/Enable Mask** (eye icon on the mask) — toggles the mask without deleting it; a red X overlays the Layers-panel thumbnail when disabled.
- **Load Selection from Mask** — reloads the mask's enclosed area as an active selection.
- **Apply Mask** / **Delete** — permanently apply the mask to the layer or remove it without applying. Note: a layer mask cannot be permanently applied when deleting it on a Smart Object layer.
- **Vector Mask** controls — `Add` / reveal-all / hide-all variants and Delete for vector masks (`LAY-005`), shared with the Paths panel.

Selecting the mask thumbnail in the **Layers panel** (a border appears around it) routes the Properties panel to these controls. The mask channel can also be viewed as grayscale or as a rubylith overlay via the Layers-panel thumbnail modifiers.

### 3D element settings (Extended-only)

When a 3D element is selected in the 3D panel (`TOOL-061`), the Properties panel shows its settings: **Environment** (global ambient, image-based lights, ground-plane shadows/reflections), **Scene** (render presets such as Bounding Box and Wireframe; cross-sections, surfaces, points), **Camera** (FOV, depth of field, stereo), **Mesh** (catch/cast shadows, extrusion, edit source), **Material** (texture/bump maps), and **Light** (type, colour, intensity, shadows). A **Coordinates** icon at the top toggles precise numerical properties for objects, cameras and lights; the `V` key cycles between properties and coordinates. A **Render** icon at the bottom commits the scene. 3D settings are gated by `Edition::Extended`.

### Shape / live properties and type

The panel's content is contextual, so shape and type layers likewise surface their relevant properties. Two claims that were previously open are now resolved:

- **Shape layers (CS6): no editable shape geometry in the panel.** A CS6 shape layer carries a vector mask and the panel can expose the vector-mask controls (`LAY-005`); the shape layer's path is also available in the Paths panel. The per-shape **Live Shape Properties** page (fill, stroke, stroke width, width/height/position, and per-corner rectangle radii) is a **CC 2013** feature, not CS6: the CS6-era Adobe community thread "Live Shape properties not showing in Photoshop cs6" confirms its absence, and the PFP book documents it under the CC 2013 guide. CS6's panel is therefore expected to show the mask/vector-mask page (or read-only state) for a shape layer rather than geometry fields.
- **Type layers (CS6): edited via Character/Paragraph, not the panel.** In CS6 type is edited through the **Character** and **Paragraph** panels, the options bar and the `Type` menu; folding the full Character/Paragraph option sets into the Properties panel is a **CC 2020** enhancement, not CS6.

The shape/type page contents remain a design proposal pending a CS6 capture, but the *absence* of Live Shape geometry and full type controls in CS6 is now sourced.

### Relationship to the old Masking / Adjustments panels

- **Masks panel (CS4/CS5):** retired in CS6; its controls live in the Properties panel (and, for mask creation, in the Layers panel and `Layer > Layer Mask`). The Adobe-era book source states plainly that in Photoshop CS6 the Masks panel controls have been amalgamated into the Properties panel.
- **Adjustments panel (CS5):** retained in CS6 as the **icon launcher** only; the parameter controls and presets moved to the Properties panel (`PAN-005`). CS6's Adjustments panel keeps its icons always visible.
- **3D panel (Extended):** retained as the element selector; settings moved to the Properties panel (`TOOL-061`).

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Window > Properties` | Menu → panel | — | Display the panel. |
| Properties panel, top | Mask / vector-mask / adjustment tabs or icons | — | Switch between the layer content and its mask. |
| Properties panel | Parameter controls | — | Contextual per selected node. |
| Properties panel | Presets menu | — | Seven preset-capable adjustments (`LAY-012`). |
| Properties panel | Clip to Layer | — | `LAY-012`; also `Ctrl/Cmd+Alt+G`. |
| Properties panel | Reset / Toggle Visibility / Delete / Previous State | `\` | Adjustment layer affordances; `\` (hold) previews the pre-edit state. |
| Properties panel | Density / Feather / Invert / Mask Edge / Color Range | — | Mask controls (`LAY-004`, `PAN-006`). |
| Properties panel | Disable/Enable, Load Selection, Apply Mask, Delete Mask | — | Mask lifecycle. |
| Properties panel | Vector Mask buttons | — | Add reveal-all/hide-all; delete (`LAY-005`). |
| Properties panel | 3D Environment/Scene/Camera/Mesh/Material/Light | `V` | Extended only; coordinates toggle (`TOOL-061`). |
| Properties panel | Render | — | Commits the 3D scene. |
| Properties panel menu | Menu | — | `Auto-Select Parameter`; `Auto-Select Targeted Adjustment Tool`; `Save Preset`. |
| Layers panel mask thumbnail | Select mask | click | Routes the panel; Shift/Alt click toggles mask view. |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Active page | enum | by selection | Layer content / Mask / 3D element | Contextual swap. |
| Density | percent | 100 | 0–100 | Mask opacity (`LAY-004`). |
| Feather | px | 0 | 0–1000 | Mask edge softening (CS6-era reference radius cap). |
| Invert | bool | off | on / off | Reverses mask; CS6 also in 32-bit. |
| Mask Edge | action | — | Smooth, Contract/Expand, … | `SEL-003`. |
| Color Range | dialog | — | Fuzziness, Range, Localized Color Clusters | `SEL-005`. |
| Enable/Disable mask | bool | on | on / off | Red X when disabled. |
| Load Selection | action | — | — | Reload the mask as an active selection. |
| Apply / Delete mask | action | — | — | Apply is refused for Smart Object layer masks. |
| Previous State (`\`) | action | — | hold | Pre-edit preview. |
| Clip to Layer | bool | off | on / off | Adjustment/fill layers. |
| Adjustment preset | enum | none | per preset-capable type | Applies via an adjustment layer. |
| Coordinates (3D) | bool | off | on / off | `V`; Extended only. |

Per-adjustment parameter ranges are owned by `04-image-ops/adjustments/*`; per-mask geometry by `LAY-004`/`SEL-003`.

## Algorithms & pipeline

The panel is a **router**: it inspects the current selection and binds a page to the selected object's model.

1. **Selection → page.** A `PropertiesTarget` is derived from the current Layers/3D selection: `Adjustment(id)`, `Fill(id)`, `LayerMask(id)`, `VectorMask(id)`, `Shape(id)`, `Type(id)`, `ThreeDElement(node)`, `LayerStyle(id)`, or `None`.
2. **Bidirectional binding.** The panel reads the target's model and writes parameter changes back as commands; it never caches authoritative state. Changes re-render through the compositor's dirty-tile pass (`LAY-012`, `ARCH-006`).
3. **Mask controls** delegate to the mask engine (`LAY-004`); Density/Feather are non-destructive parameters, Invert is a flag, Color Range invokes `SEL-005`.
4. **Preset application** creates/updates an adjustment layer via the same command as `PAN-005`/`LAY-012`.
5. **3D binding** marshals a `SceneNodeId` to the 3D pages and writes through `pictura_3d` (`TOOL-061`).
6. **Coalescing.** Live drags (sliders, 3D coordinates) coalesce into one undo state on release (`ARCH-009`).

## Rust module mapping

Design proposal.

- `pictura_ui_bridge::PropertiesTarget` — enum of what the panel is bound to; derived from the selection.
- `pictura_ui_bridge::PropertiesController` — reads the target's fields, emits `SetAdjustmentParams` / `SetMaskDensity` / `SetMaskFeather` / `SetMaskInvert` / `SetClipping` / `ApplyMask` / `DeleteMask` / `SetThreeDSetting` commands.
- `pictura_core::document::adjustment_ops` — adjustment create/edit/reset/delete (`LAY-012`).
- `pictura_core::mask` — density/feather/invert/apply (`LAY-004`).
- `pictura_core::vector` / `path_ops` — vector-mask controls (`LAY-005`, `PAN-003`).
- `pictura_adjust::preset` — the seven preset-capable types.
- `pictura_3d::scene` — Environment/Scene/Camera/Mesh/Material/Light read/write (`TOOL-061`).
- `pictura_core::edition` — Extended gate for 3D pages.

Crossing types: `NodeId`, `SceneNodeId`, `AdjustmentParams`, `MaskParams`, `Edition`, `PresetId`. No Qt types in `pictura_core`.

## Qt6 component mapping

Widgets, consistent with `ARCH-003` and with `PAN-005` for the adjustment editors.

| Proposal | Base | Responsibility |
|---|---|---|
| `PropertiesPanel` | `QDockWidget` | Host; routes pages on selection change. |
| `PropertiesStack` | `QStackedWidget` | One page per `PropertiesTarget` variant. |
| `MaskPropertiesWidget` | `QWidget` | Density, Feather, Invert, Mask Edge, Color Range, Enable/Disable, Apply/Delete. |
| `VectorMaskPropertiesWidget` | `QWidget` | Add reveal-all/hide-all, Delete (`LAY-005`). |
| `AdjustmentPropertiesPage` | `QWidget` | Hosts the per-adjustment editors + Presets menu + Clip to Layer. |
| `AdjustmentEditor` subclasses | `QWidget` | `LevelsEditor`, `CurvesEditor`, `HueSaturationEditor`, `SelectiveColorEditor`, `ColorLookupEditor`, etc. (shared with `PAN-005`). |
| `ThreeDPropertiesPage` | `QWidget` | Environment/Scene/Camera/Mesh/Material/Light + Coordinates (`V`) + Render. |
| `PropertiesMenu` | `QMenu` | `Auto-Select Parameter`, `Auto-Select Targeted Adjustment Tool`, `Save Preset`. |

## Data-model impact

- **No new persistent fields.** The panel edits existing `Node` adjustment params, mask params, clipping flag, and 3D scene settings (`ARCH-008`, `LAY-012`, `LAY-004`, `TOOL-061`).
- **The panel is a view/router**; it holds no authoritative state and stores only transient UI position/size and the auto-select preferences.
- **Undo:** every committed change is the corresponding command record (`ARCH-009`); live drags coalesce.
- **Old-panel migration:** there is no CS5 "Masks panel" state to migrate — its settings were already node fields.
- **3D pages** are edition-gated; files round-trip the 3D scene even in a Standard build (`OVR-002`/`TOOL-061`).

## Edge cases

- **No selection / unsupported layer kind:** the panel shows an empty or read-only page rather than stale settings.
- **Mask on a Smart Object:** Apply/Delete of the mask is refused (Help note); the control disables with a reason.
- **Disabled mask:** the panel indicates the disabled state (red X in the Layers panel) and keeps the settings editable.
- **Multiple selected layers:** the panel must define whether it shows the active target only or a common subset; CS6's exact multi-select Properties behavior is not documented in the fetched text — see Open questions.
- **Adjustment with no editable settings (Invert):** read-only page.
- **3D without OpenGL / in Standard edition:** 3D pages unreachable (`TOOL-061`).
- **Color Range with no mask:** the action is disabled or first creates a mask, per `SEL-005`.
- **32-bit / CMYK / Lab:** adjustment and mask controls follow each spec's gating (`LAY-012`, `LAY-004`).
- **Huge PSB:** mask edge/color-range operations stream per tile and respect the memory cap.
- **Undo/redo:** switching selection must flush/commit any in-progress live edit so an undo returns to a well-defined state.

## Parity acceptance criteria

1. Given an adjustment layer is selected, the Properties panel shows its parameters, the Presets menu for preset-capable types, `Reset`, `Toggle Layer Visibility`, `Delete`, and `Clip to Layer`.
2. Given a layer mask thumbnail is selected, the panel shows Density, Feather, Invert, Mask Edge and Color Range, and Disable/Enable, Apply and Delete.
3. Given Density `D` and Feather `F`, the mask's effective opacity/edge change accordingly without editing pixels, matching `LAY-004`.
4. Given a mask on a Smart Object layer, Apply/Delete is disabled with an explanatory state (not silently failing).
5. Given a 3D element selected in the 3D panel (Extended), the panel swaps to the matching page and `V` toggles properties ↔ coordinates; Render commits the scene.
6. Given `Clip to Layer` on an adjustment, only the layer below changes; clicking again widens it to all layers below.
7. Given a preset chosen from the Presets menu, an adjustment layer is created/updated with those settings within the adjustment's tolerance.
8. Given `Auto-Select Parameter` on, the parameter text field is focused when the adjustment page opens.
9. Given a selection change from an adjustment to a pixel layer, no adjustment settings leak and the page reflects the new target.
10. Given a CS6 PSD containing a 3D scene opened in a Standard build, the panel shows no 3D page and the scene round-trips unmodified.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` (downloaded, `pdftotext -layout`) — official CS6 Help. Sections used: "Properties panel provides contextual settings" (3D element settings after selection in the 3D panel; Coordinates icon and `V`; Render icon; Environment/Scene/Camera/Mesh/Material/Light summary) and the CS6 3D workflow (Properties panel replaces the CS5 panel split); "Masks panel (CS5) and Properties panel (CS6)" (the panel "provide[s] additional controls to adjust a mask" — opacity/Density, Feather, Invert, Mask Edge, Color Range; Disable/Enable; Apply/Delete; the Smart Object apply restriction; vector-mask controls); "Adjusting mask opacity and edges" (Density/Feather/Invert/Mask Edge, Color Range); "Add layer masks" / "Apply a deletion of a layer mask" (mask lifecycle); "Adjustments panel overview" (CS6 puts the preset menu in the Properties panel; Clip to Layer; Reset/Toggle/Delete); "Save and apply adjustment presets" (CS6 Preset menu); "Automatically select text fields or the targeted adjustment tool" (Properties panel menu); "Using adjustment layers" (CS6 properties in the Properties panel); "What's new in CS6 > Layers enhancements" (the Properties panel quickly modifies the layer components selected in the Layers panel).
- `https://jkost.com/blog/2012/06/the-properties-panel-in-photoshop-cs6.html` — Adobe's Julieanne Kost: CS6 Properties panel shows the selected layer's properties and a mask icon at the top; can be resized both ways.
- `http://www.photoshopforphotographers.com/3101-1901/Help_guide/tp/Paths_palette.html` — used only for the CS6-era statement that a path can be a vector mask and be saved as a clipping path; vector-mask controls live in the Properties panel in CS6.
- `https://www.photoshopessentials.com/basics/using-the-enhanced-properties-panel-in-photoshop` — the Properties panel is the contextual editor for the selected layer; type's full Character/Paragraph integration is explicitly a **CC 2020** enhancement (used to separate CS6 behavior from later versions).
- `http://www.photoshopforphotographers.com/3101-1901/Help_guide/tp/propertiespanel.html` (fetched) — CS6-era book companion: the panel has an "adjustment controls" mode (default when an adjustment layer is created) and a "mask controls" mode; double-clicking a layer opens the panel if hidden; the mask mode exposes pixel/vector mask buttons, Density, Feather (up to 1000 px), Mask Edge, Color Range, Invert, and Load-Selection/Apply/Delete buttons; the adjustment mode exposes Presets, the parameter controls, Clip to Layer, a before/after **Previous State** toggle (`\`), Reset, Toggle Visibility and Delete; `Shift+Return` enters panel edit mode and `Esc` exits.
- `https://www.apogeephoto.com/photoshop-cs6-cc-the-adjustments-panel-and-properties-panel` (fetched) — CS6/CC tutorial enumerating the adjustment-view functions (Toggle Layer Visibility, Reset to Default, Delete Layer, Clip to Layer, Previous State) and the mask-view functions (Mask Density, Mask Edge, Color Range, Feather Edge, Invert Selection, Delete Mask, Disable/Enable Mask, Load Selection from Mask).
- `https://www.photoshopessentials.com/basics/managing-panels-in-photoshop-cs6` (fetched) — CS6 Essentials default workspace: Properties shares the narrow secondary column with History, below it.
- `https://community.adobe.com/t5/photoshop-ecosystem-discussions/live-shape-properties-not-showing-in-photoshop-cs6/m-p/11035207` and `http://www.photoshopforphotographers.com/CC_2013/Help_guide/PDFs/LiveShapeProperties.pdf` — confirm that per-shape Live Shape Properties (corner radii, width/height, etc.) are a CC 2013 feature absent from CS6.

Consulted as search-result snippets only (not individually fetched; community):

- SearXNG query for "Photoshop CS6 Properties panel shape layer live properties type contextual" — indicated the Properties panel is CS6-new and context-sensitive, and that per-corner "Live Shape" editing and full Character/Paragraph in the panel are later CC revisions.
- SearXNG query for "Photoshop CS6 Properties panel type layer character options".

Not used in this pass:

- `https://help.adobe.com` / `helpx.adobe.com` (403) — modern Properties-panel help inaccessible; the archived CS6 Help PDF was used instead.
- `https://graphicdesign.stackexchange.com/questions/7197/...` (403/transport errors).

## Open questions

- **Multiple-selection behavior.** What the panel shows when several heterogeneous layers are selected is not stated. *Resolves with:* a CS6 test.
- **Pixel-layer / Background pages.** CC added Transform, Align, and Quick Actions pages for these layers; whether CS6 had any (e.g. read-only document info) is not in the fetched text. *Resolves with:* a CS6 capture.
- **Page set completeness.** The full list of CS6 `PropertiesTarget` variants (including layer styles, Smart Objects, artboards) is inferred from the contextual-panel concept rather than enumerated by the Help. *Resolves with:* a CS6 workspace capture.
- **Exact shape-layer page in CS6.** That CS6 lacks Live Shape geometry is now sourced; whether a selected shape layer shows an empty/read-only page or the vector-mask page needs a CS6 capture.
- **Whether the CS6 Adjustments panel retains any in-panel preset affordance** in addition to the Properties Presets menu — see `PAN-005`.
