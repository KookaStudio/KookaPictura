# Texture Filters

- **Spec ID:** `FILT-083`
- **Status:** `Draft`
- **Parity tier:** `Core` — the Texture submenu and its Filter Gallery category are in CS6 Standard and Extended.
- **New in CS6:** `No` — the 6 Texture filters are unchanged from CS5.
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

**Texture filters.** The CS6 Help describes the Texture filters as simulating the appearance of depth or substance, or adding an organic look. They appear as the **Texture** category in the Filter Gallery (one of the six gallery categories: Artistic, Brush Strokes, Distort, Sketch, Stylize, Texture).

The family has **6** filters. All are **8-bit only** and run in the Filter Gallery.

| Filter | CS6 Help behavior (sourced) |
|---|---|
| **Craquelure** | Paints the image onto a high-relief plaster surface, producing a fine network of cracks that follow the image contours. Useful for embossing effects on broad-range color/grayscale images. |
| **Grain** | Adds texture by simulating different kinds of grain. Types: **Regular, Soft, Sprinkles, Clumped, Contrasty, Enlarged, Stippled, Horizontal, Vertical, Speckle** *(all 10 sourced from the CS6 PDF)*. The sprinkles and stippled types use the background color *(PSE Help)*. |
| **Mosaic Tiles** | Draws the image as if made of small chips/tiles with grout between them. (Distinct from `Pixelate > Mosaic`, which makes blocks of different-colored pixels.) |
| **Patchwork** | Breaks the image into squares filled with the predominant color, randomly raising/lowering tile depth to replicate highlights/shadows. |
| **Stained Glass** | Repaints the image as single-colored adjacent cells outlined in the foreground color. |
| **Texturizer** | Applies a selected or loaded texture (e.g. canvas, brick, or a glass-block look); the texture is lit/scaled/relief-adjusted. |

**Filter Gallery mechanics (shared).** One dialog: preview, category thumbnails, selected-effect options, applied-effect list. Effects are **cumulative and applied in list order**, **reorderable by drag**, **hideable with the eye icon**, **deletable**. The gallery is **8-bit-per-channel only**. On a Smart Object the stack becomes one grouped "Filter Gallery" entry (`LAY-021`). Shared keys, Fade, and pipeline are in `FILT-001`.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Filter > Texture > <filter>` | menu submenu | — | Same filters also inside the Filter Gallery |
| `Filter > Filter Gallery > Texture` | dialog category | — | Cumulative stack; canonical CS6 route |
| Filter Gallery — filter thumbnail | click | `Alt`-click applies on top *(PDF)* | Adds the effect |
| Filter Gallery — options pane | controls | — | Per-filter sliders/combo |
| Filter Gallery — applied list | drag / eye / delete | — | Reorder, hide, delete |
| Filter Gallery — Cancel button | button | `Ctrl`→Default, `Alt`→Reset *(PDF)* | Button changes label |
| `Edit > Fade` | dialog | — *(shortcut unverified)* | Opacity + mode of the last effect |
| Layers panel — Smart Filters line | panel | — | Stack appears as one "Filter Gallery" entry |

## Parameters & ranges

All Texture filters run in **8-bit** only. Control names are from the CS6 Help PDF and Adobe/PSE Help; ranges are *(inferred)* from the CS6 dialog and community documentation unless marked *(sourced)* (the CS6 PDF publishes the **Grain Type** enumeration; `iezombie` names the controls).

### Craquelure
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Crack Spacing | slider int | 10 *(inferred)* | 2–100 *(inferred)* | Distance between cracks |
| Crack Depth | slider int | 6 *(inferred)* | 0–10 *(inferred)* | |
| Crack Brightness | slider int | 9 *(inferred)* | 0–10 *(inferred)* | |

### Grain
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Intensity | slider int | 40 *(inferred)* | 0–100 *(inferred)* | Grain intensity |
| Contrast | slider int | 50 *(inferred)* | 0–100 *(inferred)* | |
| Grain Type | enum | Regular *(inferred)* | Regular / Soft / Sprinkles / Clumped / Contrasty / Enlarged / Stippled / Horizontal / Vertical / Speckle *(sourced)* | Sprinkles/Stippled use the background color |

### Mosaic Tiles
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Tile Size | slider int | 12 *(inferred)* | 2–100 *(inferred)* | Chip size |
| Grout Width | slider int | 3 *(inferred)* | 1–15 *(inferred)* | |
| Lighten Grout | slider int | 1 *(inferred)* | 0–10 *(inferred)* | Grout brightness |

### Patchwork
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Square Size | slider int | 5 *(inferred)* | 0–10 *(inferred)* | |
| Relief | slider int | 8 *(inferred)* | 0–25 *(inferred)* | Tile depth variation |

### Stained Glass
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Cell Size | slider int | 10 *(inferred)* | 2–50 *(inferred)* | Cell size; larger speeds the filter *(CS6 PDF)* |
| Border Thickness | slider int | 4 *(inferred)* | 1–20 *(inferred)* | Minimum reachable value is 1 *(community)* |
| Light Intensity | slider int | 5 *(inferred)* | 0–10 *(inferred)* | |

### Texturizer
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Texture | enum | Canvas *(inferred)* | Brick / Burlap / Canvas / Sandstone + Load Texture *(sourced shape)* | Shared texture options (`FILT-080`) |
| Scaling | slider % | 100 *(inferred)* | 50–200 *(inferred)* | |
| Relief | slider int | 4 *(inferred)* | 0–50 *(inferred)* | |
| Light Direction | enum | Top *(inferred)* | 8 directions *(inferred)* | |
| Invert | checkbox | off | on / off | |

`(inferred)` defaults and ranges come from the CS6 dialog and community documentation, not the fetched CS6 PDF; read them back from a CS6 build (see `## Open questions`).

