# Pixelate Filters

- **Spec ID:** `FILT-084`
- **Status:** `Draft`
- **Parity tier:** `Core` — the Pixelate submenu is in CS6 Standard and Extended.
- **New in CS6:** `No` — the 7 Pixelate filters are unchanged from CS5.
- **Depends on:** `FILT-001` filters-overview, `LAY-021` smart-filters, `LAY-020` smart-objects, `IMG-005` bit-depth-and-conversion, `ARCH-006` gpu-rendering-pipeline, `ARCH-009` undo-history, `ARCH-008` document-model, `ARCH-003` performance-targets.

> All crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Adobe's kernels are closed: **behavioral parity only,
> algorithm TBD**. Ranges marked *(sourced)* are from the fetched references in
> `## Sources`; unmarked defaults/ranges are *(inferred)* and must be confirmed
> against a CS6 build.

## CS6 behavior

**Pixelate filters.** The CS6 Help describes the Pixelate submenu as sharply defining a selection by clumping together, into cells, pixels whose color values are similar.

**Scope note (important).** Unlike `FILT-080`–`FILT-083`, the Pixelate filters are **not part of the Filter Gallery** in Photoshop CS6. The Filter Gallery's six categories are **Artistic, Brush Strokes, Distort, Sketch, Stylize, and Texture**; Pixelate is a separate `Filter > Pixelate` submenu, and each filter opens its **own modal dialog**. This spec is grouped with the gallery families by catalogue convention, but the shared Filter-Gallery stack/reorder/eye behavior in `FILT-001` and `LAY-021` does **not** apply to Pixelate as a gallery stack. Pixelate filters can still be applied as **Smart Filters** on a Smart Object (they are not in CS6's non-smart exclusion list).

