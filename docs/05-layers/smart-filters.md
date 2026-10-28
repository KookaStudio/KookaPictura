# Smart Filters

- **Spec ID:** `LAY-021`
- **Status:** `Draft`
- **Parity tier:** `Core` (filter stack, filter mask, blending options, reorder/hide/delete, Clear Smart Filters)
- **New in CS6:** `Changed` — the Smart Filter mechanism is unchanged from CS5, but CS6 adds new smart-capable filters (**Blur Gallery**, **Oil Paint**). **Liquify** is documented as smart-compatible only after a later "Creative Cloud update", not in the shipped CS6 exclusions list (Extract, Liquify, Pattern Maker, Vanishing Point). See `## Open questions`.
- **Depends on:** `ARCH-008` document-model, `ARCH-006` gpu-rendering-pipeline, `ARCH-009` undo-history, `LAY-020` smart-objects, `LAY-022` layer-comps, `06-filters/filters-overview.md`, `06-filters/blur-gallery.md`, `06-filters/liquify.md`, `02-ui-ux/panels/layers-panel.md`

> All crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Behavior is taken from the fetched CS6 Help PDF unless
> marked *(inferred)*.

## CS6 behavior

Any filter applied to a Smart Object becomes a **Smart Filter**. Smart Filters appear in the Layers panel on a **Smart Filters** line **below** the Smart Object layer, because they are non-destructive: they can be adjusted, reordered, hidden, or removed at any time and the original object data is untouched. A triangle next to the Smart Filter icon collapses/expands the Smart Filter (and Layer Style) list; `Layers panel Options > Expand New Effects` controls the default.

**Which filters are smart-compatible (CS6):** Help says *any* Photoshop filter that has been enabled for Smart Filters, **except Extract, Liquify, Pattern Maker, and Vanishing Point**. In addition, the `Shadow/Highlight` and `Variations` **adjustments** can be applied as Smart Filters (`Image > Adjustments > Shadow/Highlight` or `Image > Adjustments > Variations`). Third-party filters that declare Smart Filter support also work. Filters applied through the **Filter Gallery** appear as a grouped entry named "Filter Gallery"; double-clicking an individual gallery entry edits it. The CS6 What's-New notes that the **Blur Gallery** effects and **Oil Paint** support Smart Objects and can be applied as Smart Filters.