## Algorithms & pipeline

All Texture kernels are closed; **behavioral parity only, algorithm TBD**. Shared application pipeline in `FILT-001`.

- **Gallery stack.** Each filter is an independent stage in `pictura-filters::gallery`, evaluated in list order on the 8-bit image.
- **Craquelure (behavioral model).** Help describes painting onto high-relief plaster with a crack network following contours: a plausible model is a **cellular/crack pattern generated from the image luminance as a height field**, then lit (emboss) so crack edges catch light. The exact crack generator is closed.
- **Mosaic Tiles.** A tiling grid with grout: possibly a jittered/irregular cell tessellation with averaged cell color and a grout band between cells; `Lighten Grout` lifts the grout luminance. Contrast with `FILT-084` Mosaic (square-block averaging) per the Help note.
- **Patchwork (behavioral model).** Square blocks filled with the predominant color; squares are raised/lowered in depth (relief) to create highlights/shadows — a per-square luminance offset plus optional bevel.
- **Stained Glass.** A cell segmentation (e.g. watershed/Voronoi-like) with each cell flat-filled by its average color and a foreground-colored border; `Cell Size` sets the segmentation scale and `Light Intensity` modulates cell luminance.
- **Grain.** An additive/multiplicative noise field with ten distinct spatial/temporal distributions (Regular, Soft, Sprinkles, Clumped, Contrasty, Enlarged, Stippled, Horizontal, Vertical, Speckle); Sprinkles/Stippled draw from the background color. Use a **seeded RNG** so redo is reproducible.
- **Texturizer.** Tiles a selected/loaded grayscale height map under the image and lights it (Scaling, Relief, Light Direction, Invert) — the same shared texture model as Rough Pastels/Underpainting (`FILT-080`) and Conté Crayon (`FILT-082`).
- **Memory/performance.** Stained Glass is named in the CS6 Help's performance tip (increase cell size to speed it up); the implementation must be tile-local with a bounded apron (`ARCH-003`).

## Rust module mapping

Proposed under `pictura-filters::texture` (`ARCH-002`), implementing the shared `Filter` trait (`FILT-001`):

- `pictura-filters::texture::craquelure` — `Craquelure { crack_spacing, crack_depth, crack_brightness }`.
- `pictura-filters::texture::grain` — `Grain { intensity, contrast, grain_type: GrainType }`; `enum GrainType { Regular, Soft, Sprinkles, Clumped, Contrasty, Enlarged, Stippled, Horizontal, Vertical, Speckle }`; seeded RNG.
- `pictura-filters::texture::mosaic_tiles` — `MosaicTiles { tile_size, grout_width, lighten_grout }`.
- `pictura-filters::texture::patchwork` — `Patchwork { square_size, relief }`.
- `pictura-filters::texture::stained_glass` — `StainedGlass { cell_size, border_thickness, light_intensity }`.
- `pictura-filters::texture::texturizer` — `Texturizer { texture: TextureOptions }` (reuses `FILT-080`).
- Shared helpers: `pictura-filters::kernel::relief`, `pictura-filters::kernel::tessellate` (cellular/Voronoi), `pictura-filters::kernel::noise::SeededRng`, `pictura-filters::kernel::tile_grid`.

