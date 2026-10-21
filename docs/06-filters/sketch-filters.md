# Sketch Filters

- **Spec ID:** `FILT-082`
- **Status:** `Draft`
- **Parity tier:** `Core` — the Sketch submenu and the Filter Gallery are in CS6 Standard and Extended.
- **New in CS6:** `No` — the 14 Sketch filters are unchanged from CS5.
- **Depends on:** `FILT-001` filters-overview, `LAY-021` smart-filters, `LAY-020` smart-objects, `FILT-080` artistic-filters (shared texture options), `IMG-005` bit-depth-and-conversion, `ARCH-006` gpu-rendering-pipeline, `ARCH-009` undo-history, `ARCH-008` document-model, `ARCH-003` performance-targets.

> All crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Adobe's kernels are closed: **behavioral parity only,
> algorithm TBD**. Control names are taken from the CS6 Help PDF and Adobe Help;
> ranges marked *(sourced)* are from the fetched references in `## Sources`;
> unmarked defaults/ranges are *(inferred)* and must be confirmed against a CS6
> build.

## CS6 behavior

**Sketch filters.** The CS6 Help describes the Sketch submenu as adding texture to an image, frequently for a 3D effect, and notes their use for fine-arts and hand-drawn looks. Many of these filters redraw the image using the **foreground and background color**, and all Sketch filters can be applied through the Filter Gallery.

The family has **14** filters. All are **8-bit only** and run in the Filter Gallery. Several depend on the document's **foreground/background colors** as ink/paper/graphite (see the per-filter table).

| Filter | CS6 Help behavior (sourced) |
|---|---|
| **Bas Relief** | Transforms the image to appear carved in low relief and lit to accent surface variation. Dark areas take the **foreground** color, light colors the **background** color. |
| **Chalk & Charcoal** | Redraws highlights and midtones in coarse chalk over a solid midtone-gray background; shadow areas become black diagonal charcoal lines. Charcoal = foreground; chalk = background. |
| **Charcoal** | Posterized, smudged effect. Major edges are bold; midtones are sketched with a diagonal stroke. Charcoal = foreground; paper = background. |
| **Chrome** | Renders the image as a polished chrome surface; highlights are high points, shadows low points. Help recommends adding contrast with Levels afterward. |
| **Conté Crayon** | Replicates dense dark / pure white Conté crayon texture. Foreground for dark areas, background for light areas. |
| **Graphic Pen** | Fine linear ink strokes capture detail; especially striking on scans. Foreground = ink, background = paper. |
| **Halftone Pattern** | Simulates a halftone screen while maintaining a continuous tonal range. |
| **Note Paper** | Looks like handmade paper; combines `Stylize > Emboss` and `Texture > Grain`. Dark areas appear as holes revealing the background color. |
| **Photocopy** | Simulates a photocopy: large dark areas copy only around their edges; midtones fall to solid black or white. |
| **Plaster** | Molds the image from 3D plaster and colorizes with foreground/background; dark areas raised, light recessed. |
| **Reticulation** | Simulates controlled shrinking/distortion of film emulsion; clumped shadows, lightly grained highlights. |
| **Stamp** | Simplifies the image to look rubber/wood stamped; best with black-and-white images. |
| **Torn Edges** | Reconstructs the image as ragged torn paper, colorized with the foreground/background colors; useful for text/high-contrast objects. |
| **Water Paper** | Blotchy daubs on fibrous damp paper, letting colors flow and blend. |

**Filter Gallery mechanics (shared).** One dialog: preview, category thumbnails, selected-effect options, applied-effect list. Effects are **cumulative and applied in list order**, **reorderable by drag**, **hideable with the eye icon**, **deletable**. The gallery is **8-bit-per-channel only**. On a Smart Object the stack becomes one grouped "Filter Gallery" entry (`LAY-021`). Shared keys, Fade, and pipeline are in `FILT-001`.

