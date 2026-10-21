# Stylize Filters

- **Spec ID:** `FILT-050`
- **Status:** `Draft`
- **Parity tier:** `Core` (Emboss, Find Edges, Solarize, Trace Contour, Wind, Tiles, Extrude) + `Extended-only`/`Core` per GPU for **Oil Paint**; **Diffuse** and **Glowing Edges** are Filter-Gallery effects
- **New in CS6:** `Yes` — **Oil Paint** (`Filter > Oil Paint`) is new in CS6 and is a GPU/Mercury filter. The other Stylize filters are unchanged from CS5.
- **Depends on:** `06-filters/filters-overview.md`, `06-filters/oil-paint.md`, `06-filters/blur-filters.md`, `06-filters/sketch-filters.md`, `04-image-ops/bit-depth-and-conversion.md`, `01-architecture/gpu-rendering-pipeline.md`, `01-architecture/document-model.md`, `05-layers/smart-filters.md`, `05-layers/blend-modes.md`

> All crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Behavior is from the fetched CS6 Help unless marked
> *(inferred)*. Adobe's exact edge detector, threshold curve, extrusion renderer,
> and Oil Paint brush model are closed and are marked **behavioral parity only,
> algorithm TBD**.

## CS6 behavior

`Filter > Stylize` gives a selection a painted or impressionistic look by displacing pixels and by finding and intensifying contrast. Help notes that after Find Edges and Trace Contour you can run Invert to get colored/white outlines. The submenu in CS6:

| Filter | Menu path | What it does (CS6 Help) |
|---|---|---|
| Diffuse | `Filter > Stylize > Diffuse` | Shuffles pixels to soften focus: Normal (random, ignores color), Darken Only (light→dark), Lighten Only (dark→light), Anisotropic (shuffles along the direction of least color change). |
| Emboss | `Filter > Stylize > Emboss` | Makes a selection look raised/stamped: converts fill to gray and traces edges with the original fill color. Options: angle, height, amount. |
| Extrude | `Filter > Stylize > Extrude` | Gives a 3D texture; blocks (square front + four sides) or pyramids (four triangular sides to a point). |
| Find Edges | `Filter > Stylize > Find Edges` | Emphasizes significant transitions; dark lines on white. |
| Glowing Edges | `Filter > Stylize > Glowing Edges` | Finds color edges and adds a neon-like glow; **can be applied cumulatively**. |
| Solarize | `Filter > Stylize > Solarize` | Blends a negative and a positive image (like brief exposure of a print during development). |
| Tiles | `Filter > Stylize > Tiles` | Breaks the image into tiles offset from the original; fills gaps with background, foreground, a reverse image, or an unaltered image. |
| Trace Contour | `Filter > Stylize > Trace Contour` | Finds major brightness transitions and thinly outlines them per channel, like a contour map. |
| Wind | `Filter > Stylize > Wind` | Places tiny horizontal lines for a windblown effect: Wind, Blast (stronger), Stagger (offset lines). |
| Oil Paint | `Filter > Oil Paint` | (CS6-new) Gives an image a classic-painting look; brush and lighting options. Requires a supported GPU. Separate spec: `06-filters/oil-paint.md`. |

**Halftone Pattern is not Stylize.** Halftone Pattern, which simulates a halftone screen while keeping a continuous tonal range, is listed under **Sketch** in CS6 Help and belongs in `06-filters/sketch-filters.md`. It is recorded here only to prevent mis-categorization.

