# Artistic Filters

- **Spec ID:** `FILT-080`
- **Status:** `Draft`
- **Parity tier:** `Core` — the Artistic submenu and the Filter Gallery are available in CS6 Standard and Extended.
- **New in CS6:** `No` — the 15 Artistic filters and their options are unchanged from CS5. CS6's new filters (Blur Gallery, Oil Paint, Camera Raw Filter, Adaptive Wide Angle) live elsewhere.
- **Depends on:** `FILT-001` filters-overview, `LAY-021` smart-filters, `LAY-020` smart-objects, `LAY-010` blend-modes, `IMG-005` bit-depth-and-conversion, `ARCH-006` gpu-rendering-pipeline, `ARCH-009` undo-history, `ARCH-008` document-model, `ARCH-003` performance-targets.

> All crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Adobe's kernels are closed: this document targets
> **behavioral parity only, algorithm TBD**. Control names are taken from Adobe
> Help; ranges marked *(sourced)* are from the fetched community/Adobe references
> listed in `## Sources`, and ranges/defaults not published by Adobe are marked
> *(inferred)* and must be confirmed against a CS6 build.

## CS6 behavior

**Artistic filters.** Per the CS6 Help, they give painterly and artistic results for fine-arts or commercial work by imitating natural or traditional media. All of them can be applied through the Filter Gallery.

The family has **15** filters. Each redraws the image to imitate a physical medium; in CS6 several use the **foreground and background colors** as their "paint" and "paper" and depend on the document's color mode (RGB/Grayscale/Multichannel — see Edge cases). Pictura's dialogs show no foreground or background color controls for these filters (see Algorithms & pipeline, Color-source dependency).

| Filter | CS6 Help behavior (sourced unless marked) |
|---|---|
| **Colored Pencil** | Draws the image with colored pencils on a solid background. Edges are retained and given a rough crosshatch appearance; the background color shows through smooth areas. |
| **Cutout** | Makes the image look like roughly cut colored paper. High-contrast images appear as silhouettes; color images are built from several paper layers. |
| **Dry Brush** | Dry-brush (between oil and watercolor) painting of the edges. Simplifies the image by reducing its color range to areas of common color. |
| **Film Grain** | Applies an even grain pattern to shadows and midtones and a smoother, more saturated pattern to lighter areas; used to hide banding and unify mixed-source elements. |
| **Fresco** | Coarse painting with short, rounded, hastily applied daubs. |
| **Neon Glow** | Adds glows to objects; colorizes the image while softening it. Glow color is chosen from a color box. |
| **Paint Daubs** | Painterly brush stamping; brush sizes 1–50 and brush **types** Simple, Light Rough, Dark Rough, Wide Sharp, Wide Blurry, Sparkle *(types and 1–50 size sourced from the CS6 PDF)*. |
| **Palette Knife** | Reduces detail to give the look of a thin painted canvas revealing texture underneath. |
| **Plastic Wrap** | Coats the image in shiny plastic, accentuating surface detail. |
| **Poster Edges** | Posterizes the image, finds edges, and draws black lines on them; broad areas get simple shading. |
| **Rough Pastels** | Applies pastel-chalk strokes on a textured background; bright areas appear thick, darker areas appear scraped to reveal texture. |
| **Smudge Stick** | Softens with short diagonal strokes that smear darker areas; lighter areas brighten and lose detail. |
| **Sponge** | Highly textured areas of contrasting color, simulating sponge painting. |
| **Underpainting** | Paints the image on a textured background, then paints the final image over it. |
| **Watercolor** | Watercolor-style painting with a medium, water-loaded brush; simplifies detail and saturates color where significant tonal changes occur at edges. |