> Note: Photoshop Elements Help lists three **non-CS6** extra Sketch filters — **Comic**, **Graphic Novel**, and **Pen and Ink**. They are **not** part of the CS6 Sketch submenu and are excluded from this spec.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Filter > Sketch > <filter>` | menu submenu | — | Same filters also inside the Filter Gallery |
| `Filter > Filter Gallery > Sketch` | dialog category | — | Cumulative stack; canonical CS6 route |
| Filter Gallery — filter thumbnail | click | `Alt`-click applies on top *(PDF)* | Adds the effect |
| Filter Gallery — options pane | controls | — | Per-filter sliders/combo |
| Filter Gallery — applied list | drag / eye / delete | — | Reorder, hide, delete |
| Filter Gallery — Cancel button | button | `Ctrl`→Default, `Alt`→Reset *(PDF)* | Button changes label |
| `Edit > Fade` | dialog | — *(shortcut unverified)* | Opacity + mode of the last effect |
| Layers panel — Smart Filters line | panel | — | Stack appears as one "Filter Gallery" entry |

## Parameters & ranges

All Sketch filters run in **8-bit** only. Control names marked *(Adobe Help)* are from Adobe's filter reference. Ranges/defaults are *(inferred)* from the CS6 dialog and community documentation unless marked *(sourced)*; the CS6 PDF does not publish Sketch dialog ranges.

### Bas Relief
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Detail | slider int | 6 *(inferred)* | 1–15 *(inferred)* | Relief detail |
| Smoothness | slider int | 3 *(inferred)* | 1–15 *(inferred)* | |
| Light Direction | enum | Bottom *(inferred)* | Bottom / Bottom Left / Left / Top Left / Top / Top Right / Right / Bottom Right *(inferred)* | 8 directions |

### Chalk & Charcoal
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Charcoal Area | slider int | 6 *(inferred)* | 0–20 *(inferred)* | Shadow (charcoal) coverage |
| Chalk Area | slider int | 6 *(inferred)* | 0–20 *(inferred)* | Midtone (chalk) coverage |
| Stroke Pressure | slider int | 1 *(inferred)* | 0–5 *(inferred)* | |

### Charcoal
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Charcoal Thickness | slider int | 1 *(inferred)* | 1–7 *(inferred)* | |
| Detail | slider int | 3 *(inferred)* | 0–5 *(inferred)* | |
| Light/Dark Balance | slider int | 50 *(inferred)* | 0–100 *(inferred)* | 0 = dark, 100 = light |

### Chrome
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Detail | slider int | 4 *(inferred)* | 0–10 *(inferred)* | Surface detail |
| Smoothness | slider int | 7 *(inferred)* | 0–10 *(inferred)* | |

### Conté Crayon
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Foreground Level | slider int | 8 *(inferred)* | 1–15 *(inferred)* | Dark-area emphasis |
| Background Level | slider int | 7 *(inferred)* | 1–15 *(inferred)* | Light-area emphasis |
| Texture | enum | Canvas *(inferred)* | Brick / Burlap / Canvas / Sandstone + Load Texture *(sourced shape)* | Shared texture options (`FILT-080`) |
| Scaling | slider % | 100 *(inferred)* | 50–200 *(inferred)* | |
| Relief | slider int | 4 *(inferred)* | 0–50 *(inferred)* | |
| Light Direction | enum | Top *(inferred)* | 8 directions *(inferred)* | |
| Invert | checkbox | off | on / off | |

### Graphic Pen
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Stroke Length | slider int | 6 *(inferred)* | 1–15 *(inferred)* | |
| Light/Dark Balance | slider int | 50 *(inferred)* | 0–100 *(inferred)* | |
| Stroke Direction | enum | Right Diagonal *(inferred)* | Right Diagonal / Horizontal / Left Diagonal / Vertical *(inferred)* | |

### Halftone Pattern
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Size | slider int | 5 *(inferred)* | 1–12 *(inferred)* | Halftone cell size |
| Contrast | slider int | 5 *(inferred)* | 0–50 *(inferred)* | |
| Pattern Type | enum | Dot *(inferred)* | Dot / Line / Circle *(inferred)* | |

### Note Paper
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Image Balance | slider int | 25 *(inferred)* | 0–50 *(inferred)* | Foreground/background balance |
| Graininess | slider int | 10 *(inferred)* | 0–20 *(inferred)* | |
| Relief | slider int | 11 *(inferred)* | 0–25 *(inferred)* | |

### Photocopy
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Detail | slider int | 5 *(inferred)* | 0–24 *(inferred)* | |
| Darkness | slider int | 20 *(inferred)* | 1–50 *(inferred)* | |

### Plaster
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Image Balance | slider int | 25 *(inferred)* | 0–50 *(inferred)* | |
| Smoothness | slider int | 2 *(inferred)* | 0–15 *(inferred)* | |
| Light Direction | enum | Bottom *(inferred)* | 8 directions *(inferred)* | |

### Reticulation
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Density | slider int | 13 *(inferred)* | 0–50 *(inferred)* | Clump density |
| Black Level | slider int | 10 *(inferred)* | 0–50 *(inferred)* | Shadow (foreground) level |
| White Level | slider int | 40 *(inferred)* | 0–50 *(inferred)* | Highlight (background) level |

### Stamp
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Light/Dark Balance | slider int | 25 *(inferred)* | 0–50 *(inferred)* | |
| Smoothness | slider int | 5 *(inferred)* | 1–50 *(inferred)* | |

### Torn Edges
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Image Balance | slider int | 25 *(inferred)* | 0–50 *(inferred)* | |
| Smoothness | slider int | 1 *(inferred)* | 1–15 *(inferred)* | |
| Contrast | slider int | 8 *(inferred)* | 1–25 *(inferred)* | |

### Water Paper
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Fiber Length | slider int | 15 *(inferred)* | 3–50 *(inferred)* | |
| Brightness | slider int | 45 *(inferred)* | 0–100 *(inferred)* | |
| Contrast | slider int | 60 *(inferred)* | 0–100 *(inferred)* | |

`(inferred)` defaults and ranges come from the CS6 dialog and community documentation, not the fetched CS6 PDF; read them back from a CS6 build (see `## Open questions`).