**Color inside Emboss.** Help recommends `Edit > Fade` after Emboss to retain color and detail, because Emboss replaces the fill with gray and you often want the original color back. This is the standard Fade mechanism (`LAY-021`, `05-layers/blend-modes.md`).

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Filter > Stylize > Diffuse` | Menu / modal dialog | — | Mode enum only |
| `Filter > Stylize > Emboss` | Menu / modal dialog | — | Angle/Height/Amount; Fade often used after |
| `Filter > Stylize > Extrude` | Menu / modal dialog | — | Type/Size/Depth/options |
| `Filter > Stylize > Find Edges` | Menu | — | Applies immediately (no dialog) |
| `Filter > Stylize > Glowing Edges` | Menu / modal dialog | — | Edge Width/Brightness/Smoothness |
| `Filter > Stylize > Solarize` | Menu | — | Applies immediately (no dialog) |
| `Filter > Stylize > Tiles` | Menu / modal dialog | — | Number/Maximum Offset/Fill |
| `Filter > Stylize > Trace Contour` | Menu / modal dialog | — | Edge + Level |
| `Filter > Stylize > Wind` | Menu / modal dialog | — | Method + Direction |
| `Filter > Oil Paint` | Menu / modal dialog | — | CS6-new GPU filter |
| `Filter > Filter Gallery > Stylize > …` | Gallery | — | Includes Diffuse, Emboss, Extrude, Find Edges, Glowing Edges, Solarize, Tiles, Trace Contour, Wind |
| `Edit > Fade` | Menu | `Ctrl+Shift+F`/`Cmd+Shift+F` | Post-filter blend, commonly used after Emboss |
| Smart Filter row | Layers panel | — | Oil Paint is Smart-Object capable; gallery filters appear as one "Filter Gallery" entry |

## Parameters & ranges

Ranges marked **[Help]** are stated in the CS6 Help text. Ranges marked *(inferred)* are the dialog limits (not stated in Help) and must be confirmed against a CS6 build. Adobe's AppleScript reference does **not** expose Emboss/Extrude/Glowing Edges/Tiles/Wind as scriptable filter classes, so those ranges cannot be sourced from it.

| Filter | Control | Type | Default | Range / options | Source |
|---|---|---|---|---|---|
| Diffuse | Mode | enum | Normal | Normal / Darken Only / Lighten Only / Anisotropic | [Help] |
| Emboss | Angle | int ° | 135 *(inferred)* | −360 to +360 (negative recesses, positive raises) | [Help] |
| Emboss | Height | int px | 3 *(inferred)* | 1–100 *(inferred)* | [Help] |
| Emboss | Amount | int % | 100 *(inferred)* | 1–500% | [Help] |
| Extrude | Type | enum | Blocks *(inferred)* | Blocks / Pyramids | [Help] |
| Extrude | Size | int px | 30 *(inferred)* | 2–255 (length of a base side) | [Help] |
| Extrude | Depth | int | 30 *(inferred)* | 1–255 (how far the tallest object protrudes; Solid Front Faces only changes the front-face fill) | [Help] |
| Extrude | Depth option | enum | Level-based *(inferred)* | Random / Level-based (brightness→protrusion) | [Help] |
| Extrude | Solid Front Faces | bool | off | Blocks only; fill the front face with the block's average color | [Help] |
| Extrude | Mask Incomplete Blocks | bool | off | Hide objects extending beyond the selection | [Help] |
| Find Edges | — | — | — | No options | [Help] |
| Glowing Edges | Edge Width | int px | 2 *(inferred)* | 1–14 *(inferred)* | Computer Hope (params), range *(inferred)* |
| Glowing Edges | Edge Brightness | int | 6 *(inferred)* | 0–20 *(inferred)* | Computer Hope (params), range *(inferred)* |
| Glowing Edges | Smoothness | int | 1 *(inferred)* | 1–15 *(inferred)* | Computer Hope (params), range *(inferred)* |
| Solarize | — | — | — | No options (fixed threshold curve) | [Help] |
| Tiles | Number of Tiles | int | 10 *(inferred)* | 1–99 *(inferred)* | Help lists the control, ranges *(inferred)* |
| Tiles | Maximum Offset | int % | 10 *(inferred)* | 1–99% *(inferred)* | Help lists the control, ranges *(inferred)* |
| Tiles | Fill Empty Area With | enum | Background *(inferred)* | Background / Foreground / Inverse Image / Unaltered Image | [Help] |
| Trace Contour | Edge | enum | Lower *(inferred)* | Lower / Upper | [Help] |
| Trace Contour | Level | int | 128 *(inferred)* | 0–255 | [Help] |
| Wind | Method | enum | Wind | Wind / Blast / Stagger | [Help] |
| Wind | Direction | enum | From the Right *(inferred)* | From the Right / From the Left | [Help] |
| Oil Paint | see `06-filters/oil-paint.md` | — | — | Brush: Stylization, Cleanliness, Scale, Bristle Detail; Lighting: Angle, Shine *(inferred ranges)* | [Help] only lists "Brush and Lighting options" |

**Bit-depth gate (Help "Filter basics").** The 16-bit capable list includes **Emboss, Find Edges, and Solarize**. The 32-bit capable list includes **Emboss**. Diffuse, Extrude, Glowing Edges, Tiles, Trace Contour, and Wind are not listed for 16/32-bit (8-bit only); Oil Paint is a GPU filter with its own support rules. (Sourced.)

## Algorithms & pipeline

**Behavioral parity only, algorithm TBD** where Adobe does not document the kernel.

| Filter | Algorithm family | Notes |
|---|---|---|
| Diffuse | Stochastic / edge-directed pixel shuffle | Normal = random permutation ignoring color; Darken Only = choose darker neighbor; Lighten Only = choose lighter; Anisotropic = shuffle along the direction of least color change (edge-tangent). *(inferred)* |
| Emboss | Directional gradient convolution + gray offset | Grey raised appearance: highlight one side, shadow the opposite side along the angle; the "amount" mixes the original color back in (trace the edges with fill color). Fade afterwards is the supported way to retain color. *(inferred)* |
| Extrude | Height-field extrusion rendering | Depth from brightness (Level-based) or per-object random; render block front + four side faces or pyramid faces; Solid Front Faces fills with the average block color; Mask Incomplete Blocks clips objects crossing the selection edge. *(inferred)* |
| Find Edges | Gradient magnitude | Compute a first-derivative edge response (Sobel/Prewitt-class) per channel, output dark edges on white. Adobe's exact kernel is closed. *(inferred)* |
| Glowing Edges | Edge detection + neon colorization | Detect color edges, colorize with a neon palette on black, blur for glow; Help notes it is cumulative, so repeated application compounds. *(inferred)* |
| Solarize | Fixed tonal inversion curve | Identity up to a threshold, inverted above it (classic solarization). No user controls. Adobe's threshold is closed; behavioral parity only. |
| Tiles | Tiled offset | Number of Tiles sets the super-tile count; Maximum Offset is an upper bound on the random offset; gaps filled per the chosen fill rule. *(inferred)* |
| Trace Contour | Per-channel threshold contour | For each channel, outline where the value crosses Level in the chosen direction (Upper/Lower); thin single-pixel contours. *[Help]* |
| Wind | Horizontal streak displacement | Copy/extend pixels along a horizontal direction in thin streaks; Blast widens/lengthens, Stagger offsets the lines. *(inferred)* |
| Oil Paint | Edge-preserving stylized stroke + lighting | Belongs to the anisotropic Kuwahara / edge-aware smoothing family, with a lighting pass (angle + shine) for glaze highlights. GPU-implemented in CS6 (Mercury). Separate spec: `06-filters/oil-paint.md`. |

**Pipeline.** Emboss, Find Edges, Glowing Edges, Trace Contour, and Oil Paint are neighborhood operatives (tile halo = kernel radius); Extrude and Tiles are resampling/reprojection filters; Diffuse and Wind are local displacement filters. A shared `NeighborhoodFilter` with an explicit halo size lets the tile scheduler (`gpu-rendering-pipeline`) give every Stylize filter the right overlap.

## Rust module mapping

- `pictura_filter::stylize` — one submodule per filter, all implementing `Filter`.
- `pictura_filter::neighbor::Convolution` — shared NxN/5×5 convolution for Emboss and Find Edges; `Kernel` is a `[[i32; 5]; 5]` plus divisor and bias (also reused by Custom, `FILT-070`).
- `pictura_filter::stylize::extrude` — `ExtrudeKind::{Blocks, Pyramids}`, height field from luma or RNG, front-face fill from block average.
- `pictura_filter::stylize::tiles` — tile layout + `Fill::Background|Foreground|Inverse|Unaltered` needing fg/bg color and the source tile.
- `pictura_filter::stylize::trace_contour` — per-channel threshold pass; outputs a 1-bit mask per channel.
- `pictura_filter::stylize::oil_paint` — thin wrapper delegating to `pictura_filter::oil_paint`, or a `Filter` adapter around that engine.
- `pictura_filter::registry` — maps filter id + CS6 four-char event id (`'Dfs '`, `'Embs'`, `'Extr'`, `'FndE'`, `'GlwE'`, `'Slrz'`, `'Tls '`, `'TrcC'`, `'Wnd '`) to implementation + `supported(mode, depth)`.

Crossing types: `Tile`, `Rect`, `Kernel`, `ColorMode`, `BitDepth`, `Rgb`, `FilterParams`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `StylizeOptionsDialog` | `QDialog` | Per-filter parameter forms in a `QStackedWidget` |
| `ExtrudeOptionsPanel` | `QWidget` | Type combo, Size/Depth spins, Random/Level-based, two checkboxes |
| `TraceContourPanel` | `QWidget` | Edge combo + Level spin 0–255 |
| `EmbossOptionsPanel` | `QWidget` | Angle dial, Height/Amount spins; hint to use Fade |
| `WindOptionsPanel` | `QWidget` | Method + Direction combos |
| `TilesOptionsPanel` | `QWidget` | Number/Offset spins + Fill combo |
| `GlowingEdgesPanel` | `QWidget` | Width/Brightness/Smoothness sliders |
| `OilPaintPanel` | `QWidget` | Embedded from `06-filters/oil-paint.md` |
| `FilterPreviewPane` | `QGraphicsView` | Shared preview (zoom + click/drag to center) |
| `FilterMenuBuilder` | helper | Greys 8-bit-only entries on 16/32-bit documents |

Widgets for the dialogs; the shared preview may reuse the canvas widget. Oil Paint's sliders are integrated, not re-implemented.

## Data-model impact

- Each Stylize filter is a typed `FilterParams` variant recorded once per apply; undo recomputes from parameters (no pixel backup).
- **Find Edges and Solarize** apply immediately with no dialog, but still create a distinct undo history state and, on a Smart Object, still appear as an editable Smart Filter (Smart Filters re-expose the deterministic algorithm; Find Edges/Solarize have no parameters).
- **Diffuse / Emboss / Extrude / Glowing Edges / Tiles / Trace Contour / Wind** when applied via the Filter Gallery appear as one grouped "Filter Gallery" Smart Filter entry whose sub-filters are editable individually (`LAY-021`).
- **Oil Paint** is a single Smart Filter entry with its own parameter descriptor (`06-filters/oil-paint.md`).
- Serialization into PSD: no Stylize-specific additional-layer keys are documented; parameters live in the Smart Filter record. Fade is stored as the filter's blend/opacity (`LAY-021`).

## Edge cases

- **8/16/32-bit.** Most Stylize filters are 8-bit only. Emboss runs at 16 and 32-bit; Find Edges and Solarize run at 16-bit; Emboss is the only one in the 32-bit list. Enforce `supported(mode, depth)` and grey the menu.
- **No-dialog filters.** Find Edges and Solarize still need a history entry and Smart Filter entry.
- **Emboss color loss.** Gray fill replaces color; parity requires the documented Fade workflow to behave the same (blend back the original with the chosen mode/opacity).
- **Glowing Edges cumulative.** Repeated application compounds because it detects edges on the already-glowing image; tests must not assume idempotence.
- **Trace Contour on a flat image.** No crossings ⇒ empty output, not black.
- **Tiles at edges.** Inverse/Unaltered options need the pre-filter source; cache it so the fill doesn't read already-tiled pixels.
- **Extrude at the selection boundary.** Mask Incomplete Blocks clips partial objects; Level-based depth depends on the bit depth's luma range.
- **Solarize in 8/16-bit vs 32-bit.** The threshold curve must be defined in the working space; 32-bit is unsupported by Help's list.
- **Oil Paint GPU.** Without a supported GPU the filter is unavailable (greyed), matching CS6.
- **CMYK/Lab.** Filter Gallery filters are 8-bit RGB/gray only in practice; gate accordingly.
- **Huge/PSB documents.** Extrude and Tiles allocate block/tile metadata proportional to area; stream by tile.
- **Undo/redo.** Parameters alone are enough except Extrude/Tiles, which use no randomness except Extrude's Random depth (store the RNG seed).

## Parity acceptance criteria

- Given Diffuse Normal, output pixels are a permutation of the neighborhood values; Diffuse Darken Only never yields a value lighter than the neighborhood maximum.
- Given Emboss Angle A and its negative, the highlight and shadow sides swap.
- Given Emboss followed by `Edit > Fade` at 100% Normal, the original color is restored to the embossed shape within tolerance.
- Given Extrude Depth = 1, the protruding height is minimal; Depth = 255 is maximal; Level-based depth increases monotonically with source brightness.
- Given Extrude Solid Front Faces on, each block's front face is a single flat average color.
- Given Find Edges on a constant image, the output is uniform (no edges).
- Given Glowing Edges applied twice, the second result differs from the first (cumulative).
- Given Solarize applied twice, the result differs from one application (it is not an involution at 8-bit rounding).
- Given Tiles with Fill = Unaltered Image, the gaps reveal the original pre-filter pixels.
- Given Trace Contour Level L, pixels adjacent to a crossing of L are outlined; shifting L moves the contours.
- Given Wind Method = Blast, the streak length exceeds Method = Wind at the same image.
- Given a 16-bit document, Emboss/Find Edges/Solarize run and the other Stylize entries are disabled; on 32-bit only Emboss runs.
- Given a Smart Object with any Stylize Smart Filter, editing and re-rendering reproduces the same result; a gallery-applied stack appears as one "Filter Gallery" entry.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — Photoshop CS6 Help. Established: the Stylize submenu and each filter's description; Diffuse modes; Emboss angle/height/amount and the Fade recommendation; Extrude "Apply the Extrude filter" (Blocks/Pyramids, Size 2–255, Depth 1–255, Random/Level-based, Solid Front Faces, Mask Incomplete Blocks); Trace Contour "Apply the Trace Contour filter" (Lower/Upper, Level 0–255); Tiles fill options; Wind methods; Solarize/Find Edges behavior; the Halftone Pattern entry under Sketch; the 16-bit list containing Emboss/Find Edges/Solarize and the 32-bit list containing Emboss; the Oil Paint CS6 What's-New note and the "Brush and Lighting options" text; Filter Gallery rules and cumulative application.
- `https://applescriptlibrary.files.wordpress.com/2013/11/photoshop-cs6-applescript-reference.pdf` — Photoshop CS6 AppleScript Scripting Reference. Established the four-char event IDs for Diffuse (`'Dfs '`), Emboss (`'Embs'`), Extrude (`'Extr'`), Find Edges (`'FndE'`), Glowing Edges (`'GlwE'`), Solarize (`'Slrz'`), Tiles (`'Tls '`), Trace Contour (`'TrcC'`), Wind (`'Wnd '`), and that these filters are not exposed as scriptable filter-options classes (so no sourced ranges).
- `https://www.computerhope.com/jargon/p/photoshop-glowing-edges.htm` — established the Glowing Edges parameter names (edge width, edge brightness, smoothness) and that it is a Filter-Gallery 8-bit filter; used only for control identity (ranges marked inferred).
- `https://web.archive.org/web/2014id_/https://helpx.adobe.com/photoshop/using/filter-effects-reference.html` — archived Adobe reference; corroborated the Stylize prose.