**Filter Gallery mechanics (shared).** `Filter > Filter Gallery` opens one dialog with a preview, category thumbnails, the selected effect's options, and an applied-effect list. Effects are **cumulative and applied in list order**, can be **reordered by dragging**, **hidden with the eye icon**, and **deleted**. Clicking a category name shows its filter thumbnails; the thumbnail pane can be hidden. The gallery is **8-bit-per-channel only**; for such images most filters can be stacked cumulatively through the Filter Gallery. Applied to a Smart Object, the whole stack becomes one grouped Smart Filter entry named "Filter Gallery" (`LAY-021`). Shared dialog keys, Fade, and the application pipeline are in `FILT-001`.

> Note: the source (PCWorld) states there are "15 Artistic filter options"; CS6's Artistic set matches. PS Elements Help additionally exposes several non-CS6 filters (Comic, Graphic Novel, Pen and Ink) — those are **not** CS6 Artistic filters and are excluded here.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Filter > Artistic > <filter>` | menu submenu | — | CS6 also exposes the same filters inside the Filter Gallery |
| `Filter > Filter Gallery > Artistic` | dialog category | — | Cumulative stack; the canonical CS6 route |
| Filter Gallery — category name | click | — | Shows the 15 Artistic thumbnails |
| Filter Gallery — filter thumbnail | click | `Alt`-click applies on top *(PDF)* | Adds the effect |
| Filter Gallery — options pane | controls | — | Per-filter sliders/combo/color box |
| Filter Gallery — applied list | drag / eye / delete | — | Reorder, hide, delete |
| Filter Gallery — Cancel button | button | `Ctrl`→Default, `Alt`→Reset *(PDF)* | Button changes label |
| `Filter > Filter Gallery` — disclosure triangles | click | `Alt`-click toggles all *(PDF)* | Per-category roll-up |
| `Edit > Fade` | dialog | — *(sourced: exists; shortcut unverified in CS6 PDF)* | Opacity + mode of the last effect |
| Layers panel — Smart Filters line | panel | — | Gallery stack appears as one "Filter Gallery" entry |

## Parameters & ranges

Every Artistic filter runs in **8-bit** only (`FILT-001`). Parameter names marked *(Adobe Help)* are from Adobe's filter reference; ranges marked *(sourced)* come from the fetched community references (PCWorld; AliveColors for corroboration). Defaults and unmarked ranges are *(inferred)* from the CS6 dialog and **need verification**.

### Colored Pencil
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Pencil Width | slider int | 4 *(inferred)* | 1–24 *(sourced)* | Pencil stroke width |
| Stroke Pressure | slider int | 8 *(inferred)* | 0–15 *(sourced)* | Stroke darkness/force |
| Paper Brightness | slider int | 25 *(inferred)* | 0–50 *(sourced)* | Background brightness showing through |

Pictura shows no foreground or background color control: the paper is white until the pipeline passes the document background color.

### Cutout
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Number of Levels | slider int | 6 *(inferred)* | 2–8 *(sourced)* | Color quantization levels |
| Edge Simplicity | slider int | 5 *(inferred)* | 0–10 *(sourced)* | Corner-angle simplification; raise to speed the filter *(CS6 PDF)* |
| Edge Fidelity | slider int | 2 *(inferred)* | 1–3 *(sourced)* | Edge smoothness; lower to speed the filter *(CS6 PDF)* |

### Dry Brush
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Brush Size | slider int | 2 *(inferred)* | 1–50 *(inferred; CS6 dialog)* | PCWorld reports 0–10 — see Open questions |
| Brush Detail | slider int | 8 *(inferred)* | 1–12 *(inferred; CS6 dialog)* | |
| Texture | slider int | 1 *(inferred)* | 1–3 | Texture amount |

### Film Grain
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Grain | slider int | 10 *(inferred)* | 0–20 *(sourced)* | Grain amount |
| Highlight Area | slider int | 0 *(inferred)* | 0–20 *(sourced)* | Highlight coverage |
| Intensity | slider int | 0 *(inferred)* | 0–10 *(sourced)* | Grain contrast/intensity |

### Fresco
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Brush Size | slider int | 4 *(inferred)* | 1–50 *(inferred; CS6 dialog)* | PCWorld reports 0–10 — see Open questions |
| Brush Detail | slider int | 9 *(inferred)* | 1–12 *(inferred; CS6 dialog)* | |
| Texture | slider int | 1 *(inferred)* | 1–3 | Texture amount |