Crossing types: `Rect`, `TileView`, `Rgba` (foreground/background), `GrainType`, `LightDir`, `TextureRef`, `Seed(u64)`. No Qt types.

## Qt6 component mapping

Widgets, matching the CS6 Filter Gallery; pixel work in Rust. Shared gallery/preview/progress widgets and `TextureOptionsPane` are in `FILT-001`/`FILT-080`.

| Proposal | Base | Responsibility |
|---|---|---|
| `TextureOptionsPane` | `QWidget` (`QStackedWidget` page) | Hosts the selected Texture filter's controls |
| `CraquelurePane` … `TexturizerPane` | `QWidget` | One pane per filter; sliders via `FilterSliderSpin` |
| `GrainTypeCombo` | `QComboBox` | The 10 grain types |
| `SharedTexturePane` | `QWidget` | Texture preset + Load Texture, Scaling, Relief, Light Direction, Invert (Texturizer) |
| `FilterGalleryDialog` | `QDialog` | Shared category list, thumbnails, options stack, applied list, preview (`FILT-001`) |

Panes are generated from each filter's parameter descriptor; `GrainTypeCombo` is the only family-specific control type.

## Data-model impact

- **Destructive apply:** no persistent field; one `HistoryRecord::FilterOp { filter_id, roi, params_blob, seed, before_tiles, after_hash }` per committed apply (`ARCH-009`); grain stores the `seed`.
- **Smart Object:** each Texture filter (or a gallery stack) becomes a Smart Filter entry `{ filter_id, params, blend, opacity, enabled }`; the stack is one grouped entry (`LAY-021`).
- **Loaded textures:** a Texturizer "Load Texture" file is referenced by the filter descriptor; embed or reference the texture bytes so a saved document is self-contained (design decision — embed, matching PSD behavior). No new document-model node.
- **Serialization:** descriptors are written through the Smart Filter/Filter Effects path (`LAY-021`); per-parameter `FXid`/`FEid` key mapping is **unsourced**.
- Undo granularity is one apply.

## Edge cases

- **Mode:** Texture filters target 8-bit RGB/Grayscale/Multichannel; CMYK/Lab support is per filter and must be capability-gated (never convert).
- **Depth:** 16-bit/32-bit are not in Adobe's filter depth lists → 8-bit only; disable/show warning at higher depth.
- **Foreground/background identity:** Grain (Sprinkles/Stippled), Stained Glass border, and Texturizer use fg/bg; fg == bg can flatten the result; no divide-by-zero.
- **Loaded texture size.** Texturizer must tile arbitrary texture sizes across the ROI without assuming the texture is larger than a tile; missing/corrupt texture file must refuse with a message, not crash.
- **Degenerate cell sizes.** Stained Glass cell size 2, Mosaic Tiles tile size 2, Patchwork square size 0 must not loop or produce empty cells.
- **Huge PSB:** tile-local; cellular segmentation must be computed per tile or on a downscaled proxy with seam handling; grain must be procedural per tile with a coordinate-derived seed (not a full-canvas buffer).
- **Cancellation:** atomic no-op (`ARCH-003`).
- **GPU:** CPU fallback must be correct; no kernel requires a GPU (`ARCH-006`).
- **Empty/locked layer:** no-op or refuse per `FILT-001`.

## Parity acceptance criteria