Not used in this pass:

- `https://helpx.adobe.com/photoshop/using/oil-paint-filter.html` — the live page and an AWS mirror both timed out during this pass; Oil Paint ranges are inferred and deferred to `06-filters/oil-paint.md`.

## Open questions

- **Emboss height/amount ranges and defaults.** Help states the angle range and the percent ceiling but the height range and all defaults are inferred. Resolve from a CS6 UI capture.
- **Glowing Edges ranges and defaults.** Parameter names are sourced; ranges/defaults are inferred. Resolve from a CS6 UI capture.
- **Tiles ranges/defaults.** Control names sourced; ranges inferred. Resolve from a CS6 UI capture.
- **Solarize threshold.** Whether CS6's Solarize uses a fixed 50% threshold curve or a different closed curve is unconfirmed. Resolve by fitting reference renders.
- **Find Edges kernel.** Adobe's edge operator (Sobel vs. other) and per-channel handling are undocumented. Resolve by fitting.
- **Extrude renderer.** The exact shading of side faces, the Level-based depth mapping, and the block-average color space are inferred. Resolve by fitting.
- **Anisotropic Diffuse.** The precise "least color change" direction metric and neighborhood size are undocumented. Resolve by fitting.
- **Oil Paint ranges.** Stylization/Cleanliness/Scale/Bristle Detail/Angle/Shine ranges and defaults are inferred pending the Oil Paint spec.
- **Wind "Blast" geometry.** The exact streak length/spread model is inferred.