### Neon Glow
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Glow Size | slider int | 5 *(inferred)* | −24 … +24 *(sourced)* | Negative restricts glow to shadows; positive spreads to midtones/highlights |
| Glow Brightness | slider int | 15 *(inferred)* | 0–50 *(sourced)* | Glow luminance |
| Glow Color | color box | *(inferred)* | Color Picker | Sourced: chosen from the Color Picker |

### Paint Daubs
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Brush Size | slider int | 8 *(inferred)* | 1–50 *(sourced)* | |
| Sharpness | slider int | 5 *(inferred)* | 0–40 *(sourced)* | |
| Brush Type | enum | Simple *(inferred)* | Simple / Light Rough / Dark Rough / Wide Sharp / Wide Blurry / Sparkle *(sourced)* | CS6 PDF |

### Palette Knife
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Stroke Size | slider int | 1 *(inferred)* | 1–50 *(sourced)* | |
| Stroke Detail | slider int | 1 *(inferred)* | 1–3 *(sourced)* | |
| Softness | slider int | 1 *(inferred)* | 0–10 *(sourced)* | |

### Plastic Wrap
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Highlight Strength | slider int | 15 *(inferred)* | 0–20 *(sourced)* | |
| Detail | slider int | 5 *(inferred)* | 1–15 *(sourced)* | |
| Smoothness | slider int | 5 *(inferred)* | 1–15 *(sourced)* | |

### Poster Edges
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Edge Thickness | slider int | 2 *(inferred)* | 0–10 *(sourced)* | |
| Edge Intensity | slider int | 3 *(inferred)* | 0–10 *(sourced)* | |
| Posterization | slider int | 6 *(inferred)* | 0–10 *(PCWorld reports 0–6)* | Number of posterization levels — see Open questions |

### Rough Pastels
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Stroke Length | slider int | 6 *(inferred)* | 0–40 *(inferred)* | |
| Stroke Detail | slider int | 6 *(inferred)* | 1–20 *(inferred)* | |
| Texture | enum | Canvas *(inferred)* | Brick / Burlap / Canvas / Sandstone + Load Texture *(sourced shape)* | Texture presets named in Help |
| Scaling | slider % | 100 *(inferred)* | 50–200 *(inferred)* | Shared texture option |
| Relief | slider int | 4 *(inferred)* | 0–50 *(inferred)* | Shared texture option |
| Light Direction | enum | Top *(inferred)* | 8 directions *(inferred)* | Shared texture option |
| Invert | checkbox | off | on / off | Shared texture option |

### Smudge Stick
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Stroke Length | slider int | 1 *(inferred)* | 0–10 *(inferred)* | |
| Highlight Area | slider int | 3 *(inferred)* | 0–20 *(inferred)* | |
| Intensity | slider int | 1 *(inferred)* | 0–10 *(inferred)* | |

### Sponge
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Brush Size | slider int | 2 *(inferred)* | 0–10 *(inferred)* | |
| Definition | slider int | 12 *(inferred)* | 0–25 *(inferred)* | |
| Smoothness | slider int | 5 *(inferred)* | 1–15 *(inferred)* | |

### Underpainting
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Brush Size | slider int | 6 *(inferred)* | 0–40 *(inferred)* | |
| Texture Coverage | slider int | 16 *(inferred)* | 0–40 *(inferred)* | |
| Texture | enum | Canvas *(inferred)* | Brick / Burlap / Canvas / Sandstone + Load Texture *(sourced shape)* | |
| Scaling | slider % | 100 *(inferred)* | 50–200 *(inferred)* | |
| Relief | slider int | 4 *(inferred)* | 0–50 *(inferred)* | |
| Light Direction | enum | Top *(inferred)* | 8 directions *(inferred)* | |
| Invert | checkbox | off | on / off | |

