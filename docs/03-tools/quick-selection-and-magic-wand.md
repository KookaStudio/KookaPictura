# Quick Selection & Magic Wand

- **Spec ID:** `TOOL-004`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `No` — Quick Selection (CS3) and Magic Wand (long-standing) are unchanged in CS6; the shared Refine Edge machinery carries over from CS5.
- **Depends on:** `ARCH-002` document-model, `ARCH-006` gpu-rendering-pipeline, `ARCH-007` undo-history, `TOOL-002`/`TOOL-003` (sibling selection tools), `08-selection/selection-model.md`, `08-selection/refine-edge.md`, `07-color-painting/brush-engine.md`.

> All crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not.

## CS6 behavior

Two colour/region-based selection tools share the `W` shortcut:

- **Quick Selection** — "paints" a selection with an adjustable round brush. As the user drags, the selection grows outward and automatically finds and follows defined edges. Selection options are **New**, **Add To**, and **Subtract From** (there is no Intersect mode for Quick Selection). After the first selection, the mode auto-switches from New to Add. Brush tip size is set in the options-bar Brush pop-up, can be sensitive to pen pressure or a stylus wheel, and changes with `]` / `[`. `Alt`(`Option`) temporarily toggles between add and subtract. Options: **Sample All Layers** (build the selection from all layers rather than the active layer) and **Auto-Enhance** (reduces roughness and blockiness in the boundary, flowing the selection further toward image edges and applying some of the edge refinement available manually in Refine Edge via Contrast and Radius). Clicking `Refine Edge` refines the result.
- **Magic Wand** — selects a consistently coloured area by clicking it. Selection options are **New**, **Add To**, **Subtract From**, and **Intersect With** (the pointer changes with the mode). Options: **Tolerance** (0–255; low selects only very similar colours, high selects a broader range), **Anti-aliased** (smoother-edged selection), **Contiguous** (select only adjacent pixels of the same colour; otherwise every matching pixel in the image is selected), and **Sample All Layers** (sample all visible layers instead of the active layer only). The Magic Wand **cannot be used on Bitmap-mode or 32-bits-per-channel images**. Clicking `Refine Edge` refines the result.

Both tools are documented in the CS6 Help under "Making quick selections". Because they are selection tools, they participate in the shared mode/feather/anti-alias/Refine Edge conventions (`Select > Refine Edge`, `Ctrl+Alt+R`).

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Tools panel | Tool | `W` | Default `W` tool is context-dependent (the two share the slot) |
| Tools panel | Hidden tools | `Shift+W` | Cycles Quick Selection ↔ Magic Wand |
| Quick Selection options bar | Buttons | — | New / Add To / Subtract From (no Intersect) |
| Quick Selection options bar | Brush pop-up | `]` / `[` | Size (px), pressure/stylus-wheel sensitivity |
| Quick Selection options bar | Checkbox | — | Sample All Layers |
| Quick Selection options bar | Checkbox | — | Auto-Enhance |
| Quick Selection options bar | Button | `Ctrl+Alt+R` | Refine Edge |
| Magic Wand options bar | Buttons | — | New / Add To / Subtract From / Intersect With |
| Magic Wand options bar | Spin box | — | Tolerance (0–255) |
| Magic Wand options bar | Checkbox | — | Anti-aliased |
| Magic Wand options bar | Checkbox | — | Contiguous |
| Magic Wand options bar | Checkbox | — | Sample All Layers |
| Magic Wand options bar | Button | `Ctrl+Alt+R` | Refine Edge |
| Canvas | Modifier | `Alt`(`Option`) | Quick Selection: temporary add/subtract toggle |
| Canvas | Modifier | `Shift`/`Alt`/`Shift+Alt` | Magic Wand: add/subtract/intersect (generic selection modifier) |
| `Edit > Preferences > Cursors > Painting Cursors` | Preference | — | Quick Selection cursor (Normal/Full-Size brush tip, crosshair) |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Quick Selection mode | enum | New (auto-switches to Add) | New / Add / Subtract | No Intersect |
| Brush Size | int (px) | ~30 | 1–up | Community-reported default; `[`/`]` stepping |
| Brush Hardness | int (%) | 100% | 0–100 | Brush-engine option |
| Brush Spacing | int (%) | 25% | 1–1000 | Brush-engine option (Quick Selection reuses the brush engine) |
| Size pressure/stylus wheel | bool | off | on / off | Corresponding Dynamics toggles |
| Sample All Layers | bool | off | on / off | Quick Selection |
| Auto-Enhance | bool | on | on / off | Quick Selection; applies edge refinement |
| Magic Wand mode | enum | New | New / Add / Subtract / Intersect | Pointer changes per mode |
| Tolerance | int | 32 | 0–255 | 8-bit scale; scales with bit depth (see Open questions) |
| Anti-aliased | bool | on | on / off | Set before clicking |
| Contiguous | bool | on | on / off | Off = whole-image colour match |
| Sample All Layers | bool | off | on / off | Magic Wand |
| Refine Edge controls | various | see `08-selection/refine-edge.md` | — | Shared dialog |