## Algorithms & pipeline

All Sketch kernels are closed; **behavioral parity only, algorithm TBD**. Shared application pipeline in `FILT-001`.

- **Gallery stack.** Each filter is an independent stage in `pictura-filters::gallery`, evaluated in list order on the 8-bit image.
- **Color-source dependency (sourced):** Bas Relief, Chalk & Charcoal, Charcoal, Conté Crayon, Graphic Pen, Plaster, Reticulation, and Torn Edges read the **foreground/background colors** as ink/charcoal/paper. The pipeline must pass the current fg/bg colors to the kernel.
- **Relief/lighting model (Bas Relief, Plaster, Note Paper).** These are documented as low-relief/emboss-like: treat the input luminance as a height field, compute a directional gradient, and map the signed response to foreground (dark/recessed) vs background (light/raised) per the chosen Light Direction. `Note Paper` is explicitly described as `Stylize > Emboss` combined with `Texture > Grain`, so an emboss + grain decomposition is a strong behavioral model.
- **Halftone Pattern** is a halftone-screen simulation (Dot/Line/Circle) with a contrast control; standard halftoning applies (a rotated cell grid, threshold by luminance), but the exact cell kernel/angle is closed.
- **Chrome** is a highlight/shadow "polished surface" mapping; the Help's own workaround (add contrast with Levels afterward) suggests the filter output is intentionally low-contrast. Behavioral model only.
- **Charcoal / Chalk & Charcoal / Conté Crayon** are graphite/charcoal stroke renderers over posterized tonal zones; orientation and stroke pressure controls drive a directional stroke stage.
- **Graphic Pen / Photocopy / Stamp / Torn Edges / Water Paper** are threshold/quantization + stroke/texture renderers; Water Paper and Torn Edges add a fibrous/ragged texture (procedural paper grain), Photocopy a high-contrast threshold with edge-only dark areas, Stamp a simplification.
- **Reticulation** models film-emulsion clumping: a shadow-clumping + highlight-graining texture driven by Density / Black Level / White Level.
- **Determinism.** Textures/grain are pseudo-random; use a **seeded RNG** stored in the history/effect record for reproducible redo and PSD round-trips.

## Rust module mapping

Proposed under `pictura-filters::sketch` (`ARCH-002`), implementing the shared `Filter` trait (`FILT-001`):

- `pictura-filters::sketch::bas_relief` — `BasRelief { detail, smoothness, light_direction: LightDir }`.
- `pictura-filters::sketch::chalk_charcoal` — `ChalkCharcoal { charcoal_area, chalk_area, stroke_pressure }`.
- `pictura-filters::sketch::charcoal` — `Charcoal { thickness, detail, light_dark_balance }`.
- `pictura-filters::sketch::chrome` — `Chrome { detail, smoothness }`.
- `pictura-filters::sketch::conte_crayon` — `ConteCrayon { foreground_level, background_level, texture: TextureOptions }` (reuses `FILT-080`).
- `pictura-filters::sketch::graphic_pen` — `GraphicPen { stroke_length, light_dark_balance, direction: StrokeDirection }`.
- `pictura-filters::sketch::halftone_pattern` — `HalftonePattern { size, contrast, pattern: HalftoneType }`; `enum HalftoneType { Dot, Line, Circle }`.
- `pictura-filters::sketch::note_paper` — `NotePaper { image_balance, graininess, relief }`.
- `pictura-filters::sketch::photocopy` — `Photocopy { detail, darkness }`.
- `pictura-filters::sketch::plaster` — `Plaster { image_balance, smoothness, light_direction }`.
- `pictura-filters::sketch::reticulation` — `Reticulation { density, black_level, white_level }`.
- `pictura-filters::sketch::stamp` — `Stamp { light_dark_balance, smoothness }`.
- `pictura-filters::sketch::torn_edges` — `TornEdges { image_balance, smoothness, contrast }`.
- `pictura-filters::sketch::water_paper` — `WaterPaper { fiber_length, brightness, contrast }`.
- Shared helpers: `pictura-filters::kernel::relief` (height→directional gradient), `pictura-filters::kernel::halftone`, `pictura-filters::kernel::threshold`, `pictura-filters::kernel::noise::SeededRng`.