**Pictura exclusions:** besides Liquify and Vanishing Point, Pictura refuses **Lens Blur**, **Displace**, and **Adaptive Wide Angle** on a Smart Object (greyed out in the Filter menu, "Rasterize the Smart Object to apply …" otherwise): each reads a session resource (a depth map, a displacement map, drawn constraints) that a stored Smart Filter cannot name yet (#1001). Image > Adjustments are greyed out on a Smart Object; Shadow/Highlight and Variations as Smart Filters are #996. Linked and legacy placed Smart Objects refuse filters.

**Applying:**
1. Do one of: select the Smart Object layer for a whole-layer effect; make a selection first to constrain the effect to that area; or, for a **regular layer**, choose `Filter > Convert For Smart Filters` and `OK` (this converts the layer to a Smart Object).
2. Choose a filter from the `Filter` menu (or `Shadow/Highlight` / `Variations`).
3. Set options and `OK`. The Smart Filter appears under the Smart Filters line. A **warning icon** next to a Smart Filter means the filter does not support the image's current color mode or bit depth.

**Editing a Smart Filter:**
- Edit settings: double-click the Smart Filter in the Layers panel, set options, `OK`. While editing, filters stacked above it are not previewed; Photoshop restores them after the edit.
- Edit blending options: double-click the **Edit Blending Options** icon (`Blend Mode` and `Opacity`, equivalent to the `Fade` command on a traditional layer).

**Hiding:** click the eye next to a single Smart Filter to hide/show it; click the eye next to the **Smart Filters** line to hide/show all filters on the object.

**Reorder / duplicate / delete:**
- Reorder: drag a Smart Filter up or down in the list. Photoshop applies Smart Filters **from the bottom up**.
- Duplicate: `Alt`/`Option`-drag a Smart Filter to another Smart Object or another position in the list; `Alt`/`Option`-drag the Smart Filters icon to duplicate all filters. Smart Filters **cannot be dragged onto regular layers**.
- Delete: drag an individual filter to the Delete icon; `Layer > Smart Filters > Clear Smart Filters` removes all filters from the object.

**Filter mask (one per object, shared by all its Smart Filters):**
- Applying a Smart Filter to a Smart Object creates an empty (white) mask thumbnail on the Smart Filters line; if a selection existed at apply time, that selection becomes the mask. With no mask the filters apply everywhere, as a white mask would; Pictura creates a mask only from a selection (the white mask without one is #1000), and a later apply with a selection keeps the existing mask *(inferred)*. The mask applies to **all** Smart Filters on the layer — individual filters cannot be masked.
- The mask behaves like a layer mask: stored as an alpha channel (visible in the Channels panel, loadable as a selection), painted black to hide, white to show, gray for partial; can be edited with tools and can receive image adjustments/filters.
- Masks panel controls: **Density** (opacity) and **Feathering**; `Invert`. `Mask Edge` is **not** available for filter masks.
- Display only the mask: `Alt`/`Option`-click the mask thumbnail.
- Move/copy the mask to another Smart Filter Effect: drag, or `Alt`/`Option`-drag to copy.
- Disable: `Shift`-click the thumbnail, the Masks panel Disable/Enable button, or `Layer > Smart Filter > Disable Filter Mask` (a red X marks it). Delete: Mask panel Delete button, drag to Delete, or `Layer > Smart Filters > Delete Filter Mask`.
- Add a mask after deletion: click the **Filter Mask** button in the Masks panel (empty mask), or with a selection right-click the Smart Filters line and choose `Add Filter Mask`.

**Merging / flattening:** Smart Filters exist only on a Smart Object. Rasterising the Smart Object (`Layer > Rasterize > Smart Object`) bakes the filter result and removes editability. Merging/flattening likewise loses the filter stack. Help does not provide a "merge smart filters" command that preserves them; the documented destructive routes are Rasterize and Clear Smart Filters.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Filter > <filter>` | Menu | — | Applies as a Smart Filter when the active layer is a Smart Object |
| `Filter > Convert For Smart Filters` | Menu | — | Converts a regular layer to a Smart Object so it can take Smart Filters |
| `Image > Adjustments > Shadow/Highlight` | Menu | — | Applicable as a Smart Filter |
| `Image > Adjustments > Variations` | Menu | — | Applicable as a Smart Filter |
| Layers panel — Smart Filters line | Panel row | — | Lists the filter stack below the Smart Object |
| Layers panel — expand triangle | Toggle | `Click` | Collapse/expand filters (and Layer Styles) |
| Layers panel — eye column | Toggle | `Click` | Hide/show one filter or all filters |
| Layers panel — warning icon | Indicator | — | Filter unsupported for the mode/depth |
| Layers panel — Edit Blending Options icon | Button | `Double-click` | Per-filter Blend Mode + Opacity |
| Layers panel — filter mask thumbnail | Thumbnail | `Alt`-click | Show mask only |
| Layers panel — drag | Gesture | `Drag` | Reorder; `Alt`-drag duplicates |
| `Layer > Smart Filters > Clear Smart Filters` | Menu | — | Deletes all filters on the object |
| `Layer > Smart Filters > Disable Filter Mask` | Menu | `Shift`-click | Toggles the filter mask |
| `Layer > Smart Filters > Delete Filter Mask` | Menu | — | Removes the filter mask |
| Masks panel — Filter Mask button | Panel | — | Adds a filter mask |
| Masks panel — Density / Feathering / Invert | Sliders/button | — | Filter-mask controls (`Mask Edge` absent) |
| Layers panel menu — `Options` | Dialog | — | `Expand New Effects` toggle |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Smart Filter Blend Mode | enum | Normal | 27 CS6 layer blend modes | Per filter; the "Edit Blending Options" icon |
| Smart Filter Opacity | int percent | 100 | 0–100 | Per filter; a Fade-style strength |
| Filter Mask presence | bool | created on apply | — | One per Smart Object, shared by all filters |
| Filter Mask initial state | mask | white (all shown) | — / selection | Selection at apply time becomes the mask |
| Filter Mask Density | int percent | 100 | 0–100 | Masks panel |
| Filter Mask Feather | double (px) | 0 | ≥ 0 | Masks panel; `Mask Edge` unavailable |
| Filter Mask Invert | bool | off | on / off | Masks panel |
| Filter Mask enabled | bool | on | on / off | Red X when disabled |
| Filter order | ordered list | apply order | drag | Applied bottom-up |
| Filter-specific params | filter-defined | filter default | filter-defined | Each filter exposes its own options dialog |

## Algorithms & pipeline

### Filter stack evaluation

*(inferred)* The Smart Filter stack is an ordered list. Rendering computes:

```text
result = render_smart_object()
for filter in stack_from_bottom_to_top:      # Photoshop applies bottom-up
    layer = filter.apply(result)             # filter's own algorithm
    result = blend(layer, result,
                   mode = filter.blend_mode,
                   opacity = filter.opacity)
result = result * filter_mask                # single mask, all filters
```

The per-filter blend/opacity step is the Fade behaviour: it interpolates between the filter output and its input using the mode and opacity. The **filter mask is applied after the whole stack**, not per filter — this is the observable reason the Help text says the mask "applies to all Smart Filters".

### Caching

Smart Filters are expensive because the source may be a multi-megapixel embedded document. *(inferred)* Cache the unfiltered source render keyed by `(source_revision, transform, doc_resolution, depth, mode)`; then cache each prefix of the stack so that editing/removing a filter at position *k* only recomputes *k*…*n*. This matches the Help note that editing a filter does not preview filters stacked above it (they are recomputed/restored after the edit).

### Color mode / depth gating

Each filter declares supported modes/depths. When the document mode or depth is unsupported, PS6 shows the warning icon and the filter is not applied. The compositor must carry a per-filter `supported(mode, depth) -> bool` and render the warning state without aborting the whole stack. *(inferred)*

### Plugin/serialization shape

Each Smart Filter instance carries: an identifier (filter name/plugin id), its parameter descriptor, a blend mode, an opacity byte, and a disabled flag. In PSD these live in the additional-layer-information blocks under the Smart Object: `FMsk` for the **Filter Mask** (CS3; color space + opacity) and `FXid`/`FEid` for **Filter Effects** (version 1/2/3, per-filter identifier/version/length/rectangle/depth/max-channels plus channel data with compression). *(sourced for the block shape; the mapping of each filter's parameters to its descriptor is `(inferred)`.)*

## Rust module mapping

- `pictura_filter::smart::SmartFilterStack` — `Vec<SmartFilterEntry>`; `render(source: &TileStore) -> TileStore`.
- `pictura_filter::smart::SmartFilterEntry` — `{ filter: Box<dyn Filter>, params: FilterParams, blend: BlendMode, opacity: u8, enabled: bool }`.
- `pictura_filter::registry` — maps filter id → implementation + `supported(mode, depth)`; shared with `06-filters`.
- `pictura_filter::smart::FilterMask` — single raster mask + density + feather + invert + enabled; reuses `pictura_core::mask`.
- `pictura_filter::smart::cache` — prefix cache keyed by source/transform revision.
- `pictura_core::smart` — owns the `SmartFilterStack` as part of the Smart Object (`LAY-020`).
- `pictura_io::psd::filters` — read/write `FMsk`, `FXid`/`FEid`; preserve unknown filter descriptors verbatim.

Crossing types: `NodeId(u64)`, `FilterId`, `FilterParams` (typed enum), `BlendMode`, `MaskRef`, and a rendered `TileStore` handle retained in the core.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `LayersModel` filter rows | `QAbstractItemModel` | Emits Smart Filters as child rows (icon, name, warning, eye, blend icon, mask thumbnail) |
| `SmartFilterDelegate` | `QStyledItemDelegate` | Paints the filter icon, warning badge, eye column, blend-options icon, and the single shared filter mask |
| `FilterBlendingOptionsDialog` | `QDialog` | Blend Mode combo + Opacity spin for one filter |
| `MasksPanel` | `QWidget` | Filter Mask button, Density/Feathering sliders, Invert, Disable/Enable, Delete; disables `Mask Edge` for filter masks |
| `FilterOptionsHost` | `QDialog`/`QStackedWidget` | Hosts each filter's parameter UI for edit; suppresses preview of higher filters while editing |
| `FilterMenuBuilder` | helper | Greys out filters whose `supported(mode, depth)` is false on the active layer |

Widgets over QML for the panel rows and dialogs per `ARCH-003`; the warning/eye columns are model roles consumed by the shared Layers delegate.

## Data-model impact

- Extend `NodeKind::SmartObject` with a `SmartFilterStack`; each entry references a filter implementation plus a serialized parameter descriptor.
- The Smart Object carries **at most one filter mask** (alpha-channel-backed) with density/feather/invert/enabled state.
- Undo: adding/removing/reordering/editing a filter, editing the mask, and changing blend/opacity are discrete commands. Undo records are structural (stack order + per-filter params), not pixel backups, because rendering is derived.
- Serialization: `FMsk` (filter mask) and `FXid`/`FEid` (filter effects) additional-layer blocks; unknown filter parameter descriptors and unknown effect records must be preserved byte-for-byte for lossless round-trip (`ARCH-008`, `ARCH-011`).
- **Layer comps:** Smart Filter settings are **not** comp-varied — once applied, a filter appears in all comps (`LAY-022`). The filter mask blends into the layer's stored mask/alpha state.
- Rasterising (`LAY-020`) collapses the stack into pixels and drops the `FMsk`/`FXid` data for that layer.

## Edge cases

- **Unsupported filter for mode/depth** — show the warning icon; do not apply the filter, but keep it in the stack (so switching mode/depth can re-enable it). *(inferred from the Help warning-icon note.)*
- **Extract / Pattern Maker / Vanishing Point** — never smart-compatible in the CS6 list; and the optional plug-ins may be absent.
- **Liquify** — documented as smart-compatible only in a later CC update; in the shipped CS6 list it is excluded. Must be decided per Open questions.
- **Filter Gallery group** — appears as one "Filter Gallery" entry; individual gallery entries are editable by double-click.
- **Regular layer** — `Filter` menu items do not create Smart Filters until `Filter > Convert For Smart Filters` is used; Smart Filters cannot be dragged onto regular layers.
- **Multiple objects** — a filter can be dragged between two Smart Object layers only; it cannot be copied to a regular layer.
- **Filter mask on a not-yet-masked object** — applying creates a mask automatically; deleting the mask is reversible via the Masks panel.
- **Editing a mid-stack filter** — filters above are not previewed during the edit; the UI must not show a stale composite as final.
- **8/16/32-bit** — filters that only support 8-bit (many legacy/artistic filters) show the warning at 16/32-bit; 32-bit float results must not be clamped.
- **CMYK/Lab** — many filters support only RGB; warning-icon behaviour applies.
- **GPU unavailable** — filter preview falls back to CPU; ensure the cache key includes the backend.
- **Smart Object transform** — filter effects are suppressed during the transform and re-applied after (`LAY-020`).
- **PSB / huge documents** — stack render must be tiled/streamed; a full-resolution filtered copy per filter prefix can blow the memory budget.
- **Undo/redo** — derived results are recomputed, so undo must restore the stack/params/mask state and invalidate caches.
- **Third-party filters** — a filter may declare Smart Filter support without being serializable into `FXid`; must degrade gracefully on save.

## Parity acceptance criteria

- Given a Smart Object and applying any supported filter, a Smart Filters line appears below the layer with an edit-blending-options icon and a white filter-mask thumbnail.
- Given the stack `[A, B]` (A applied first, B second) rendered bottom-up, the result equals applying A then B; dragging B below A changes the result to B then A.
- Given a Smart Filter and changing its Blend Mode/Opacity to `50% Normal`, the result equals a 50/50 fade between the filter output and its input within tolerance.
- Given a selection made before applying a Smart Filter, the automatically created filter mask matches the selection.
- Given a filter mask painted black in a region, the filter has no effect there; painting white restores it; gray gives partial effect.
- Given the filter mask Density/Feather controls, the mask opacity/edge softness change accordingly, and `Mask Edge` is disabled/absent.
- Given `Shift`-click on the filter-mask thumbnail, a red X appears and the Smart Filter applies without masking; `Shift`-click again re-enables it.
- Given `Layer > Smart Filters > Clear Smart Filters`, the stack is emptied and the source render is unchanged.
- Given a CMYK or 16-bit/32-bit document and a filter that does not support that mode/depth, the warning icon appears and the filter is not applied; switching back re-enables it.
- Given a Smart Object with Smart Filters, `Layer > Rasterize > Smart Object` bakes the rendered result and removes the Smart Filters line.
- Given a CS6 PSD with `FXid`/`FEid` and `FMsk` blocks, opening and re-saving preserves the filter descriptors and mask byte-for-byte (or re-emits an equivalent stack).
- Given a layer comp change, the Smart Filter stack does not change (`LAY-022`).

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — the CS6 Help corpus. Established: "any filter applied to a Smart Object is a Smart Filter"; the excluded filters **Extract, Liquify, Pattern Maker, Vanishing Point**; Shadow/Highlight and Variations as smart-capable adjustments; Filter Gallery grouping; `Filter > Convert For Smart Filters`; the warning-icon meaning (unsupported mode/depth); editing settings and blending options; hide one/all; reorder (applied **bottom-up**), duplicate, delete, `Clear Smart Filters`; the single filter-mask model and its paint/density/feather/invert/disable/delete/add operations; `Mask Edge` unavailable for filter masks; masks stored as alpha channels; the CS6 What's-New notes for Blur Gallery and Oil Paint Smart Filters and the "Creative Cloud update" Liquify note; the Layer Comps note that Smart Filter settings cannot vary across comps.
- `https://web.archive.org/web/2023id_/https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/` — Adobe PSD/PSB File Formats Specification. Established: `FMsk` Filter Mask (Photoshop CS3; 10-byte color space + 2-byte opacity); `FXid`/`FEid` Filter Effects (version 1/2/3; per-effect identifier, version, length, rectangle, depth, max channels, channel data and compression); PSB 8-byte key list including `FMsk`/`FEid`/`FXid`.
- `https://bjango.com/articles/photoshopcc2014smartobjects/` — established the CC 2014 layer-comp/innovation context; used only to bound which Smart Filter behaviours are post-CS6.

Not used in this pass:

- `helpx.adobe.com` (HTTP 403) — modern Help pages were inaccessible; the archived CS6 Help PDF was used instead.

## Open questions

- **Liquify timing.** Is Liquify smart-compatible in shipped CS6.0, or only in the later CC update the Help text mentions? This directly determines the exclusion list. Resolve with a CS6 build test or a CS6.0 Help snapshot.
- **Full smart-compatible filter catalogue.** Help's "any filter … except …" wording does not enumerate the set. Some legacy filters may silently not support Smart Filters. Resolve by applying every CS6 filter to a Smart Object and recording which show the warning.
- **`FXid`/`FEid` parameter mapping.** The block shape is sourced, but the per-filter parameter descriptor keys are not. Resolve by parsing CS6-authored PSDs or `psd-tools`/`PhotoshopAPI` effect structures.
- **Filter-mask density/feather serialization.** Whether CS6 writes filter-mask density/feather in the mask record or a separate block is unconfirmed. Resolve with reference PSDs.
- **Filter Gallery group ordering.** Help says double-click a gallery entry to reorder "any gallery filters"; whether the group is one stack entry or expands into sub-entries in PSD is unconfirmed.
- **Blend-formula parity.** Per-filter fade/blend rounding is closed; define numeric tolerance against the PDF blend formulas and a CS6 reference render (see `ARCH-008` blending note).
- **Cache/invalidation policy.** Whether PS6 caches filter prefixes or re-renders the whole stack on each edit affects interactive performance parity. Resolve with a timing study.