## Algorithms & pipeline

### Selection representation

Both tools produce a coverage mask combined with the existing selection via the same `New`/`Add`/`Subtract`/`Intersect` algebra as `TOOL-002`, and support the same anti-alias/feather/Refine Edge post-processing. The difference is how the initial region is grown.

### Magic Wand

Magic Wand is a colour-similarity region selector:

```text
sample = colour at clicked pixel (active layer or all visible layers)
seed_colour = composite(sample)

# Contiguous on: 4-/8-connected flood fill from the seed
# Contiguous off: global mask over the whole image
for each pixel p (respecting Contiguous connectivity):
    if distance(p, seed_colour) <= tolerance_map(Tolerance):
        include p
```

Key unknowns that must be treated as *behavioral parity only*:
- **Distance metric.** Photoshop tolerance is commonly described as a per-channel difference in 8-bit levels (a tolerance of 32 accepts pixels within 32 levels). Whether it is max-channel, per-channel sum, Euclidean, or luminance-based is closed. Proposal: precompute a tolerance predicate and validate against CS6.
- **Anti-aliasing.** With anti-aliasing on, boundary pixels within tolerance but adjacent to included/excluded regions receive partial coverage rather than a binary include.
- **Sampling.** `Sample All Layers` selects colours from a flattened visible composite; otherwise from the active layer's pixels (including transparency).
- **Contiguous connectivity.** 4- vs 8-connected is undocumented; proposal is 4-connected with an anti-aliasing pass to match the smoother visual result.

`Select > Grow` and `Select > Similar` reuse the Magic Wand tolerance from the options bar per the CS6 Help, so that predicate should be a shared `ToleranceMap` service.

### Quick Selection

Quick Selection is a brush-driven, edge-aware region grow. The CS6 Help describes the observable result ("selection expands outward and automatically finds and follows defined edges", Auto-Enhance "flows the selection further toward image edges" and applies Contrast/Radius-like refinement). Proposed model:

```text
seed brush stroke -> sample colours/texture under the brush
region_grow(seed, features, edge_map):
    queue = pixels under the brush
    while queue:
        p = pop(); include p
        for n in neighbours(p):
            if !included[n] and similarity(n, region_stats) > threshold
               and !is_strong_edge(p, n, edge_map):     # stop at edges
                push n
    if auto_enhance:
        region = refine(region, contrast, radius)       # edge-flow + smooth
```