Crossing types: `Rect`, `TileView`, `Rgba` (fg/bg), `LightDir`, `StrokeDirection`, `TextureRef`, `Seed(u64)`. No Qt types.

## Qt6 component mapping

Widgets, matching the CS6 Filter Gallery; pixel work in Rust. Shared gallery/preview/progress widgets are in `FILT-001`.

| Proposal | Base | Responsibility |
|---|---|---|
| `SketchOptionsPane` | `QWidget` (`QStackedWidget` page) | Hosts the selected Sketch filter's controls |
| `BasReliefPane` … `WaterPaperPane` | `QWidget` | One pane per filter; sliders via `FilterSliderSpin` |
| `LightDirectionCombo` | `QComboBox` | 8-direction picker (Bas Relief, Plaster, and shared texture options) |
| `HalftoneTypeCombo` | `QComboBox` | Dot / Line / Circle |
| `StrokeDirectionCombo` | `QComboBox` | Right Diagonal / Horizontal / Left Diagonal / Vertical (Graphic Pen; shared with `FILT-081`) |
| `FilterGalleryDialog` | `QDialog` | Shared category list, thumbnails, options stack, applied list, preview (`FILT-001`) |

Panes are generated from each filter's parameter descriptor; the combo boxes are the only family-specific control types.

## Data-model impact

- **Destructive apply:** no persistent field; one `HistoryRecord::FilterOp { filter_id, roi, params_blob, seed, before_tiles, after_hash }` per committed apply (`ARCH-009`); textures store the `seed`.
- **Smart Object:** each Sketch filter (or a gallery stack) becomes a Smart Filter entry `{ filter_id, params, blend, opacity, enabled }`; the stack is one grouped entry (`LAY-021`).
- **Serialization:** descriptors are written through the Smart Filter/Filter Effects path (`LAY-021`); per-parameter `FXid`/`FEid` key mapping is **unsourced**.
- Undo granularity is one apply.

## Edge cases

- **Mode:** Sketch filters target 8-bit RGB/Grayscale/Multichannel images; CMYK/Lab support is per filter and must be capability-gated (never convert).
- **Depth:** 16-bit/32-bit are not in Adobe's filter depth lists → 8-bit only; disable/show warning at higher depth.
- **Foreground/background identity:** Bas Relief/Chalk & Charcoal/Charcoal/Conté Crayon/Plaster/Reticulation/Torn Edges can collapse or look flat when fg == bg; must not divide by zero in ink/paper logic.
- **Relief on flat input:** a completely uniform region has zero gradient; Bas Relief/Plaster/Note Paper must handle a null height field.
- **Halftone on 1×1:** cell grid must not loop infinitely for a cell larger than the ROI.
- **Huge PSB:** tile-local; procedural paper/grain textures must not be materialized full-canvas.
- **Randomness:** seeded for reproducible redo; two identical applies match bit-for-bit.
- **Cancellation:** atomic no-op (`ARCH-003`).
- **GPU:** CPU fallback must be correct; no kernel requires a GPU (`ARCH-006`).
- **Empty/locked layer:** no-op or refuse per `FILT-001`.

## Parity acceptance criteria

