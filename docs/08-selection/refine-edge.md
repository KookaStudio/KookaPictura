# Refine Edge

- **Spec ID:** `SEL-003`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `No` — the Refine Edge dialog with **Smart Radius**, the refinement brushes, the global sliders, and **Decontaminate Colors** shipped in CS5; the CS6 Help documents it as carried forward. CS6's documented *selection* novelties are Color Range Skin Tones / Detect Faces (`SEL-005`). No fetched CS6 source states a behavioral expansion of Refine Edge in CS6 — see Open questions.
- **Depends on:** `SEL-001` selection-model, `SEL-002` selection-tools-overview, `SEL-005` color-range, `TOOL-002`/`TOOL-003`/`TOOL-004`, `TOOL-044` quick-mask-tool, `05-layers/layer-masks.md` (`LAY-004`), `06-filters/blur-filters.md` (feather-like blur), `01-architecture/document-model.md` (`ARCH-008`), `01-architecture/undo-history.md` (`ARCH-009`), `01-architecture/gpu-rendering-pipeline.md` (`ARCH-006`), `02-ui-ux/panels/properties-panel.md`.

> All module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Behavior is taken from the fetched CS6 Help corpus;
> the view-mode enumeration, their letter shortcuts, and the Output To list are
> community-reported and marked as such.

## CS6 behavior

Refine Edge improves the quality of selection edges so objects can be extracted more easily. CS6 invokes it in two contexts:

1. **From a pixel selection:** create a selection with any selection tool, then click **Refine Edge** in the options bar or choose `Select > Refine Edge`. The result is a refined selection or a mask.
2. **From a layer mask:** Refine Edge can refine a layer mask via the mask section of the Properties panel (CS6 moved mask editing there from the CS5 Masks panel, `LAY-004`).

The dialog's documented controls:

- **View Mode** — a pop-up that changes how the selection is displayed. The CS6 Help names **Show Original** (shows the original selection for comparison) and **Show Radius** (shows the selection border where edge refinement occurs) and says to hover over each mode for its tooltip. The concrete mode list is not enumerated in the fetched text; the CS6-era/community list is **Marching Ants** (`M`), **Overlay** (`V`), **On Black** (`A`), **On White** (`T`), **Black & White** (`K`), **On Layers** (`Y`), and **Reveal Layer** (`R`). (Note: "Mask" as a view mode belongs to the later CC **Select and Mask** workspace, not CS6 Refine Edge.)
- **Refine Radius** and **Erase Refinements** tools — adjust precisely the border area in which edge refinement occurs. `Shift+E` toggles between them; bracket keys change brush size. Brushing over soft areas such as hair or fur adds fine details to the selection.
- **Smart Radius** — automatically adjusts the radius for hard and soft edges found in the border region. Deselect it when the border is uniformly hard- or soft-edged, or when you want to control the Radius setting and refinement brushes directly.
- **Radius** — the width of the border band in which edge refinement occurs; use a small radius for sharp edges and a large one for softer edges.
- **Smooth** — reduces irregular areas (hills and valleys) in the selection border to create a smoother outline.
- **Feather** — blurs the transition between the selection and its surrounding pixels.
- **Contrast** — increasing it makes soft-edged transitions along the selection border more abrupt. Usually Smart Radius and the refinement tools give better results.
- **Shift Edge** — moves soft-edged borders inward with negative values and outward with positive ones; shifting inward helps remove unwanted background colors from selection edges.
- **Decontaminate Colors** — replaces color fringes with the color of nearby fully selected pixels; the strength of replacement is proportional to edge softness. Because the option changes pixel color, CS6 requires output to a new layer or document and keeps the original layer so you can revert to it. A **Reveal Layer** view mode shows the result.
- **Amount** — sets the level of decontamination and fringe replacement (appears with Decontaminate Colors).
- **Output To** — determines whether the refined selection becomes a selection or mask on the current layer, or is written to a new layer or document. The CS6-era/community list is **Selection**, **Layer Mask**, **New Layer**, **New Layer with Layer Mask**, **New Document**, and **New Document with Layer Mask**.

