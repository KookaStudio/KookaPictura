# Smart Object and Mask Panels

- **Spec ID:** `PAN-026`
- **Status:** `Draft`
- **Parity tier:** `Core` — layer masks, vector masks, Smart Objects, and Smart Filters are in both CS6 editions. Image-stack Stack Modes (a Smart Object surface) are `Extended-only`.
- **New in CS6:** `Changed` — the CS5 **Masks panel was retired** and its controls moved into the new **Properties panel** (mask mode; `PAN-006`). Clicking the mask thumbnail in the Layers panel now activates the mask (CS5 used the Masks-panel Pixel Mask button). CS6 also enables **Invert** and **Threshold** for masks in 32-bit/channel images, and adds two new smart-filter-capable filters (**Blur Gallery**, **Oil Paint**). The CS5 Help's "Masks panel (CS6)" wording survives in the CS6 PDF as an artifact (see Open questions).
- **Depends on:** `PAN-006` `02-ui-ux/panels/properties-panel.md` (the contextual router; **this spec is the mask/smart-object relationship surface, not a second panel**), `PAN-001` `02-ui-ux/panels/layers-panel.md` (mask/smart-filter thumbnails and badges), `LAY-004` `05-layers/layer-masks.md`, `LAY-005` `05-layers/vector-masks-and-clipping-masks.md`, `LAY-020` `05-layers/smart-objects.md`, `LAY-021` `05-layers/smart-filters.md`, `SEL-003` `08-selection/refine-edge.md`, `SEL-005` `08-selection/color-range.md`, `01-architecture/document-model.md` (`ARCH-008`), `01-architecture/undo-history.md` (`ARCH-009`).

> All crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Behavior is taken from the fetched CS6 Help PDF unless
> marked *(inferred)*. The mask and Smart Object **semantics** are owned by
> `LAY-004`/`LAY-005`/`LAY-020`/`LAY-021`; this file specifies the **UI surfaces
> and the Properties-panel mask/property relationship**.

## CS6 behavior

### The Mask panel is the Properties panel in mask mode

CS6 has **no separate Masks panel**. Its controls were amalgamated into the **Properties panel** (`PAN-006`), which is a contextual router: the page it shows follows the current selection. The CS6 Help describes the controls with duality wording — the Properties panel (CS6) or the Masks panel (CS5) provides further controls for adjusting a mask — and its figure caption still reads *"Masks panel (CS5) and Properties panel (CS6)"*.