### Watercolor
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Brush Detail | slider int | 14 *(inferred)* | 1–14 *(inferred)* | |
| Shadow Intensity | slider int | 1 *(inferred)* | 0–10 *(inferred)* | |
| Texture | slider int | 1 *(inferred)* | 1–3 | |

`(inferred)` defaults and ranges come from the CS6 dialog and community documentation, not the fetched CS6 PDF; they must be read back from a CS6 build (see `## Open questions`).

## Algorithms & pipeline

The Artistic kernels are closed; all of the following is **behavioral parity only, algorithm TBD**. The shared gallery/application pipeline (target resolution → depth/mode gate → apron → backend dispatch → tile iteration → selection composite → single undo commit) is in `FILT-001`.

- **Gallery stack.** Each Artistic filter is an independent stage in `pictura-filters::gallery`; the stack is evaluated in list order on the 8-bit image; drag-reorder/hide/delete mutate the stage list.
- **Color-source dependency:** in CS6 several filters read the **foreground/background colors** as ink/paper — Colored Pencil, Rough Pastels, Underpainting, Watercolor, and the Neon Glow color box *(sourced)*. Pictura removes the foreground/background color options from these filters: their dialogs show no color swatches, and the kernels take no foreground color. Colored Pencil keeps a background (paper) color, filled with white in place of the document background until the pipeline passes the document colors. Rough Pastels, Underpainting, and Watercolor take no colors. Neon Glow keeps its glow color box.
- **Texture options (Rough Pastels, Underpainting):** a surface preset (Brick/Burlap/Canvas/Sandstone) or a loaded file is tiled under the effect, with Scaling/Relief/Light-Direction/Invert controlling a height-to-light emboss of that surface. Modeling the texture as a loadable grayscale height map, lit by the chosen direction, reproduces the documented behavior.
- **Poster Edges** is qualitatively posterization + edge detection + black edge composite: a behavioral model is `posterize(src, levels=N)` combined with a gradient/edge magnitude (Sobel-like) thresholded to draw black lines; the exact Adobe edge kernel is closed.
- **Cutout** is a posterize/quantization step with an edge-simplification and fidelity bias; the "levels/simplicity/fidelity" triad suggests a region-quantization plus contour smoothing, but this is a behavioral model.
- **Film Grain** is an additive/multiplicative noise field whose amplitude varies by tonal zone (stronger in shadows/midtones, smoother in highlights); likely a deterministic or seeded PRNG (parity requires recording the seed for reproducible redo — design choice).
- **Colored Pencil, Dry Brush, Fresco, Palette Knife, Smudge Stick, Sponge, Watercolor** are a family of "local color reduction + directional/brush reconstruction" effects. A plausible behavioral decomposition is (a) an edge/orientation map, (b) a color simplification step, and (c) a stroke/daub renderer driven by orientation and tone. Adobe does not publish the steps; treat each as a black box to be matched, not re-derived.
- **Paint Daubs** is brush stamping with a size and sharpness and one of six brush types; the types most plausibly control stamp shape/hardness and grain, but the exact brushes are closed.

**Determinism.** Randomness (film grain, daubs jitter, sponge) must be seeded so that undo/redo and PSD round-trips reproduce results; store the seed in the history/effect record.

## Rust module mapping

Proposed under `pictura-filters::artistic` (`ARCH-002`), all implementing the shared `Filter` trait (`FILT-001`):