1. Given an 8-bit RGB document, each of the 14 Sketch filters appears under `Filter > Filter Gallery > Sketch` and in `Filter > Sketch`, opens its options, produces a non-empty effect, and adds exactly one history state.
2. Given a 16-bit/32-bit document, Sketch filters are disabled or show the Smart Filter warning; applying is refused with no conversion.
3. Given two Sketch filters in the gallery, reordering changes the result; hiding one removes its contribution.
4. Given `Bas Relief`, dark areas take the foreground color and light areas the background color; changing Light Direction moves the apparent illumination.
5. Given `Chalk & Charcoal`, Charcoal Area/Chalk Area independently change the shadow/midtone coverage; Stroke Pressure changes stroke darkness.
6. Given `Halftone Pattern`, Dot/Line/Circle produce distinguishable cell shapes; Size increases cell size; Contrast changes cell threshold.
7. Given `Photocopy`, large dark areas are reduced to edge-only marks and midtones collapse toward black or white.
8. Given `Note Paper`, the result visually combines an emboss and grain; changing Relief changes apparent depth and Graininess changes surface noise.
9. Given identical input, parameters, and seed, CPU and GPU paths agree within 1 LSB (8-bit) and redo is bit-exact.
10. Given a Smart Object, applying a Sketch filter adds a Smart Filters entry, leaves the pixels unchanged, and reopening restores saved parameters.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — Adobe Photoshop CS6 Help reference (downloaded to `/tmp` and text-extracted with `pdftotext`). Established: the Sketch family introduction and that **all Sketch filters can be applied through the Filter Gallery**; the one-line behavior of all 14 filters (including Bas Relief fg/bg roles, Conté Crayon, Graphic Pen ink/paper, Note Paper as Emboss+Grain, Plaster, Reticulation, Stamp, Torn Edges, Water Paper); the Filter Gallery mechanics/keys and 8-bit-only statement. Primary source.
- `https://web.archive.org/web/20220724170527id_/https://helpx.adobe.com/photoshop-elements/using/sketch-filters.html` — Photoshop Elements Help, "Sketch filters" (Adobe content, archived). Established the exact **control names**: Bas Relief (relief detail, smoothness), Chalk & Charcoal (stroke pressure, charcoal and chalk areas), Charcoal (thickness, detail, light/dark balance), Chrome (detail, smoothness), Conté Crayon (foreground/background level, texture options), Graphic Pen (stroke length/direction, light/dark balance), Halftone Pattern (size, contrast, pattern type), Note Paper (image balance, graininess, relief), Photocopy (detail, darkness), Plaster (image balance, smoothness, light direction), Reticulation (density, foreground/background levels), Stamp (smoothness, light/dark balance), Torn Edges (image balance, smoothness, contrast), Water Paper (fiber length, brightness, contrast). Also established the PSE-only extras **Comic**, **Graphic Novel**, **Pen and Ink** (excluded from CS6).
- `https://www.underwaterphotography.com/PhotoShop/PhotoShop/1_15_14_0.html` — mirror of the classic Photoshop Sketch filter reference; cross-checked the descriptions. Community mirror.
- `https://www.computerhope.com/jargon/p/photoshop-bas-relief.htm` and `https://www.computerhope.com/jargon/p/photoshop-note-paper.htm` — community pages naming Bas Relief/Note Paper parameters and confirming 8-bit RGB/Grayscale/Multichannel + Smart Object targeting.
- `https://iezombie.net/texture-photoshop-filter` — cross-check of Photoshop option naming conventions.

Searches performed (not documents fetched): DuckDuckGo/Brave for Sketch filter ranges/defaults; unconfirmed values are marked *(inferred)*.

Not used: `helpx.adobe.com` live pages return HTTP 403; archived copies were used instead.

## Open questions

- **Bas Relief Light Direction.** PSE Help omits it; Photoshop's Bas Relief dialog includes a Light Direction control. Confirm the CS6 control set and its default on a CS6 build.
- **All defaults.** Adobe does not publish Filter Gallery defaults; every "Default" above is *(inferred)*. Resolve by a scripted CS6 dialog read.
- **All ranges.** The CS6 PDF publishes no Sketch dialog ranges; all ranges above are *(inferred)* from the CS6 dialog/community and need confirmation.
- **Halftone cell geometry.** Cell size units and screen angle (if any) are undocumented; Dot/Line/Circle enumeration is *(inferred)*. Resolve with reference renders.
- **Reticulation "foreground/background levels".** PSE names them; Photoshop labels them Black Level / White Level. Confirm naming and mapping.
- **`FXid`/`FEid` parameter mapping.** See `LAY-021`; the per-filter parameter descriptor keys are unsourced.
- **PSE-only filters.** Whether any CS6.0 build exposed Comic/Graphic Novel/Pen and Ink under Sketch is unconfirmed; current evidence says no.