Keyboard behavior documented in the CS6 key tables: `Ctrl+Alt+R` / `Cmd+Opt+R` opens the dialog; `F` cycles view modes forward, `Shift+F` backward; `X` toggles original image vs selection preview; `P` toggles original selection vs refined version; `J` toggles the radius preview; `Shift+E` toggles the Refine Radius / Erase Refinements tools.

Refine Edge is the CS6-recommended replacement for the old Extract plug-in: it gives better results and nondestructive processing, because unlike Extract — which permanently erases pixel data — Refine Edge creates selection masks that can be readjusted and fine-tuned later.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Options bar (selection tools) | Button | `Ctrl+Alt+R` / `Cmd+Opt+R` | Refine Edge |
| `Select > Refine Edge…` | Menu | `Ctrl+Alt+R` / `Cmd+Opt+R` | Same dialog |
| Properties panel (mask) | Button | — | Refine Edge / Mask Edge on a layer mask (`LAY-004`) |
| Dialog | Combo | `F` / `Shift+F` | View Mode pop-up |
| Dialog | Checkbox/toggle | `X` | Toggle original image / selection preview |
| Dialog | Checkbox/toggle | `P` | Toggle original / refined selection |
| Dialog | Checkbox/toggle | `J` | Toggle radius preview |
| Dialog | Toggle | `Shift+E` | Switch Refine Radius ↔ Erase Refinements |
| Dialog | Brush | `[` / `]` | Refinement-brush size |
| Dialog | Slider/check | — | Smart Radius |
| Dialog | Slider/number | — | Radius |
| Dialog | Slider/number | — | Smooth |
| Dialog | Slider/number | — | Feather |
| Dialog | Slider/number | — | Contrast |
| Dialog | Slider/number | — | Shift Edge |
| Dialog | Checkbox + slider | — | Decontaminate Colors (+ Amount) |
| Dialog | Combo | — | Output To |
| Canvas | Brush overlay | — | Refinement brush painting over edge region |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| View Mode | enum | Marching Ants *(community)* | Marching Ants / Overlay / On Black / On White / Black & White / On Layers / Reveal Layer | CS6 Help does not enumerate the list |
| Show Original | toggle | off | on / off | Compare against original selection (`P`) |
| Show Radius | toggle | off | on / off | Radius preview (`J`) |
| Smart Radius | bool | off | on / off | Auto radius for mixed hard/soft edges |
| Radius | int px | 0 | CS6 max unverified (community up to ~1000) | Edge-detection band |
| Smooth | int | 0 | 0–100 *(community)* | Hills/valleys reduction |
| Feather | int px | 0 | 0..dialog max *(community)* | Transition blur |
| Contrast | percent | 0 | 0–100% *(community)* | Sharpen soft transitions |
| Shift Edge | percent | 0 | −100..+100% *(community)* | Inward (negative) / outward (positive) |
| Decontaminate Colors | bool | off | on / off | Changes pixel color; forces new layer/doc output |
| Amount | int percent | 50 *(community)* | 0–100% | Decontamination strength |
| Output To | enum | Selection *(community)* | Selection / Layer Mask / New Layer / New Layer with Layer Mask / New Document / New Document with Layer Mask | Decontaminate constrains this list |
| Refinement brush size | int px | tool default | bracket-stepped | Refine Radius / Erase Refinements |

Ranges for Radius/Smooth/Feather/Contrast/Shift Edge/Amount are not specified in the fetched CS6 Help text and are marked community-reported; the CS6 Help only describes their direction and meaning.

## Algorithms & pipeline

Refine Edge is largely closed. Only the observable behavior is documented; the following is an implementation-agnostic model, marked inferred.

### Edge detection and radius

For each pixel in the initial selection's border band:

```text
for each boundary pixel p:
    if SmartRadius:
        r(p) = adapt(band width)     # narrower where edge is hard, wider where soft
    else:
        r(p) = Radius
    band(p) = disk(p, r(p))
```