- `pictura-filters::artistic::colored_pencil` — `ColoredPencil { width, pressure, paper_brightness }`.
- `pictura-filters::artistic::cutout` — `Cutout { levels, edge_simplicity, edge_fidelity }`.
- `pictura-filters::artistic::dry_brush` — `DryBrush { brush_size, brush_detail, texture }`.
- `pictura-filters::artistic::film_grain` — `FilmGrain { grain, highlight_area, intensity }`; seeded RNG.
- `pictura-filters::artistic::fresco` — `Fresco { brush_size, brush_detail, texture }`.
- `pictura-filters::artistic::neon_glow` — `NeonGlow { glow_size, glow_brightness, glow_color: Rgba }`.
- `pictura-filters::artistic::paint_daubs` — `PaintDaubs { brush_size, sharpness, brush_type: BrushType }`; `enum BrushType { Simple, LightRough, DarkRough, WideSharp, WideBlurry, Sparkle }`.
- `pictura-filters::artistic::palette_knife` — `PaletteKnife { stroke_size, stroke_detail, softness }`.
- `pictura-filters::artistic::plastic_wrap` — `PlasticWrap { highlight_strength, detail, smoothness }`.
- `pictura-filters::artistic::poster_edges` — `PosterEdges { edge_thickness, edge_intensity, posterization }`.
- `pictura-filters::artistic::rough_pastels` — `RoughPastels { stroke_length, stroke_detail, texture: TextureOptions }`.
- `pictura-filters::artistic::smudge_stick` — `SmudgeStick { stroke_length, highlight_area, intensity }`.
- `pictura-filters::artistic::sponge` — `Sponge { brush_size, definition, smoothness }`.
- `pictura-filters::artistic::underpainting` — `Underpainting { brush_size, texture_coverage, texture: TextureOptions }`.
- `pictura-filters::artistic::watercolor` — `Watercolor { brush_detail, shadow_intensity, texture }`.
- `pictura-filters::artistic::TextureOptions` — `{ texture: TextureRef, scaling: Percent, relief: u8, light_direction: LightDir, invert: bool }`; shared with Rough Pastels, Underpainting, and (in `FILT-082`) Conté Crayon.
- Shared helpers: `pictura-filters::kernel::quantize` (posterize), `pictura-filters::kernel::edge` (gradient magnitude), `pictura-filters::kernel::noise::SeededRng`, `pictura-filters::kernel::stroke`.

Crossing types: `Rect`, `TileView`, `Rgba` (foreground/background/glow), `Percent`, `LightDir`, `TextureRef`, `Seed(u64)`. No Qt types.

## Qt6 component mapping

Widgets, matching the CS6 modal/gallery dialogs; pixel work in Rust. Shared gallery, preview, and progress widgets are specified in `FILT-001`; this family contributes per-filter option panes.

| Proposal | Base | Responsibility |
|---|---|---|
| `ArtisticOptionsPane` | `QWidget` (`QStackedWidget` page) | Hosts the selected Artistic filter's controls inside the Filter Gallery |
| `ColoredPencilPane` … `WatercolorPane` | `QWidget` | One pane per filter; sliders via `FilterSliderSpin` |
| `TextureOptionsPane` | `QWidget` | Texture preset combo + Load Texture, Scaling, Relief, Light Direction, Invert; shared by Rough Pastels / Underpainting |
| `BrushTypeCombo` | `QComboBox` | Simple / Light Rough / Dark Rough / Wide Sharp / Wide Blurry / Sparkle |
| `GlowColorButton` | `QToolButton`+`QColorDialog` | Neon Glow color box |
| `FilterGalleryDialog` | `QDialog` | Shared: category list, thumbnails, options stack, applied list, preview (`FILT-001`) |

Controls are model-driven from `FilterRegistry::params_schema()`; the pane is generated from the parameter descriptor so no per-filter hand-written widget is required beyond the shared control types.

## Data-model impact

- **Destructive apply:** no persistent document field; one `HistoryRecord::FilterOp { filter_id, roi, params_blob, seed, before_tiles, after_hash }` per committed apply (`ARCH-009`). Randomness stores the `seed`.
- **Smart Object:** each Artistic filter (or a whole Filter Gallery stack) becomes a Smart Filter entry `{ filter_id, params, blend, opacity, enabled }`; the gallery stack serializes as one grouped entry (`LAY-021`). No new document-model nodes.
- **Serialization:** the filter descriptor (filter id + typed params + seed) must be written through the Smart Filter/Filter Effects path (`LAY-021`); the mapping from each Artistic parameter to a PSD `FXid`/`FEid` key is **unsourced** (open question in `LAY-021`).
- **Color state:** the glow color is chosen in the dialog; Colored Pencil's paper color stands in for the document background (`ARCH-008`). The filter stores any color it uses, so the result stays reproducible.
- Undo granularity is one gallery-stack apply (destructive) or one Smart Filter entry mutation (non-destructive).