Design points:
- **Feature vector:** colour (in the document's working space) plus local gradient/edge confidence. Region statistics (mean/σ) update incrementally as pixels are added, so the grow adapts to gradual shading.
- **Edge stopping:** a strong gradient across a candidate boundary stops growth. This is what makes the selection "follow defined edges".
- **Add/subtract:** subtract runs the same grow with negative coverage and combines.
- **Auto-Enhance:** post-process with the Refine Edge `Contrast` + `Radius` operations (see `08-selection/refine-edge.md`) to smooth the boundary and pull it toward edges.
- **Performance:** run the grow at viewport/selection-tile resolution and refine at full resolution only along the boundary. The CS6 Help itself notes slow updates should be met by continuing to drag — i.e. the operation is expected to be interactive-but-not-instant on large images. Keep it off the GUI thread and show progressive results.

Exact Adobe similarity metric and thresholds are closed; mark *behavioral parity only, algorithm TBD*.

## Rust module mapping

- `pictura_selection::wand` — `MagicWandSettings { tolerance, anti_alias, contiguous, sample_all_layers }`; `magic_wand(doc, seed, settings) -> SelectionMask`.
- `pictura_selection::tolerance` — `ToleranceMap` / `within_tolerance(a, b, tolerance)`; shared with `Select > Grow`/`Similar`.
- `pictura_selection::quick` — `QuickSelectionEngine { settings }`; `paint_stroke(path, brush, mode) -> SelectionMask`; incremental region grow with edge stopping.
- `pictura_selection::flood` — connectivity flood fill (4/8), work-queue implementation over tiles.
- `pictura_selection::edge_stop` — gradient/edge lookup (shared `EdgeMap` from `TOOL-003`).
- `pictura_selection::refine` — shared edge refinement used by Auto-Enhance and Refine Edge.
- `pictura_tools::quick_selection` / `pictura_tools::magic_wand` — interaction state machines and options.
- `pictura_brush::BrushTip` — reused brush tip, size/hardness/spacing/dynamics (see `07-color-painting/brush-engine.md`).

Data crossing the boundary: `Point2` stroke samples, `MagicWandSettings`, `QuickSelectionSettings`, brush tip parameters, selection-mode enum, and mask tiles. Edge maps and region statistics stay in the core.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `QuickSelectionOptionsBar` | `QWidget` | Mode buttons, Brush pop-up, Sample All Layers, Auto-Enhance, Refine Edge |
| `MagicWandOptionsBar` | `QWidget` | Mode buttons, Tolerance spin box, Anti-aliased, Contiguous, Sample All Layers, Refine Edge |
| `BrushPresetPicker` | `QToolButton` + popup | Reused from the painting tools for size/hardness/spacing |
| `BrushCursorItem` | `QGraphicsItem` | Quick Selection round-brush cursor with +/- mode marker |
| `SelectionModeButtonGroup` | `QButtonGroup` | Shared with `TOOL-002`/`TOOL-003` (Quick Selection omits Intersect) |
| `RefineEdgeDialog` | `QDialog` | Shared with `08-selection/refine-edge.md` |
| `ToleranceSlider` | `QWidget` | Dense tolerance control reused by Grow/Similar |

Quick Selection reuses the brush preset picker from the painting domain; do not fork a second brush UI. Mode buttons and Refine Edge are shared controls across selection tools.

## Data-model impact

- One undo command per completed stroke (Quick Selection) or click (Magic Wand); intermediate growth previews are not pushed to history.
- Undo shape: `SelectionChange { before, after, region }` (bounding-box mask snapshot), as in `TOOL-002`.
- Tool options (Quick Selection brush tip, Auto-Enhance, Sample All Layers, Magic Wand tolerance/contiguous/AA/Sample All Layers) are tool/preset state, persisted in preferences/tool presets, **not** in PSD/XMP.
- `Sample All Layers` requires reading a flattened visible composite; cache it per document revision and invalidate on any layer/compositing change (`ARCH-002`, `ARCH-006`).
- Region statistics and edge maps are derived caches, never serialised.

## Edge cases

- **Magic Wand on Bitmap mode or 32-bpc:** explicitly unsupported in CS6 — disable with the same affordance and message.
- **Quick Selection on 32-bpc:** not explicitly stated in the CS6 Help; treat as an open question. Proposal: support it (feature-based, bit-depth-independent) unless a CS6 comparison proves otherwise.
- **Empty layer / fully transparent seed:** no colour to grow from; return an empty selection and do not error.
- **Sample All Layers with hidden layers:** sample only *visible* layers (CS6 wording: "all the visible layers" for Magic Wand; Quick Selection says "all layers").
- **Tolerance at 0/255:** 0 selects only the exact seed colour(s); 255 selects essentially everything within connectivity/AA constraints.
- **Contiguous off on a large image:** global scan is O(pixels); must be tiled and cancellable.
- **Anti-aliasing interaction with feather:** both soften the edge but differently; do not double-apply. Refine Edge overrides.
- **CMYK/Lab/16-bit:** tolerance is defined on the working-space channels; the 0–255 range is an 8-bit UI scale. Define the scaling rule and document it (see Open questions).
- **Interrupted grow:** a user clicking elsewhere mid-grow must merge or restart cleanly without corrupting the mask.
- **Huge PSB + GPU unavailable:** tiled CPU flood fill with a memory budget; overlay falls back to `QPainter`.
- **Undo/redo:** an in-progress Quick Selection stroke is discardable; only committed strokes enter history.

## Parity acceptance criteria

- Given a uniform red region on a white background, clicking inside with the Magic Wand at Tolerance 32 selects the red region and not the white background; at Tolerance 0 it selects only pixels exactly equal to the seed.
- Given `Contiguous` off, disconnected same-coloured patches are all selected by one click; with it on, only the connected patch is.
- Given `Anti-aliased` on, boundary coverage includes values strictly between 0 and 255; with it off, the mask is binary.
- Given `Sample All Layers` on, a colour visible only through a layer above the active layer is matched; with it off, it is not.
- Given a Magic Wand click on a Bitmap-mode or 32-bpc document, the tool is disabled/unavailable as in CS6.
- Given Quick Selection and a drag across a softly shaded object against a contrasting background, the grown boundary follows the object edge and does not bleed past a strong edge by more than ~2 px.
- Given Quick Selection with `Auto-Enhance` on versus off on the same stroke, the on-result has fewer boundary "hills and valleys" (smaller perimeter at equal area) while covering the same region within 1 px.
- Given the first Quick Selection stroke, the mode auto-switches from New to Add; `Alt`-drag temporarily subtracts.
- Given `Refine Edge` opened from either tool, the same dialog and controls appear as for `TOOL-002`/`TOOL-003`.
- Given `Select > Grow` after a Magic Wand selection, growth uses the same tolerance value shown in the Magic Wand options bar.
- Given an empty/transparent seed area, the click yields an empty selection without an error dialog.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — primary CS6 Help corpus (Feb 2013). Established: Quick Selection brush workflow, New/Add/Subtract modes and auto-switch to Add, `]`/`[` sizing, pressure/stylus-wheel sizing, Sample All Layers, Auto-Enhance (edge flow + Contrast/Radius refinement), `Alt` temporary mode toggle, cursor preference; Magic Wand Tolerance 0–255, Anti-aliased, Contiguous, Sample All Layers, New/Add/Subtract/Intersect, unavailability on Bitmap and 32-bpc images; `Select > Grow`/`Similar` using the Magic Wand tolerance; Refine Edge and its `Ctrl+Alt+R` shortcut; shared anti-alias/feather rules; tool shortcut table (`W`, Shift-cycle); selection modifier table (Intersect unavailable for Quick Selection).

Consulted as search-result snippets only (not individually fetched; community-reported):

- `https://www.adobe.com/.../select-by-color-magic-wand` and `https://www.photoshopessentials.com/basics/selections/magic-wand-tool` — Magic Wand default Tolerance 32.
- Multiple Quick Selection tutorials surfaced via SearXNG — brush size default ~30 px, hardness 100%, presence of a Spacing brush option.

Not used in this pass:

- `helpx.adobe.com` (HTTP 403).
- `https://www.scribd.com/...` and `glensmith.co.uk`/`expertphotography.com` — undecodable, paywalled, or binary payloads.

## Open questions

- **Magic Wand distance metric.** Whether tolerance is max-channel, per-channel sum, Euclidean, or luminance-based (and how it scales for 16/32-bit and CMYK/Lab) is undocumented. Resolve with a controlled tolerance-sweep comparison against a CS6 reference.
- **Contiguous connectivity.** 4- vs 8-connected is unspecified. Resolve with a single-pixel diagonal test in CS6.
- **Anti-aliasing algorithm.** How partial boundary coverage is computed is closed. Resolve with a boundary-coverage comparison.
- **Quick Selection similarity metric and thresholds.** The feature space (colour, texture, gradient), region-statistics update rule, and edge thresholds are closed. Resolve with a behavioral study / reference implementation.
- **Auto-Enhance parameters.** How strongly Auto-Enhance maps to Refine Edge Contrast/Radius is not stated. Resolve by comparing Auto-Enhance output against manual Refine Edge sweeps.
- **Quick Selection defaults.** Brush size (~30 px) and hardness/spacing defaults are community-reported; verify against a CS6 UI capture.
- **Quick Selection on 32-bpc.** CS6 explicitly blocks the Magic Wand but is silent for Quick Selection. Resolve with a CS6 test.
- **`Sample All Layers` and hidden layers.** Magic Wand wording says "visible layers"; Quick Selection wording says "all layers". Determine whether hidden layers are sampled. Resolve with a CS6 test.
- **Performance target.** The CS6 Help acknowledges slow Quick Selection updates on large images; set the interactive budget in `01-architecture/performance-targets.md`.
- **Feather profile** (shared with `TOOL-002`). Exact falloff unknown.
