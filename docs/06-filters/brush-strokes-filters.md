# Brush Stroke Filters

- **Spec ID:** `FILT-081`
- **Status:** `Draft`
- **Parity tier:** `Core` — the Brush Stroke submenu and the Filter Gallery are in CS6 Standard and Extended.
- **New in CS6:** `No` — the 8 Brush Stroke filters are unchanged from CS5.
- **Depends on:** `FILT-001` filters-overview, `LAY-021` smart-filters, `LAY-020` smart-objects, `LAY-010` blend-modes, `IMG-005` bit-depth-and-conversion, `ARCH-006` gpu-rendering-pipeline, `ARCH-009` undo-history, `ARCH-008` document-model, `ARCH-003` performance-targets.

> All crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Adobe's kernels are closed: **behavioral parity only,
> algorithm TBD**. Control names are taken from the CS6 Help PDF and Adobe Help;
> ranges marked *(sourced)* are from the fetched references in `## Sources`;
> unmarked defaults/ranges are *(inferred)* and must be confirmed against a CS6
> build.

## CS6 behavior

**Brush Stroke filters.** As with the Artistic filters, the Brush Stroke filters produce a painterly or fine-arts look through varied brush and ink stroke effects; some add grain, paint, noise, edge detail, or texture. All of them can be applied through the Filter Gallery.

The family has **8** filters. All are **8-bit only** and run in the Filter Gallery. Two of them (`Spatter`, `Sumi-e`) are called out by the Help as memory-intensive, with the advice to adjust settings to speed up filters such as Spatter and Sprayed Strokes.

| Filter | CS6 Help behavior (sourced) |
|---|---|
| **Accented Edges** | Accentuates image edges. High **Edge Brightness** makes accents resemble white chalk; low makes them resemble black ink. |
| **Angled Strokes** | Repaints using diagonal strokes; lighter and darker areas are painted with strokes going in opposite directions. |
| **Crosshatch** | Preserves detail while adding texture and roughening colored-area edges with simulated pencil hatching. **Strength** (1–3) is the number of hatching passes. |
| **Dark Strokes** | Paints dark areas with short, tight, dark strokes and light areas with long, white strokes. |
| **Ink Outlines** | Redraws the image with fine narrow lines over the original details, in pen-and-ink style. |
| **Spatter** | Replicates a spatter airbrush; increasing the options simplifies the overall effect. Memory-intensive. |
| **Sprayed Strokes** | Repaints using the image's dominant colors with angled, sprayed strokes. Memory-intensive. |
| **Sumi-e** | Paints in Japanese style, as if with a fully saturated brush on rice paper; soft blurred edges with rich inky blacks. |

**Filter Gallery mechanics (shared).** `Filter > Filter Gallery` presents one dialog with a preview, category thumbnails, the selected effect's options, and an applied-effect list. Effects are **cumulative and applied in list order**, can be **reordered by dragging**, **hidden with the eye icon**, and **deleted**. The gallery is **8-bit-per-channel only**. Applied to a Smart Object, the whole stack becomes one grouped "Filter Gallery" Smart Filter entry (`LAY-021`). Shared keys, Fade, and the pipeline are in `FILT-001`.

**Fade note (sourced):** the Help's `Edit > Fade` description states that Fade also modifies the effects of the Liquify command and the Brush Strokes filters.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Filter > Brush Stroke > <filter>` | menu submenu | — | Same filters also inside the Filter Gallery |
| `Filter > Filter Gallery > Brush Stroke` | dialog category | — | Cumulative stack; canonical CS6 route |
| Filter Gallery — filter thumbnail | click | `Alt`-click applies on top *(PDF)* | Adds the effect |
| Filter Gallery — options pane | controls | — | Per-filter sliders/combo |
| Filter Gallery — applied list | drag / eye / delete | — | Reorder, hide, delete |
| Filter Gallery — Cancel button | button | `Ctrl`→Default, `Alt`→Reset *(PDF)* | Button changes label |
| `Edit > Fade` | dialog | — *(shortcut unverified)* | Opacity + mode; Help explicitly notes it modifies Brush Strokes effects |
| Layers panel — Smart Filters line | panel | — | Stack appears as one "Filter Gallery" entry |

## Parameters & ranges

All Brush Stroke filters run in **8-bit** only. Control names marked *(CS6 PDF)* are sourced from the primary Help text; ranges marked *(sourced)* come from the fetched references (the CS6 PDF states **Crosshatch Strength 1–3**; AliveColors corroborates Accented Edges and Dark Strokes).