## Edge cases

- **Mode:** Artistic filters act on RGB, Grayscale, and Multichannel 8-bit images (`computerhope` per-filter pages); CMYK/Lab support is per filter and must be gated by a capability mask. On unsupported mode, disable with a reason (never convert).
- **Depth:** 16-bit/32-bit are not in Adobe's 16-/32-bit filter lists, so all Artistic filters are 8-bit only; at higher depth show the Smart Filter warning icon / disable the menu item.
- **Paper identity:** if Colored Pencil's paper color matches the image tones, the result can collapse to a flat or near-flat field; do not crash or divide by zero in the paper logic.
- **No selection:** apply to whole layer. **1×1 / 1-px** documents: brush- and texture-based kernels must not panic on zero-extent neighborhoods.
- **Huge PSB:** tile-local only; texture surfaces and brush stamps must be generated procedurally or tiled, not materialized full-canvas.
- **Randomness:** seeded for reproducible redo; two identical applies must match bit-for-bit.
- **Cancellation:** atomic no-op, no history state (`ARCH-003`).
- **GPU:** CPU fallback must be numerically correct; no Artistic kernel may require a GPU (`ARCH-006`).
- **Empty layer / locked pixels:** no-op or refuse per `FILT-001`.

## Parity acceptance criteria

1. Given an 8-bit RGB document, each of the 15 Artistic filters appears under `Filter > Filter Gallery > Artistic` and opens its options; applying it produces a non-empty effect and adds exactly one history state.
2. Given a 16-bit or 32-bit document, Artistic filters are disabled (or show the Smart Filter warning) and applying is refused; no silent conversion occurs.
3. Given two Artistic filters in the gallery, changing their order changes the result; hiding one removes its contribution; deleting restores the prior state.
4. Given the same input and parameters (with the recorded seed), the result is bit-identical across CPU/GPU and across redo.
5. Given `Cutout` with `Edge Simplicity` raised and `Edge Fidelity` lowered, the filtered result renders measurably faster than the default (matches the CS6 performance note) within the `ARCH-003` budget.
6. Given `Paint Daubs`, each of the six Brush Types produces a visually distinct result; changing Brush Size 1→50 increases stroke scale monotonically.
7. Given `Poster Edges`, the output contains posterized flat areas bounded by black lines whose thickness grows with Edge Thickness.
8. Given `Neon Glow`, the glow color appears in the output and the glow's spatial extent grows with Glow Size; a negative Glow Size confines the glow to shadows.
9. Given `Colored Pencil`, a different paper (background) color passed to the kernel changes the paper color visible through smooth areas, and changing Paper Brightness lightens/darkens it. The dialog shows no foreground or background color control.
10. Given a Smart Object, applying an Artistic filter adds a Smart Filters entry, leaves the underlying pixels unchanged, and reopening the entry restores the saved parameters.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — Adobe Photoshop CS6 Help reference (downloaded to `/tmp` and text-extracted with `pdftotext`). Established: the Artistic family introduction and the statement that **all Artistic filters can be applied through the Filter Gallery**; the one-line behavior of all 15 filters; `Cutout` "increase Edge Simplicity, decrease Edge Fidelity" as the performance tip; `Paint Daubs` brush sizes **1–50** and the six brush types; the Filter Gallery dialog parts, cumulative/reorder/hide/delete behavior, the 8-bit-only statement, the Filter Gallery keyboard table, and the "Filter Gallery" grouped Smart Filter entry. Primary source.
- `https://web.archive.org/web/20220628182932id_/https://www.pcworld.com/article/393412/how-photoshop-artistic-filters-work-with-examples.html` — PCWorld, "How Photoshop Artistic Filters work" (archived; live page 403s). Established the **control ranges** for Colored Pencil (1–24 / 0–15 / 0–50), Cutout (2–8 / 0–10 / 1–3), Dry Brush (0–10 / 0–10 / 1–3), Film Grain (0–20 / 0–20 / 0–10), Neon Glow (−24…+24 / 0–50 + color), Paint Daubs (1–50 / 0–40 / types), Palette Knife (1–50 / 1–3 / 0–10), Plastic Wrap (0–20 / 1–15 / 1–15), and Poster Edges (0–10 / 0–10 / 0–6); and "Six categories … Artistic, Brush Strokes, Distort, Sketch, Stylize, and Texture". Community source; note Dry Brush/Fresco range conflicts (see Open questions).
- `https://web.archive.org/web/20220724170527id_/https://helpx.adobe.com/photoshop-elements/using/artistic-filters.html` — Photoshop Elements Help, "Artistic filters" (Adobe content, archived). Established the exact **control names**: Colored Pencil (pencil width, stroke pressure, paper brightness), Cutout (number of levels, edge simplicity, edge fidelity), Dry Brush/Fresco (brush size, brush detail, texture), Film Grain (grain, highlight area, intensity), Neon Glow (glow size, glow brightness, glow color), Paint Daubs (brush size, sharpness, brush types), Palette Knife (stroke size, stroke detail, softness), Plastic Wrap (highlight strength, detail, smoothness), Poster Edges (edge thickness, edge intensity, posterization), Rough Pastels (stroke length, stroke detail, texture + scaling/relief/light direction/invert), Smudge Stick (stroke length, highlight area, intensity), Sponge (brush size, definition, smoothness), Underpainting (brush size, texture coverage, texture), Watercolor (brush detail, shadow intensity, texture).
- `https://www.underwaterphotography.com/PhotoShop/PhotoShop/1_15_13_0.html` — mirror of the classic Photoshop filter reference; used to cross-check the Artistic descriptions. Community mirror.
- `https://www.computerhope.com/jargon/p/photoshop-poster-edges.htm` — established that Poster Edges targets **8-bit RGB/Grayscale/Multichannel** and Smart Objects, and names its three parameters. Community source.