1. Given an 8-bit RGB document, each of the 6 Texture filters appears under `Filter > Filter Gallery > Texture` and in `Filter > Texture`, opens its options, produces a non-empty effect, and adds exactly one history state.
2. Given a 16-bit/32-bit document, Texture filters are disabled or show the Smart Filter warning; applying is refused with no conversion.
3. Given two Texture filters in the gallery, reordering changes the result; hiding one removes its contribution.
4. Given `Grain`, each of the 10 Grain Types produces a distinct texture; Sprinkles/Stippled use the background color; results are reproducible from the stored seed.
5. Given `Craquelure`, Crack Spacing changes crack frequency, Crack Depth changes apparent crack depth, and Crack Brightness changes crack luminance.
6. Given `Mosaic Tiles`, Tile Size changes the chip grid, Grout Width changes the grout band, and Lighten Grout lifts grout luminance.
7. Given `Stained Glass`, Cell Size changes the cell scale (and larger cells render faster, matching the CS6 performance note); the border is drawn in the foreground color.
8. Given `Texturizer`, selecting a preset or loading a file changes the surface; Scaling scales it, Relief changes depth, Light Direction changes apparent illumination, and Invert flips it.
9. Given identical input, parameters, and seed, CPU and GPU paths agree within 1 LSB (8-bit) and redo is bit-exact.
10. Given a Smart Object, applying a Texture filter adds a Smart Filters entry, leaves the pixels unchanged, and reopening restores saved parameters.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — Adobe Photoshop CS6 Help reference (downloaded to `/tmp` and text-extracted with `pdftotext`). Established: the Texture family introduction; the one-line behavior of all 6 filters; the **10 Grain Types** (Regular, Soft, Sprinkles, Clumped, Contrasty, Enlarged, Stippled, Horizontal, Vertical, Speckle); the Mosaic Tiles vs Pixelate Mosaic distinction; the Stained Glass performance tip (increase cell size); the shared texture options description ("canvas, brick, burlap, or sandstone"); the Filter Gallery mechanics/keys and 8-bit-only statement. Primary source.
- `https://web.archive.org/web/2024id_/https://helpx.adobe.com/photoshop-elements/desktop/guided-edits-effects-and-filters/texture-filters.html` — Photoshop Elements Help, "Texture filters" (Adobe content, archived). Established the exact **control names**: Craquelure (crack spacing, depth, brightness), Grain (grain intensity, contrast, type), Mosaic Tiles (tile size, grout width, lighten grout), Patchwork (square size, relief), Stained Glass (cell size, border thickness, light intensity), Texturizer (texture types/loaded file). Also established that Sprinkles/Stippled grain use the background color.
- `https://iezombie.net/texture-photoshop-filter` — named the per-filter option lists (Craquelure: Crack Spacing/Depth/Brightness; Grain: Intensity/Contrast/Grain Type; Mosaic Tiles: Tile Size/Grout Width/Lighten Grout; Patchwork: Square Size/Relief; Stained Glass: Cell Size/Border Thickness/Light Intensity; Texturizer: Texture Type/Scaling/Relief/Light Direction/Invert). Community source.
- `https://www.underwaterphotography.com/PhotoShop/PhotoShop/1_15_16_0.html` — mirror of the classic Photoshop Texture filter reference; cross-checked descriptions. Community mirror.
- `https://web.archive.org/web/20220724170527id_/https://helpx.adobe.com/photoshop-elements/using/artistic-filters.html` — cross-checked the shared texture-options wording (canvas, brick, burlap, sandstone). Adobe content, archived.

Searches performed (not documents fetched): DuckDuckGo/Brave for Texture filter ranges/defaults; unconfirmed values are marked *(inferred)*.

Not used: `helpx.adobe.com` live pages return HTTP 403; archived copies were used instead.

## Open questions

- **All defaults.** Adobe does not publish Filter Gallery defaults; every "Default" above is *(inferred)*. Resolve by a scripted CS6 dialog read.
- **All ranges.** The CS6 PDF publishes no Texture dialog ranges; all ranges above are *(inferred)* from the CS6 dialog/community and need confirmation.
- **Craquelure/Stained Glass `cell`/`crack` units and algorithms.** The exact crack generator and cell segmentation are closed; behavioral parity only. Resolve with reference renders.
- **Texturizer texture presets.** The exact built-in height maps (and whether "Load Texture" stores an embedded copy) are undocumented. Resolve against a CS6-authored PSD.
- **Patchwork "predominant color" statistic.** Whether the square fill is a mean, mode, or median of the block is unknown; Resolve with synthetic block tests.
- **`FXid`/`FEid` parameter mapping.** See `LAY-021`; the per-filter parameter descriptor keys are unsourced.