### Accented Edges
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Edge Width | slider int | 2 *(inferred)* | 1–14 *(sourced)* | Thickness of the accent lines |
| Edge Brightness | slider int | 25 *(inferred)* | 0–50 *(sourced)* | Low = black ink; high = white chalk |
| Smoothness | slider int | 5 *(inferred)* | 1–15 *(sourced)* | Reduces detail / smooths uneven edges |

### Angled Strokes
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Direction Balance | slider int | 50 *(inferred)* | 0–100 *(inferred)* | Ratio of the two diagonal directions |
| Stroke Length | slider int | 15 *(inferred)* | 3–50 *(inferred)* | |
| Sharpness | slider int | 3 *(inferred)* | 0–10 *(inferred)* | |

### Crosshatch
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Stroke Length | slider int | 9 *(inferred)* | 3–50 *(inferred)* | |
| Sharpness | slider int | 6 *(inferred)* | 0–20 *(inferred)* | |
| Strength | slider int | 1 *(inferred)* | 1–3 *(sourced)* | Number of hatching passes |

### Dark Strokes
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Balance | slider int | 5 *(inferred)* | 0–10 *(sourced)* | More dark strokes as the value rises |
| Black Intensity | slider int | 6 *(inferred)* | 0–10 *(sourced)* | Darkening of dark areas |
| White Intensity | slider int | 5 *(inferred)* | 0–10 *(sourced)* | Lightening of light areas |

### Ink Outlines
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Stroke Length | slider int | 10 *(inferred)* | 1–50 *(inferred)* | |
| Dark Intensity | slider int | 25 *(inferred)* | 0–50 *(inferred)* | |
| Light Intensity | slider int | 25 *(inferred)* | 0–50 *(inferred)* | |

### Spatter
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Spray Radius | slider int | 10 *(inferred)* | 0–25 *(inferred)* | Scatter range |
| Smoothness | slider int | 5 *(inferred)* | 1–15 *(inferred)* | Merges individual spots |

### Sprayed Strokes
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Stroke Length | slider int | 12 *(inferred)* | 0–20 *(inferred)* | |
| Spray Radius | slider int | 7 *(inferred)* | 0–25 *(inferred)* | |
| Stroke Direction | enum | Right Diagonal *(inferred)* | Right Diagonal / Horizontal / Left Diagonal / Vertical *(inferred)* | |

### Sumi-e
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Stroke Width | slider int | 8 *(inferred)* | 3–15 *(inferred)* | |
| Stroke Pressure | slider int | 5 *(inferred)* | 0–15 *(inferred)* | |
| Contrast | slider int | 20 *(inferred)* | 0–40 *(inferred)* | |

`(inferred)` defaults and ranges come from the CS6 dialog and community documentation, not the fetched CS6 PDF; read them back from a CS6 build (see `## Open questions`).

## Algorithms & pipeline

All Brush Stroke kernels are closed; **behavioral parity only, algorithm TBD**. Shared application pipeline in `FILT-001`.

- **Gallery stack.** Each filter is an independent stage in `pictura-filters::gallery`, evaluated in list order on the 8-bit image.
- **Stroke-and-edge family (behavioral model).** Accented Edges, Angled Strokes, Crosshatch, Dark Strokes, Ink Outlines, Spatter, Sprayed Strokes, and Sumi-e are variations on **edge/orientation detection + directional stroke rendering + tonal gating**:
  - an edge/gradient stage supplies stroke direction and strength;
  - a stroke rasterizer stamps directional marks whose length/width come from the controls;
  - a tonal gate decides whether a pixel receives dark (shadow) or light (highlight) strokes (explicit in Dark Strokes, Angled Strokes, Sumi-e);
  - Spatter/Sprayed Strokes add a **scatter/jitter** term (airbrush-like), which is why they are memory-intensive and why increasing the options "simplifies" the result.