Searches performed (not documents fetched): DuckDuckGo/Brave via `html.duckduckgo.com`/`search.brave.com` for Artistic filter ranges and defaults; where used they are marked *(inferred)*.

Not used: `helpx.adobe.com` live pages return HTTP 403; archived copies were used instead.

## Open questions

- **Dry Brush / Fresco range conflict.** PCWorld reports Brush Size/Brush Detail **0–10**, while the CS6 dialog is widely reported as **1–50 / 1–12**. Which is CS6-at-launch? Resolve by reading the CS6 dialog directly.
- **Poster Edges posterization maximum.** PCWorld says **0–6**; older sources say **0–10**. Confirm the CS6 maximum.
- **All defaults.** Adobe does not publish Filter Gallery defaults; every "Default" above is *(inferred)*. Resolve by a scripted read of each CS6 dialog (or reference screenshots).
- **Unlisted ranges.** Smudge Stick, Sponge, Underpainting, Watercolor, Rough Pastels ranges are *(inferred)*. Same resolution.
- **Foreground/background semantics per filter.** Which of the 15 read fg/bg and in which role (ink vs paper) is only partly documented. Resolve empirically with distinct fg/bg swatches. Until then Pictura exposes no fg/bg controls for these filters.
- **Texture surface format.** Whether the Rough Pastels/Underpainting presets are fixed grayscale height maps and how Scaling/Relief/Light Direction combine is undocumented. Resolve by matching a reference render with a known loaded texture.
- **Randomness control.** Whether the Gallery exposes any seed/re-randomize control for Film Grain/sponge-daubs (Photoshop does not visibly expose one) is unconfirmed; our design choice is a hidden seeded RNG.
- **`FXid`/`FEid` parameter mapping.** See `LAY-021`; the per-filter parameter descriptor keys are unsourced, so lossless PSD round-trip of a Smart-Object-gallery is unproven.