**Activating mask mode.** Click the **mask thumbnail** in the Layers panel; a border appears around the active thumbnail and the Properties panel swaps to the mask controls. Clicking the layer thumbnail returns the panel to layer content. *(The Help's figure also shows a mask/vector-mask/filter-mask selector and buttons A–G: select filter mask, add pixel mask, add vector mask, panel menu, Apply Mask, layer mask, vector mask.)*

**Mask controls (documented):**

- **Density** — mask opacity. At **100%** the mask is fully opaque and blocks the underlying area; lowering density reveals more of the area under the mask.
- **Feather** — blurs the mask edges for a softer transition, applied from the mask edges outward within the set pixel range.
- **Invert** — reverses masked/unmasked areas (layer masks only, not vector masks). CS6 enables Invert (and **Threshold**) for masks in 32-bit/channel images.
- **Mask Edge** — opens the **Refine Mask** dialog, the same option set as Refine Edge (`SEL-003`): View Mode, Refine/Erase Radius brushes, Smart Radius, Radius, Smooth, Feather, Contrast, Shift Edge, Decontaminate Colors (+ Amount), Output To. **Decontaminate Colors changes pixel color** and forces output to a new layer/document.
- **Color Range** — confines/refines the mask by sampled color (`SEL-005`).
- **Disable/Enable Mask** — the eye icon toggles the mask without deleting it; a red X overlays the Layers-panel thumbnail when disabled.
- **Apply Mask** / **Delete** — apply the mask permanently to the layer, or delete it without applying. **A layer mask cannot be permanently applied when deleting it on a Smart Object layer.**
- **Vector Mask** controls — add reveal-all/hide-all and delete for vector masks (`LAY-005`), shared with the Paths panel.
- **Add Mask by Default** — the Adjustments/Properties panel menu controls whether adjustment/fill layers auto-receive a mask (`LAY-004`).

**Display and rubylith** are driven from the Layers-panel thumbnail (`Alt`/`Option`-click = grayscale mask; `Alt+Shift`/`Option+Shift`-click = rubylith overlay; double-click the mask channel for **Layer Mask Display Options**), owned by `LAY-004`.

### Smart Object UI surfaces

A Smart Object is a container layer (badge on its thumbnail) that renders an embedded/linked source through a transform and a Smart Filter stack (`LAY-020`, `LAY-021`). Its UI surfaces are distributed, not a single panel:

- **Layers panel** — the Smart Object badge, the layer mask thumbnail, and, nested under the layer, the **Smart Filter stack** rows (each Smart Filter shows its own **filter-mask thumbnail**, blend mode, and opacity). Double-clicking the layer thumbnail = **Edit Contents**.
- **`Layer > Smart Objects` menu** — Convert to Smart Object, New Smart Object Via Copy, Edit Contents, Replace Contents, Export Contents, Stack Mode (Extended), Rasterize (`LAY-020`).
- **Properties panel (proposed page; `PAN-006` open question)** — a contextual Smart Object page summarizing the source (format, page, embedded/linked), with **Edit/Replace/Export Contents**, **Rasterize**, and the Extended **Stack Mode** selector. The fetched CS6 Help does **not** document a dedicated Smart Object Properties page; treat this as *(inferred)* and consistent with the contextual-panel concept, not as a sourced CS6 contract.
- **`File > Place` / `Open As Smart Object`** — placement workflows that create the object (`LAY-020`).
- **`Filter` menu** — smart-compatible filters apply non-destructively to the selected Smart Object (CS6 adds Blur Gallery and Oil Paint to the smart-capable set per `LAY-021`).

### Smart Filter and filter-mask surface

Smart Filters appear under the Smart Object layer. The CS6 Help (in CS5-era wording that names the Masks panel) documents the filter-mask surface:

- **Filter mask** — like a layer mask, a filter mask can be painted. Black paint hides the filter's effect, white paint reveals it, and gray paint yields partial transparency.
- **Density / Feather / Invert** — the Masks panel controls set the filter mask's density, add feathering along its edges, or invert it. (In CS6 these are the Properties-panel mask controls.)
- **Mask Edge is not available for filter masks.**
- **Enable/disable** — Shift-click the filter-mask thumbnail, or the Disable/Enable Mask button; a red X appears when disabled.
- **Move/copy** — drag a filter mask to another Smart Filter Effect; `Alt`/`Option`-drag copies it.
- **Delete** — the Delete icon, dragging the thumbnail to the Delete icon, or `Layer > Smart Filters > Delete Filter Mask`.
- **Display only the filter mask** — `Alt`/`Option`-click its thumbnail.
- **Filter stack editing** — reorder, hide, delete Smart Filters, and **Clear Smart Filters** (`LAY-021`).

### Properties-panel mask/property mode relationship

The panel is one dock with multiple pages (`PAN-006`). The mask relationship is:

| Selection | Properties-panel mode |
|---|---|
| Adjustment/fill layer content | Adjustment parameter page + Presets menu (`PAN-005`, `LAY-012`) |
| Layer mask thumbnail (or active mask) | Mask page: Density, Feather, Invert, Mask Edge, Color Range, Enable, Apply/Delete |
| Vector mask thumbnail | Vector-mask page: Reveal/Hide All, Delete (`LAY-005`) |
| Filter mask thumbnail | Filter-mask page: Density, Feather, Invert, Enable, Delete (**no Mask Edge**) |
| Smart Object layer | Proposed Smart Object page: source summary, Edit/Replace/Export, Rasterize, Stack Mode *(inferred)* |
| 3D element (Extended) | 3D page (`TOOL-061`) |
| Pixel/other layer | Layer content page or empty |

The **eye icon** toggles mask visibility, the **mask selector** chooses layer/vector/filter mask, and the **Apply/Delete** affordances act on the active mask. Selecting a different thumbnail in the Layers panel re-routes the page; no mask state persists in the panel itself.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Window > Properties` | Menu → dock | — | Host for mask/smart-object pages (`PAN-006`) |
| Layers panel — mask thumbnail | Select | click | Routes Properties panel to mask mode; border shown |
| Layers panel — mask thumbnail | Grayscale view | `Alt`/`Option`-click | `LAY-004` |
| Layers panel — mask thumbnail | Rubylith view | `Alt+Shift`/`Option+Shift`-click | `LAY-004` |
| Layers panel — mask thumbnail | Disable/enable | `Shift`-click | Red X when disabled |
| Layers panel — mask channel | Display Options | double-click | Rubylith color/opacity (display only) |
| Layers panel — Smart Object thumbnail | Badge | — | Marks a Smart Object (`LAY-020`) |
| Layers panel — Smart Object thumbnail | Edit Contents | double-click | Opens source document |
| Layers panel — Smart Filter row | Filter-mask thumbnail | `Alt`/`Option`-click = view only; `Shift`-click = disable | `LAY-021` |
| Properties panel — mask page | Density / Feather / Invert / Mask Edge / Color Range | — | `LAY-004`, `SEL-003`, `SEL-005` |
| Properties panel — mask page | Disable/Enable (eye) | — | Toggle without deleting |
| Properties panel — mask page | Apply Mask / Delete | — | Apply refused on Smart Object layer masks |
| Properties panel — vector-mask page | Add reveal/hide all; Delete | — | `LAY-005` |
| Properties panel — filter-mask page | Density / Feather / Invert; Delete | — | No Mask Edge |
| Properties panel (proposed) — Smart Object page | Edit/Replace/Export Contents, Rasterize, Stack Mode | — | *(inferred)*; Stack Mode Extended-only |
| `Layer > Smart Objects` | Menu | — | Convert/New via Copy/Edit/Replace/Export/Stack Mode/Rasterize |
| `Layer > Smart Filters` | Menu | — | Disable/Delete Filter Mask, Clear Smart Filters |
| `Layer > Layer Mask` / `Layer > Vector Mask` | Menu | — | Full mask lifecycle (`LAY-004`/`LAY-005`) |
| Filter menu | Menu | — | Smart-compatible filters apply as Smart Filters |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Density | percent | 100 | 0–100 | Mask opacity (`LAY-004`) |
| Feather | px | 0 | ≥ 0 | Edge blur, outward within range |
| Invert | bool | off | on/off | Layer masks only; CS6 adds 32-bit support |
| Mask Edge | dialog | — | Refine Edge set (`SEL-003`) | **Not available for filter masks** |
| Color Range | dialog | — | Fuzziness / Range / Localized Color Clusters (`SEL-005`) | — |
| Enable/Disable | bool | on | on/off | Eye / Shift-click; red X when disabled |
| Apply / Delete mask | action | — | — | Apply refused when deleting on a Smart Object layer |
| Vector mask add | enum | — | Reveal All / Hide All / From Path | `LAY-005` |
| Add Mask by Default | bool | on *(assumed)* | on/off | Panels menu (`LAY-004`) |
| Rubylith color/opacity | color / percent | red / 50% *(not stated)* | 0–100% | Display only (`LAY-004`) |
| Filter-mask Density/Feather | percent / px | 100 / 0 | as above | Applies to the filter, not the layer |
| Filter-mask Invert | bool | off | on/off | — |
| Smart Filter blend mode | enum | Normal | 27 CS6 blend modes | Per Smart Filter (`LAY-021`) |
| Smart Filter opacity | percent | 100 | 0–100 | Per Smart Filter |
| Stack Mode | enum | None | None + 11 modes | Extended-only (`LAY-020`) |

## Algorithms & pipeline

This spec is a **surface/relationship layer**; the algorithms are owned elsewhere.

1. **Contextual routing.** The Properties panel derives a `PropertiesTarget` from the Layers/3D selection and binds the matching page (`PAN-006`). Mask targets are `LayerMask(id)`, `VectorMask(id)`, and `FilterMask(smart_filter_id)`.
2. **Mask evaluation.** Density/Feather/Invert are non-destructive parameters on the mask record, multiplied into the layer or filter contribution at composite time (`LAY-004`, `ARCH-006`). Filter masks gate the Smart Filter's output the same way (`LAY-021`).
3. **Mask Edge** invokes the shared Refine Edge pipeline (`SEL-003`); **Color Range** invokes `SEL-005`.
4. **Smart Object rendering.** The Smart Object renders its embedded source through the transform, then applies the Smart Filter stack (each filter optionally masked) (`LAY-020`, `LAY-021`).
5. **Smart Filter application.** Filters from the Filter menu that support smart application are pushed onto the stack as non-destructive entries rather than baked (`LAY-021`).
6. **Undo.** Each committed mask or Smart Filter change is the corresponding command (`ARCH-009`); live slider drags coalesce.

## Rust module mapping

The panel layer is thin; the model comes from the layer specs:

- `pictura_ui_bridge::PropertiesTarget` — extended with `FilterMask(SmartFilterId)` alongside the `PAN-006` variants.
- `pictura_ui_bridge::MaskController` — emits `SetMaskDensity` / `SetMaskFeather` / `SetMaskInvert` / `ApplyMask` / `DeleteMask` / `SetMaskEnabled` (`LAY-004`).
- `pictura_core::mask` — `RasterMask { density, feather, enabled, linked }`, `MaskKind::Filter` (`LAY-004`).
- `pictura_core::smart` — `SmartObject`, `SmartFilterStack` (`LAY-020`, `LAY-021`).
- `pictura_filter::smart` — Smart Filter evaluation + per-filter mask (`LAY-021`).
- `pictura_core::vector` / `path_ops` — vector-mask controls (`LAY-005`).

Crossing types: `NodeId`, `MaskRef`, `MaskKind`, `SmartFilterId`, `MaskParams`, `StackMode`. No Qt types cross into the core.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `PropertiesStack` | `QStackedWidget` | Pages (reused from `PAN-006`) |
| `MaskPropertiesWidget` | `QWidget` | Density, Feather, Invert, Mask Edge, Color Range, Enable, Apply/Delete |
| `VectorMaskPropertiesWidget` | `QWidget` | Reveal/Hide All, Delete (`LAY-005`) |
| `FilterMaskPropertiesWidget` | `QWidget` | Density, Feather, Invert, Enable, Delete; **no** Mask Edge |
| `SmartObjectPropertiesWidget` | `QWidget` | *(proposed)* Source summary, Edit/Replace/Export, Rasterize, Stack Mode |
| `SmartFilterStackModel` | `QAbstractItemModel` | Smart Filter rows + per-filter blend/opacity/mask (`LAY-021`) |
| `MaskThumbnailDelegate` | `QStyledItemDelegate` | Mask/filter-mask thumbnails, active border, red X, link icon (`PAN-001`) |
| `RefineMaskDialog` / `ColorRangeDialog` | `QDialog` | Reuse `SEL-003` / `SEL-005` dialogs |

Widgets over QML (dense docked controls, model/view; `ARCH-003`). The Layers panel owns the thumbnails/badges; the Properties panel owns the editors; both bind to the same Rust model, so no state is duplicated.

## Data-model impact

- **No second panel state.** The Masks panel has no persistent state to migrate; its settings were already `Node`/mask fields (`PAN-006`).
- **Mask records.** Layer/vector/filter mask parameters, enabled/invert flags, and density/feather round-trip per `LAY-004`/`LAY-005` (PSD mask-records, flag bit 4 for density/feather).
- **Smart Object/filter data.** The embedded source, transform, Smart Filter stack, filter-mask block (`FMsk`), and effects ids (`FXid`/`FEid`) round-trip per `LAY-020`/`LAY-021`; unknown descriptor fields are preserved opaquely.
- **Undo.** One command per committed mask/Smart Filter change (`ARCH-009`); apply/delete needs pixel backups; transform/reorder are non-destructive.
- **Edition gating.** Stack Mode is Extended-only; the proposed Smart Object Properties page must hide it in a Standard build (`OVR-002`).
- **Folder/collection gotcha.** The Help uses "Masks panel (CS6)" in a few procedures; this is a CS5 artifact — the model has no "Masks panel" object to serialize.

## Edge cases

- **Mask on a Smart Object layer** — Apply/Delete of the mask is refused; the control disables with a reason (`LAY-004`).
- **Filter mask + Mask Edge** — Mask Edge must be absent/disabled for filter masks (sourced).
- **Disabled mask/filter mask** — settings remain editable; the red X in the Layers panel communicates state.
- **No mask selected** — mask controls are hidden/disabled rather than acting on the layer; switching selection re-routes the page.
- **Both user and vector masks** — the panel must target the active one and not conflate them; the PSD `-3` channel rule applies (`LAY-004`).
- **Adjustment/fill layer auto-mask** — `Add Mask by Default` governs creation; toggling it is a panel-menu preference, not a document change.
- **32-bit masks** — Invert/Threshold enabled in CS6; density/feather remain meaningful.
- **CMYK/Lab/Gray** — masks are mode-independent; color operations use the working space.
- **Smart Filter on an unsupported mode/depth** — the filter row shows the warning icon (`LAY-021`); the filter-mask page still edits density/feather/invert.
- **Clear Smart Filters / delete filter** — removes the filter and its mask; undo must restore both.
- **Reorder Smart Filters** — filter masks travel with their filter (Help: masks can be moved/copied between effects).
- **Huge PSB** — mask edge/refine streams per tile; density/feather must not materialize the whole canvas.
- **GPU unavailable** — mask multiply and Smart Filter evaluation fall back to the CPU reference.
- **Undo/redo** — switching the Properties target flushes any in-progress live edit first (`PAN-006`).

## Parity acceptance criteria

1. Given a layer mask thumbnail is selected, the Properties panel shows Density, Feather, Invert, Mask Edge, Color Range, Enable/Disable, and Apply/Delete; selecting the layer thumbnail returns to layer content.
2. Given Density `D` and Feather `F`, the mask's effective opacity/edge change without editing pixels, matching `LAY-004`.
3. Given a mask on a Smart Object layer, Apply/Delete is disabled with an explanation.
4. Given `Layer > Smart Filters` → filter mask selected, the panel shows Density/Feather/Invert/Enable/Delete and **no** Mask Edge.
5. Given a filter mask painted black/white/gray, the filter is hidden/revealed/partially visible; Alt/Option-clicking the thumbnail shows only the mask.
6. Given two Smart Filter Effects, dragging a filter mask onto the other moves it; Alt/Option-drag copies it.
7. Given a CS6 PSD with a user mask plus a vector mask, both affect the composite, both routes exist in the panel, and both round-trip.
8. Given a Smart Object (Extended), `Stack Mode` is offered; in a Standard build the page is absent and the scene round-trips unmodified.
9. Given an adjustment/fill layer with `Add Mask by Default` on, creating it also creates a layer mask; off, it does not.
10. Given `Refine Mask` from a layer mask, the `SEL-003` option set applies and Decontaminate Colors forces a new-layer/document output.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — primary CS6 Help corpus (downloaded, text-extracted). Established: the CS6 move of mask controls from the CS5 Masks panel to the **Properties panel** and the "Properties panel (CS6) or the Masks panel (CS5)" duality wording; the figure caption "Masks panel (CS5) and Properties panel (CS6)" with the A–G parts (select filter mask, add pixel mask, add vector mask, panel menu, Apply Mask, layer mask, vector mask); Density (100% fully blocks; lower reveals), Feather (edge blur from the mask edges outward), Invert (layer masks only; CS6 adds 32-bit Invert/Threshold), **Mask Edge** → **Refine Mask** dialog, Color Range; selecting the mask thumbnail to activate mask editing; Disable/Enable via eye/Shift-click and the red X; Apply/Delete and the **Smart Object apply restriction**; the vector-mask controls; Add Mask by Default; the filter-mask paint semantics (black hides / white shows / gray partial), Density/Feather/Invert, **Mask Edge unavailable for filter masks**, enable/disable, move/copy between Smart Filter Effects, delete, display-only; Smart Object create/edit/replace/export/rasterize/Stack Mode surfaces; the Smart Filter stack relationship. The PDF's occasional "Masks panel (CS6)" wording is a CS5-era artifact (see Open questions).
- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` (same corpus) — the fetched text also established: Blur Gallery and Oil Paint as new CS6 smart-filter-capable filters, and the Smart Filter settings cannot differ across layer comps (`LAY-021`, `LAY-020`).

Cross-referenced (already sourced in the corpus):

- `02-ui-ux/panels/properties-panel.md` (`PAN-006`) — the contextual router and mask page.
- `05-layers/layer-masks.md` (`LAY-004`) — full layer-mask semantics.
- `05-layers/smart-objects.md` (`LAY-020`) and `05-layers/smart-filters.md` (`LAY-021`) — Smart Object/Filter semantics, filter-mask block, and UI note in the Layers panel.
- `08-selection/refine-edge.md` (`SEL-003`) and `08-selection/color-range.md` (`SEL-005`) — the Mask Edge / Color Range dialogs.

Consulted as search-result snippets only (not individually fetched; community/current-version):

- SearXNG query "Photoshop CS6 Masks panel Properties panel density feather invert refine mask" — corroborating snippets (Adobe help, PSforPhotographers) that the CS5 Masks panel was amalgamated into the CS6 Properties panel; used only as corroboration, with `LAY-004` as the sourced authority.

Not used in this pass:

- `helpx.adobe.com` mask/Smart Filter pages (HTTP 403 / current-version only).

## Open questions

- **"Masks panel (CS6)" wording.** The fetched CS6 Help text says "Masks panel (CS6)" in the Apply/Delete procedure (line 9531) while elsewhere stating the CS5/CS6 split. This is treated as a CS5-era typo; confirm CS6 has no standalone Masks panel. *Resolves with:* a CS6 UI capture.
- **Is there a Smart Object Properties page in CS6?** `PAN-006` flags the full `PropertiesTarget` set as inferred; the fetched CS6 Help does not document a Smart Object page. *Resolves with:* a CS6 capture of a selected Smart Object layer.
- **Where filter-mask controls live in CS6** — the Help describes them under "Masks panel", but CS6 retired it; likely the Properties panel's filter-mask mode. *Resolves with:* a CS6 UI capture.
- **Filter-mask Density/Feather defaults and ranges.** Not itemized. *Resolves with:* a CS6 capture; coordinate with `LAY-021`.
- **Vector-mask page contents** in the Properties panel vs. the Paths panel are only partially sourced. *Resolves with:* a CS6 capture; `LAY-005`.
- **Does CS6 show the mask/filter-mask/blend/opacity editor inline in the Layers panel** or only in the Properties panel? *Resolves with:* a CS6 capture; `PAN-001`/`LAY-021`.
- **Threshold for 32-bit masks** — CS6 enables Invert and Threshold, but the panel placement and range of Threshold are not detailed in the fetched text. *Resolves with:* a CS6 32-bit capture.