The family has **7** filters. All are **8-bit only** (no Pixelate filter appears in Adobe's 16- or 32-bit filter lists).

| Filter | CS6 Help behavior (sourced) |
|---|---|
| **Color Halftone** | Simulates an enlarged halftone screen on each channel: divides the image into rectangles and replaces each with a circle whose size is proportional to the rectangle's brightness. |
| **Crystallize** | Clumps pixels into solid-colored polygon shapes. |
| **Facet** | Clumps pixels of solid or similar color into blocks of like-colored pixels; useful for a hand-painted/abstract look. |
| **Fragment** | Creates **four copies** of the selection's pixels, averages them, and offsets them from each other. |
| **Mezzotint** | Converts the image to a random pattern of black-and-white areas (or fully saturated colors in a color image); choose a dot pattern from the Type menu. |
| **Mosaic** | Clumps pixels into square blocks; all pixels in a block are the same color, representing the area's colors. |
| **Pointillize** | Breaks color into randomly placed dots (pointillist painting), using the background color as the canvas between dots. |

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Filter > Pixelate > Color Halftone` | dialog | — | Own dialog with preview, Radius + Channel angles + Defaults |
| `Filter > Pixelate > Crystallize` | dialog | — | Cell Size; preview |
| `Filter > Pixelate > Facet` | menu (immediate) | — | No dialog, no parameters |
| `Filter > Pixelate > Fragment` | menu (immediate) | — | No dialog, no parameters |
| `Filter > Pixelate > Mezzotint` | dialog | — | Type combo; preview |
| `Filter > Pixelate > Mosaic` | dialog | — | Cell Size; preview |
| `Filter > Pixelate > Pointillize` | dialog | — | Cell Size; preview |
| `Edit > Fade` | dialog | — *(shortcut unverified)* | Opacity + mode of the last effect |
| Layers panel — Smart Filters line | panel | — | Applied as a Smart Filter (not a Filter Gallery group) |

**Not present:** there is no `Filter > Filter Gallery > Pixelate` category.

## Parameters & ranges

All Pixelate filters run in **8-bit** only. Ranges marked *(sourced)* are from Adobe/PSE Help (Color Halftone radius/angles) and AliveColors (corroboration); the rest are *(inferred)* from the CS6 dialog.

### Color Halftone
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Max Radius | slider int (px) | 5 *(inferred)* | 4–127 *(sourced)* | Maximum halftone-dot radius |
| Screen Angle Channel 1 | slider ° | 108 *(inferred)* | −360 … 360 *(sourced)* | Grayscale uses channel 1 only |
| Screen Angle Channel 2 | slider ° | 162 *(inferred)* | −360 … 360 *(sourced)* | Color: channels map to CMYK (C/M/Y/K) |
| Screen Angle Channel 3 | slider ° | 90 *(inferred)* | −360 … 360 *(sourced)* | |
| Screen Angle Channel 4 | slider ° | 45 *(inferred)* | −360 … 360 *(sourced)* | |
| Defaults | button | — | — | Returns all screen angles to defaults *(sourced)* |

### Crystallize
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Cell Size | slider int (px) | 10 *(inferred)* | 3–300 *(inferred; AliveColors reports 5–300)* | Higher = larger polygons |

### Facet
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| — | — | — | — | No controls |

### Fragment
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| — | — | — | — | No controls |

### Mezzotint
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Type | enum | Fine Dots *(inferred)* | Fine Dots / Medium Dots / Grainy Dots / Coarse Dots / Short Lines / Medium Lines / Long Lines / Short Strokes / Medium Strokes / Long Strokes *(inferred)* | Dot/line/stroke pattern; exact enumeration unverified |

### Mosaic
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Cell Size | slider int (px) | 10 *(inferred)* | 2–200 *(inferred)* | Square block size |

### Pointillize
| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Cell Size | slider int (px) | 5 *(inferred)* | 3–300 *(inferred)* | Random dot size |

`(inferred)` defaults and ranges come from the CS6 dialog and community documentation, not the fetched CS6 PDF; read them back from a CS6 build (see `## Open questions`).

## Algorithms & pipeline

Pixelate kernels are closed where they are artistic; several have well-known standard formulations. All are **behavioral parity only** unless noted. Because Pixelate is not a gallery stack, each filter applies directly through the `FILT-001` pipeline (target resolution → depth/mode gate → apron → backend → tiles → selection composite → single undo commit).

- **Color Halftone.** Standard halftoning per channel: for each channel, divide the image into a **rotated grid of cells** at an angle (the screen angle) with spacing proportional to the radius, and replace each cell with a **dot whose radius is proportional to the cell's brightness**; combine channels. Grayscale uses channel 1; color uses the four channels corresponding to C/M/Y/K angles. The **Defaults** button restores the standard angle set. *(Standard algorithm family: halftoning/screening; Adobe's exact cell kernel is closed.)*
- **Crystallize.** Standard **Voronoi tessellation** ("cellular" pattern): scatter seed points across the image at the given **cell size**, assign each pixel to its nearest seed, and fill each cell with the **average color** of the pixels it contains. *(Standard algorithm.)*
- **Facet.** Pixels of similar color are clumped into like-colored blocks. A behavioral model is **iterative block averaging/region-growing** (successive passes that replace each pixel with a local average when neighbors are similar), which reduces detail into flat patches; the exact Adobe pass count/radius is closed and has no UI control.
- **Fragment.** Explicitly documented: create **four copies** of the selection pixels, **offset** them from one another, and **average**. The offset is a small fixed amount (sub-pixel-ish) and the operation is equivalent to a 4-tap average with a small displacement — a deterministic blur/ghost. No controls.
- **Mezzotint.** Replace each region with a **random pattern** chosen from a Type list of dot/line/stroke textures; a color image gets fully saturated color patterns. Implement as a per-type procedural pattern over the image with a **seeded RNG** (store the seed for reproducible redo).
- **Mosaic.** Standard **block averaging**: partition into `cell_size × cell_size` square blocks and set every pixel in a block to the block's **average color** (equivalently a box downsample–upsample). *(Standard algorithm; distinct from `FILT-083` Mosaic Tiles.)*
- **Pointillize.** Scatter random **dots** of radius proportional to `cell_size` across the image, each filled with the local/averaged source color, leaving the **background color** as the canvas between dots. Use a **seeded RNG** for reproducible dot placement.

**Determinism.** Mezzotint and Pointillize are random; use a seeded RNG stored in the history/effect record so redo and PSD round-trips reproduce results.

## Rust module mapping

Proposed under `pictura-filters::pixelate` (`ARCH-002`), implementing the shared `Filter` trait (`FILT-001`):

- `pictura-filters::pixelate::color_halftone` — `ColorHalftone { max_radius: u8, angles: [f32; 4] }`; per-channel rotated screening.
- `pictura-filters::pixelate::crystallize` — `Crystallize { cell_size: u16 }`; Voronoi tessellation + cell mean.
- `pictura-filters::pixelate::facet` — `Facet` (unit struct); iterative block averaging.
- `pictura-filters::pixelate::fragment` — `Fragment` (unit struct); 4-tap offset average.
- `pictura-filters::pixelate::mezzotint` — `Mezzotint { pattern: MezzotintType }`; `enum MezzotintType { … }`; seeded RNG.
- `pictura-filters::pixelate::mosaic` — `Mosaic { cell_size: u16 }`; block mean.
- `pictura-filters::pixelate::pointillize` — `Pointillize { cell_size: u16 }`; seeded RNG.
- Shared helpers: `pictura-filters::kernel::tessellate` (Voronoi), `pictura-filters::kernel::halftone`, `pictura-filters::kernel::block_mean`, `pictura-filters::kernel::noise::SeededRng`.