- **Crosshatch** applies a 1–3 pass hatching overlay on top of a detail-preserving base, matching the "number of hatching passes" description.
- **Randomness/determinism.** Spatter and Sprayed Strokes scatter probabilistically; use a **seeded RNG** stored in the history/effect record so redo and PSD round-trips are reproducible.
- **Memory.** Spatter and Sprayed Strokes are named in the Help's performance list; the implementation must be tile-local with a bounded apron and must stream rather than allocate per-stroke large buffers (`ARCH-003`).
- **Accented Edges brightness mapping.** Edge Brightness 25 = edges not outlined (source: AliveColors, corroborated by the Help's chalk/ink description); below 25 darkens, above 25 lightens. Treat the exact crossover as behavioral.

## Rust module mapping

Proposed under `pictura-filters::brush_stroke` (`ARCH-002`), implementing the shared `Filter` trait (`FILT-001`):

- `pictura-filters::brush_stroke::accented_edges` — `AccentedEdges { edge_width, edge_brightness, smoothness }`.
- `pictura-filters::brush_stroke::angled_strokes` — `AngledStrokes { direction_balance, stroke_length, sharpness }`.
- `pictura-filters::brush_stroke::crosshatch` — `Crosshatch { stroke_length, sharpness, strength }`.
- `pictura-filters::brush_stroke::dark_strokes` — `DarkStrokes { balance, black_intensity, white_intensity }`.
- `pictura-filters::brush_stroke::ink_outlines` — `InkOutlines { stroke_length, dark_intensity, light_intensity }`.
- `pictura-filters::brush_stroke::spatter` — `Spatter { spray_radius, smoothness }`; seeded RNG.
- `pictura-filters::brush_stroke::sprayed_strokes` — `SprayedStrokes { stroke_length, spray_radius, direction: StrokeDirection }`; `enum StrokeDirection { RightDiagonal, Horizontal, LeftDiagonal, Vertical }`.
- `pictura-filters::brush_stroke::sumi_e` — `SumiE { stroke_width, stroke_pressure, contrast }`.
- Shared helpers: `pictura-filters::kernel::edge` (gradient/orientation), `pictura-filters::kernel::stroke`, `pictura-filters::kernel::noise::SeededRng`.

Crossing types: `Rect`, `TileView`, `StrokeDirection`, `Seed(u64)`. No Qt types.

## Qt6 component mapping

Widgets, matching the CS6 Filter Gallery; pixel work in Rust. Shared gallery/preview/progress widgets are in `FILT-001`.

| Proposal | Base | Responsibility |
|---|---|---|
| `BrushStrokeOptionsPane` | `QWidget` (`QStackedWidget` page) | Hosts the selected Brush Stroke filter's controls |
| `AccentedEdgesPane` … `SumiEPane` | `QWidget` | One pane per filter; sliders via `FilterSliderSpin` |
| `StrokeDirectionCombo` | `QComboBox` | Right Diagonal / Horizontal / Left Diagonal / Vertical (Sprayed Strokes, and Graphic Pen in `FILT-082`) |
| `FilterGalleryDialog` | `QDialog` | Shared category list, thumbnails, options stack, applied list, preview (`FILT-001`) |

Panes are generated from the filter's parameter descriptor; `StrokeDirectionCombo` is the only family-specific control type.

## Data-model impact

- **Destructive apply:** no persistent field; one `HistoryRecord::FilterOp { filter_id, roi, params_blob, seed, before_tiles, after_hash }` per committed apply (`ARCH-009`); scatter filters store the `seed`.
- **Smart Object:** each Brush Stroke filter (or a gallery stack) becomes a Smart Filter entry `{ filter_id, params, blend, opacity, enabled }`; the gallery stack is one grouped entry (`LAY-021`). No new document-model nodes.
- **Serialization:** descriptors are written through the Smart Filter/Filter Effects path (`LAY-021`); the per-parameter `FXid`/`FEid` key mapping is **unsourced**.
- Undo granularity is one apply.

## Edge cases

- **Mode:** Brush Stroke filters target 8-bit RGB/Grayscale/Multichannel images; CMYK/Lab support is per filter and must be capability-gated (never convert).
- **Depth:** 16-bit/32-bit are not in Adobe's filter depth lists → 8-bit only; disable/show warning at higher depth.
- **Selection edge:** edge-detection stages read an apron outside the selection; decide and pin the sampling behavior (see `FILT-001`).
- **1×1 / 1-px documents:** stroke rasterization must handle zero-extent neighborhoods without panics.
- **Huge PSB:** tile-local; scattered strokes must not allocate full-canvas buffers; Spatter/Sprayed Strokes bound their sampling.
- **Randomness:** seeded for reproducible redo; two identical applies match bit-for-bit.
- **Cancellation:** atomic no-op (`ARCH-003`).
- **GPU:** CPU fallback must be correct; no kernel requires a GPU (`ARCH-006`).
- **Empty/locked layer:** no-op or refuse per `FILT-001`.

## Parity acceptance criteria

1. Given an 8-bit RGB document, each of the 8 Brush Stroke filters appears under `Filter > Filter Gallery > Brush Stroke` and in `Filter > Brush Stroke`, opens its options, produces a non-empty effect, and adds exactly one history state.
2. Given a 16-bit/32-bit document, Brush Stroke filters are disabled or show the Smart Filter warning; applying is refused with no conversion.
3. Given two Brush Stroke filters in the gallery, reordering changes the result; hiding one removes its contribution.
4. Given `Crosshatch` Strength 1 vs 3, the number of visible hatching passes increases; Growth is monotonic.
5. Given `Accented Edges` at Edge Brightness 25, edges are not outlined (neutral); below 25 accents darken; above 25 accents lighten.
6. Given `Dark Strokes`, raising Balance increases the proportion of dark strokes while Black/White Intensity independently darken/lighten their respective regions.
7. Given `Spatter`/`Sprayed Strokes`, increasing Spray Radius scatters strokes over a wider area; results are reproducible from the stored seed.
8. Given `Ink Outlines`, fine narrow line detail is retained over the original; increasing Dark/Light Intensity strengthens the dark/light line components.
9. Given identical input, parameters, and seed, CPU and GPU paths agree within 1 LSB (8-bit) and redo is bit-exact.
10. Given a Smart Object, applying a Brush Stroke filter adds a Smart Filters entry, leaves the pixels unchanged, and reopening restores saved parameters.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — Adobe Photoshop CS6 Help reference (downloaded to `/tmp` and text-extracted with `pdftotext`). Established: the Brush Stroke family introduction and that **all Brush Stroke filters can be applied through the Filter Gallery**; the one-line behavior of all 8 filters; **Crosshatch Strength 1–3**; the memory-intensive list naming **Spatter and Sprayed Strokes**; the Fade note that Fade modifies Brush Strokes filters; and the Filter Gallery mechanics/keys and 8-bit-only statement. Primary source.
- `https://web.archive.org/web/20140129073744id_/http://helpx.adobe.com/photoshop-elements/using/brush-stroke-filters.html` — Photoshop Elements Help, "Brush Stroke filters" (Adobe content, archived). Established the exact **control names**: Accented Edges (edge width, edge brightness, smoothness), Angled Strokes (stroke direction balance, stroke length, sharpness), Crosshatch (stroke length, sharpness, strength), Dark Strokes (stroke balance, black and white intensity), Ink Outlines (stroke length, dark and light intensity), Spatter (spray radius, smoothness), Sprayed Strokes (stroke length, spray radius, stroke direction), Sumi-e (stroke width, stroke pressure, contrast).
- `https://alivecolors.com/en/tutorial/effects/brush-strokes.php` — AliveColors "Brush Strokes" reference (an independent Photoshop-compatible editor). Corroborated **Accented Edges** Edge Width 1–14, Edge Brightness 0–50 (25 = no outline), Smoothness 1–15, and **Dark Strokes** Balance 0–10, Black Intensity 0–10, White Intensity 0–10. Community/third-party.
- `https://iezombie.net/texture-photoshop-filter` — cross-check of Photoshop filter option naming (texture filters); corroborates parameter-naming conventions only.
- `https://web.archive.org/web/20220628182932id_/https://www.pcworld.com/article/393412/how-photoshop-artistic-filters-work-with-examples.html` — PCWorld established that the Filter Gallery has six categories including **Brush Strokes**; used to confirm gallery membership. Community source.

Searches performed (not documents fetched): DuckDuckGo/Brave for Brush Stroke filter ranges; unconfirmed values are marked *(inferred)*.

Not used: `helpx.adobe.com` live pages return HTTP 403; archived copies were used instead.

## Open questions

- **Which Brush Strokes support CMYK/Lab.** The 8-bit RGB/Grayscale/Multichannel support is documented for some filters in community pages; per-filter CMYK/Lab capability must be read back from CS6.
- **All defaults.** Adobe does not publish Filter Gallery defaults; every "Default" above is *(inferred)*. Resolve by a scripted CS6 dialog read.
- **Unlisted ranges.** Only Crosshatch Strength (1–3) and the Accented Edges/Dark Strokes ranges are sourced; Angled Strokes, Ink Outlines, Spatter, Sprayed Strokes, Sumi-e ranges are *(inferred)*.
- **Stroke Direction enumeration/default.** Whether CS6's Sprayed Strokes offers exactly the four directions and the default direction is *(inferred)* from the CS6 dialog family; confirm.
- **Scatter determinism.** Whether Photoshop's Spatter/Sprayed Strokes randomness is seeded/reproducible or fresh per apply is unknown; our design choice is a stored seed, which may not match Adobe's exact output (behavioral parity only).
- **`FXid`/`FEid` parameter mapping.** See `LAY-021`; the per-filter parameter descriptor keys are unsourced.
- **Spatter vs. Sprayed Strokes overlap.** Both scatter ink; the exact differentiating model (radius vs. length/direction) is behavioral only.