The **Radius** defines a search band around the current boundary. Within the band the algorithm re-estimates alpha coverage from the underlying image; outside the band the mask is unchanged. The refinement brushes override the band locally: **Refine Radius** extends refinement into soft regions (hair/fur), **Erase Refinements** removes previously refined detail.

### Global refinements (applied after edge estimation)

- **Smooth** — a majority/regularization pass on the coverage map (the `Select > Modify > Smooth` majority filter is the documented analog; `SEL-002`).
- **Feather** — a blur of coverage (`SEL-001`).
- **Contrast** — steepens the coverage ramp, pushing partial values toward 0/255.
- **Shift Edge** — moves the 50% contour inward (negative) or outward (positive), typically by re-thresholding the shifted coverage profile.

```text
smooth(mask, amount); feather(mask, r); contrast(mask, c); shift_edge(mask, s):
    mask = blend toward threshold(mask, 128) by c
    mask = shift the coverage profile along the distance field by s
```

The exact Adobe orderings and kernels are closed — **behavioral parity only, algorithm TBD**.

### Color decontamination

Documented behavior: color fringes are replaced with the color of nearby fully selected pixels, with replacement strength proportional to edge softness. A standard model is **alpha matting foreground estimation**: recover `F` from `I = αF + (1−α)B` using nearby fully-opaque foreground colors as `F`, then replace edge pixel color with `F` weighted by `Amount` and `(1−α)`. This is the conventional approach; Adobe's exact estimator and the `Amount` mapping are unspecified — **behavioral parity only**.

Because decontamination writes color, CS6 forbids in-place output and requires a new layer/document.

### Output

`Output To` maps the refined result:

- **Selection** — replace the active selection with the refined mask.
- **Layer Mask** — write the refined mask into the active layer's mask (Properties-panel invocation always targets the mask).
- **New Layer** / **New Layer with Layer Mask** — create a layer holding the selected pixels (and optionally the mask).
- **New Document** / **New Document with Layer Mask** — same, into a new document.

## Rust module mapping

- `pictura_selection::refine::RefineEdgeSettings` — `{ view_mode, show_original, show_radius, smart_radius, radius, smooth, feather, contrast, shift_edge, decontaminate: bool, amount, output_to }`.
- `pictura_selection::refine::refine(mask: &Mask, image: &Image, settings) -> Mask` — edge estimation + global refinements; operates on tiles with a boundary halo.
- `pictura_selection::refine::edge_band` — distance transform + disk band extraction; `smart_radius` adaptation.
- `pictura_selection::refine::decontaminate(image: &Image, alpha: &Mask, amount) -> Image` — foreground estimation; returns a new pixel buffer (never in place).
- `pictura_selection::refine::refine_brush` — interactive Refine Radius / Erase Refinements state; feeds a local band override.
- `pictura_selection::output::apply_output(mask, output_to, doc) -> SelectionOutput` — dispatch to selection / layer mask / new layer / new document.
- `pictura_selection::contour` — reused for the ants view mode (`SEL-001`).

Boundary types: `Mask` tiles, `RefineEdgeSettings`, `OutputTarget`, `ViewMode`, and overlay/paint strokes. Decontamination crosses as a copy of the affected layer region; the GUI never mutates pixel data.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `RefineEdgeDialog` | `QDialog` | The whole dialog; modal, remembers last-used values within the session |
| `RefineEdgePreview` | `QGraphicsView` / custom widget | Renders the chosen view mode (composite/alpha/overlay) |
| `ViewModeCombo` | `QComboBox` | View-mode list; tooltips per mode |
| `RefineBrushTool` | `QToolButton` + state | Refine Radius / Erase Refinements, `Shift+E` toggle, bracket sizing |
| `SmartRadiusCheck` | `QCheckBox` | Enables/disables Radius spin and changes brush behavior |
| `GlobalRefineSliders` | `QSlider` + `QSpinBox`/`QDoubleSpinBox` | Smooth / Feather / Contrast / Shift Edge |
| `DecontaminateGroup` | `QCheckBox` + slider | Colors + Amount; disables in-place output targets |
| `OutputToCombo` | `QComboBox` | Output target list, filtered by Decontaminate |
| `RefineEdgeKeyFilter` | `QShortcut`/event filter | `F`, `Shift+F`, `X`, `P`, `J`, `Shift+E`, `[`, `]` |