Crossing types: `Rect`, `TileView`, `Rgba` (background), `MezzotintType`, `Seed(u64)`. No Qt types.

## Qt6 component mapping

Widgets, matching the CS6 modal dialogs; pixel work in Rust. Each Pixelate filter is a standalone modal dialog, not a gallery pane.

| Proposal | Base | Responsibility |
|---|---|---|
| `PixelateDialogBase` | `QDialog` | Shared preview + OK/Cancel + `FilterSliderSpin` layout (`FILT-001` `FilterDialogBase`) |
| `ColorHalftoneDialog` | `QDialog` | Max Radius spin, four angle spins, **Defaults** button, preview |
| `CrystallizeDialog` | `QDialog` | Cell Size spin + preview |
| `MezzotintDialog` | `QDialog` | Type combo + preview |
| `MosaicDialog` | `QDialog` | Cell Size spin + preview |
| `PointillizeDialog` | `QDialog` | Cell Size spin + preview |
| `Facet` / `Fragment` | — | No dialog; apply immediately from the menu |

`FilterDialogBase` (`FILT-001`) supplies the preview, zoom, and button box; the concrete dialogs add only their controls.

## Data-model impact

- **Destructive apply:** no persistent field; one `HistoryRecord::FilterOp { filter_id, roi, params_blob, seed, before_tiles, after_hash }` per committed apply (`ARCH-009`); Mezzotint/Pointillize store the `seed`.
- **Smart Object:** each Pixelate filter becomes an individual Smart Filter entry `{ filter_id, params, blend, opacity, enabled }` (`LAY-021`) — **not** a "Filter Gallery" group, because these filters are not gallery filters.
- **Serialization:** descriptors are written through the Smart Filter/Filter Effects path (`LAY-021`); per-parameter `FXid`/`FEid` key mapping is **unsourced**.
- Undo granularity is one apply.

## Edge cases

- **Mode:** Pixelate filters act on 8-bit RGB/Grayscale and — per Adobe/PSE Help for Color Halftone — support multipass `Multichannel`/CMYK-style channel handling (`Color Halftone` treats color channels as CMYK). Per-filter CMYK/Lab support must be capability-gated (never convert).
- **Depth:** 16-bit/32-bit are not in Adobe's filter depth lists → 8-bit only; disable/show warning at higher depth.
- **Degenerate cell sizes.** Mosaic `Cell Size 2`, Crystallize `Cell Size 3`, Pointillize `Cell Size 3`, and Color Halftone `Max Radius 4` are the minima; ensure the grid math never divides by zero or produces empty ROIs.
- **Selection handling.** Pixelate filters are documented as acting on the selection; a partial cell straddling the selection edge must be handled consistently (clamp or aggregate over available pixels) and pinned by tests.
- **1×1 / 1-px documents.** Block/Voronoi code must not panic on cells larger than the ROI.
- **Huge PSB:** tile-local; Mosaic/Crystallize need either full-image cell assignment (expensive) or a documented tiling strategy with seam handling; Pointillize/Mezzotint must be procedural per tile with a coordinate-derived seed.
- **Randomness:** Mezzotint/Pointillize seeded for reproducible redo; two identical applies match bit-for-bit.
- **Color Halftone channel count.** Grayscale uses one angle; a 4-channel color image uses four; other channel counts must degrade gracefully (mirror channel 1 or reuse angles).
- **Cancellation:** atomic no-op (`ARCH-003`).
- **GPU:** CPU fallback must be correct; no kernel requires a GPU (`ARCH-006`).
- **Empty/locked layer:** no-op or refuse per `FILT-001`.

## Parity acceptance criteria

1. Given an 8-bit RGB document, each of the 7 Pixelate filters appears under `Filter > Pixelate`, opens (or immediately applies for Facet/Fragment), produces a non-empty effect, and adds exactly one history state.
2. Given a 16-bit/32-bit document, Pixelate filters are disabled or show the Smart Filter warning; applying is refused with no conversion.
3. Given `Color Halftone`, the output is a set of per-channel dot screens; increasing Max Radius enlarges the dots; **Defaults** restores the standard screen angles; a grayscale image responds only to channel 1.
4. Given `Crystallize`, increasing Cell Size produces larger, fewer polygons; each polygon is filled with its region's average color (compare to a Voronoi reference within tolerance).
5. Given `Mosaic`, increasing Cell Size produces larger square blocks; every pixel in a block equals the block mean; `Filter > Texture > Mosaic Tiles` is observably different (grout between chips).
6. Given `Fragment`, the result equals the average of the four **offset copies** of the source within tolerance; on a flat-color region it is a no-op.
7. Given `Facet`, a gradual gradient is reduced to flat patches while edges between unlike colors are preserved.
8. Given `Mezzotint`, each Type produces a distinct dot/line/stroke pattern; results are reproducible from the stored seed.
9. Given `Pointillize`, dots of size proportional to Cell Size are scattered and the background color shows between them; results are reproducible from the stored seed.
10. Given a Smart Object, applying a Pixelate filter adds an individual Smart Filter entry (not a "Filter Gallery" group), leaves the pixels unchanged, and reopening restores saved parameters.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — Adobe Photoshop CS6 Help reference (downloaded to `/tmp` and text-extracted with `pdftotext`). Established: the Pixelate family introduction (clumping similar-valued pixels into cells); the one-line behavior of all 7 filters, including **Fragment as four offset, averaged copies** and **Mezzotint's Type menu**; the Mosaic vs Mosaic Tiles distinction; and that Pixelate is **not** among the Filter Gallery's categories (the gallery is Artistic/Brush Stroke/Distort/Sketch/Stylize/Texture). Primary source.
- `https://web.archive.org/web/20220813051258id_/https://helpx.adobe.com/photoshop-elements/using/pixelate-filters.html` — Photoshop Elements Help, "Pixelate filters" (Adobe content, archived). Established: Color Halftone **max-radius range 4–127 px**, **screen angles −360…360** entered per channel (grayscale = channel 1; color = channels 1–4 mapped to CMYK), and a **Defaults** button; Crystallize (cell size), Facet (no controls), Fragment (offset/blurred, no controls), Mezzotint (dot/line/stroke pattern), Mosaic (cell size), Pointillize (cell size).
- `https://alivecolors.com/en/tutorial/effects/pixelate.php` — AliveColors "Pixelate" reference (independent Photoshop-compatible editor). Corroborated **Color Halftone Radius 4–127** and per-channel angle **−360…360**, and **Crystallize size 5–300** (Photoshop's dialog is commonly cited as 3–300 — see Open questions). Third-party.
- `https://web.archive.org/web/20220628182932id_/https://www.pcworld.com/article/393412/how-photoshop-artistic-filters-work-with-examples.html` — PCWorld established that the Filter Gallery categories are exactly **Artistic, Brush Strokes, Distort, Sketch, Stylize, and Texture** (i.e. Pixelate is not a gallery category). Community source.

Searches performed (not documents fetched): DuckDuckGo/Brave for Pixelate ranges/defaults; unconfirmed values are marked *(inferred)*.

Not used: `helpx.adobe.com` live pages return HTTP 403; archived copies were used instead.

## Open questions

- **Pixelate and the Filter Gallery.** This spec asserts Pixelate is **not** a Filter Gallery category in Photoshop CS6, based on the gallery's six-category roster. Confirm on a CS6 build whether any CS6 update added a Pixelate gallery category (Elements behavior differs). If it did, gallery stacking would apply.
- **Crystallize maximum.** AliveColors reports 5–300; Photoshop is commonly cited as 3–300. Confirm the CS6 minimum.
- **All defaults.** Adobe does not publish Pixelate defaults; every "Default" above is *(inferred)*. Resolve by a scripted CS6 dialog read.
- **Mezzotint Type enumeration.** The exact 10 Type labels and their order are *(inferred)* from the dialog; confirm against CS6.
- **Color Halftone default angles.** The default screen angles (commonly 108/162/90/45) are *(inferred)*; the **Defaults** button behavior should be read back from CS6.
- **Facet algorithm.** The number of averaging passes/radius is undocumented and has no control; behavioral parity only. Resolve with synthetic-gradient probes.
- **Fragment offset amount.** The exact pixel offset of the four copies is undocumented; resolve by matching a reference render on a fine pattern.
- **`FXid`/`FEid` parameter mapping.** See `LAY-021`; the per-filter parameter descriptor keys are unsourced.