Widgets over QML: the dialog is dense, slider-heavy, keyboard-driven, and shared with the layer-mask Properties panel; a `QDialog` keeps focus/shortcut handling and live preview integration predictable. The preview is a single GPU-backed canvas widget (`ARCH-006`) fed by core-computed tiles.

## Data-model impact

- Invoked on a **selection**: produces a replacement selection (undo: `SelectionChange`, `SEL-001`) or writes a **layer mask** / creates a layer/document.
- Invoked on a **layer mask**: edits the mask's raster (`LAY-004`); one undo state per OK.
- **Decontaminate Colors** writes pixel color, so it always creates data; output to a new layer/document is mandatory. Undo captures the new layer(s) per `ARCH-009`.
- A **New Layer with Layer Mask** output writes a mask record in the PSD (`ARCH-008`/`ARCH-011`): density 100, feather 0 by default, user mask channel id; the refined alpha is the mask image.
- Refine Edge settings are **session/preference state**, not document state — but if CS6 offers a persistent "remember settings" it would live in preferences (`SEL-003` Open questions; the later CC Select and Mask has an explicit "Remember Settings" checkbox).
- The refinement brush strokes are interaction state; only the final mask/color result is committed.

## Edge cases

- **Empty selection:** Refine Edge has nothing to refine; the button should be disabled or a no-op, not open an error.
- **Hard-edged selection:** Smart Radius off with a small Radius is the documented choice; refinement should not invent soft detail absent in the pixels.
- **Feather applied before refine:** the initial band may already be soft; refinement re-estimates from the image, not from the feathered mask, per the documented goal of improving selection-edge quality.
- **Decontaminate + Selection/Layer-Mask output:** must be disallowed (CS6 requires a new layer/document); disable those Output To choices when Decontaminate Colors is on.
- **Decontaminate on a flat background:** foreground estimation has no useful nearby fully-selected color; fall back to leaving the fringe or a neutral color, and never crash.
- **8-bit vs 16/32-bit:** edge estimation and decontamination should run at the document's native depth; coverage stays 0..1. Official CS6 support at 32 bpc is unstated.
- **CMYK/Lab:** decontamination operates per channel in the working space; no mode conversion at refine time.
- **Layer mask target:** in CS6, the mask dialog lives in the Properties panel; ensure `Output To` defaults to Layer Mask and cannot create a stray selection instead.
- **Huge/PSB:** the border band is locally bounded by Radius, but the distance transform and feather can touch the whole canvas; process in tiles with a halo and snapshot only dirty tiles for undo.
- **Live preview cost:** the dialog previews on every slider move; use a downsampled/proxy preview during drag and full-res on commit (`ARCH-006`, `01-architecture/performance-targets.md`).
- **GPU unavailable:** preview uses the CPU compositor; refined mask math is CPU either way.
- **Cancel:** must restore the pre-dialog selection/mask and discard decontaminated pixel buffers without touching history.

## Parity acceptance criteria

- Given a soft (hair) selection on a contrasting background, enabling Smart Radius and brushing the hair with Refine Radius recovers strands that the initial selection missed, and the refined 50% contour follows the strands more closely than the input.
- Given Smart Radius off and Radius = 0, the dialog output equals the input selection exactly.
- Given a larger Radius, more pixels near the boundary can change coverage; no pixel outside `Radius` of the original boundary changes.
- Given `Shift Edge` negative, the 50% contour moves inward; positive moves it outward, monotonically with the value.
- Given `Contrast` increased, the coverage histogram becomes more bimodal (fewer intermediate values).
- Given `Smooth` increased, isolated specks/holes in the boundary are reduced.
- Given `Feather` increased, the ants boundary stays at 50% while the transition widens (`SEL-001`).
- Given Decontaminate Colors on, the Output To choices Selection and Layer Mask are unavailable.
- Given Decontaminate Colors with a new-layer output, edge pixels change color toward nearby fully-selected pixels and the original layer is unchanged.
- Given `F` / `Shift+F`, the view mode cycles forward/backward through the full list and wraps.
- Given `X`, `P`, and `J`, the preview toggles original/selection, original/refined, and radius on/off respectively.
- Given `Shift+E`, the active refinement brush switches; bracket keys change its size.
- Given Cancel, the document (selection, mask, and pixels) is byte-identical to the pre-dialog state.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — primary CS6 Help corpus. Established: Refine Edge invocation from the options bar and `Select > Refine Edge`; the description of View Mode with **Show Original** and **Show Radius**; the Refine Radius / Erase Refinements tools and `Shift+E` / bracket sizing; Smart Radius; Radius; Smooth; Feather; Contrast; Shift Edge; Decontaminate Colors and the rule that it requires output to a new layer or document, plus **Amount**; Output To (selection or mask on the current layer versus a new layer or document); the Refine Edge key table (`Ctrl+Alt+R`, `F`/`Shift+F`, `X`, `P`, `J`, `Shift+E`); Refine Edge as the replacement for Extract; refining a layer mask from the mask section of the CS6 Properties panel.
- `https://shootdotedit.com/blogs/news/refine-edge-in-photoshop` — community guide. Established (community-reported, not CS6 Help): the full view-mode list with letter shortcuts — Marching Ants (M), Overlay (V), On Black (A), On White (T), Black and White (K), On Layers (Y), Reveal Layer (R) — and the full Output To list — Selection, Layer Mask, New Layer, New Layer With Mask, New Document, New Document With Layer Mask.
- `https://search.brave.com/search?q=...` / SearXNG result snippets for "Refine Edge CS6 view modes" — corroborated the view-mode names and shortcuts via multiple CS5/CS6-era tutorials (lifewire, photoshopessentials "Selecting Hair with Refine Edge", a 2012 tutorial listing "Reveal Layer … shortcut is R"). These were consulted as snippets, not all fetched.

Not used in this pass:

- `https://tricky-photoshop.com/refine_edge_tool` — HTTP 403 (the 2012-10-23 article that appeared to list the modes and shortcuts).
- `helpx.adobe.com` Refine Edge pages — HTTP 403.
- `html.duckduckgo.com` / `search.brave.com` result HTML — JS/anti-bot shells.

## Open questions

- **CS6 novelty.** No fetched CS6 source states that Refine Edge was expanded in CS6 relative to CS5; the Smart-Radius + Decontaminate dialog is documented as a CS5 feature and the CS6 Help is silent on changes. Resolve with a CS5-vs-CS6 dialog/behavior comparison (or the CS6 "What's New" list, which does not mention Refine Edge).
- **View-mode enumeration and shortcuts in CS6.** The CS6 Help does not list the modes; the community list includes Marching Ants / Overlay / On Black / On White / Black & White / On Layers / Reveal Layer. Whether CS6 has exactly these seven and which letters are bound is unverified. Resolve with a CS6 UI capture.
- **"Remember Settings".** The task framing lists a remember-settings control; the explicit "Remember Settings" checkbox belongs to the later CC **Select and Mask** workspace, and the CS6 Help does not mention one. Resolve with a CS6 dialog capture; if absent, drop it from the CS6 parity target.
- **Slider ranges.** Radius/Smooth/Feather/Contrast/Shift Edge/Amount numeric ranges are not in the CS6 text. Resolve from a CS6 UI capture.
- **Edge-estimation and decontamination algorithms.** Closed. Resolve only to the extent pixel-comparison parity requires; otherwise keep behavioral.
- **Output To defaults and constraints.** Whether Output To defaults to Selection and exactly which options Decontaminate Colors disables is community-reported. Resolve with a CS6 capture.
- **32-bpc behavior.** Whether the dialog and decontamination work at 32 bpc is unstated. Resolve with a CS6 test.
