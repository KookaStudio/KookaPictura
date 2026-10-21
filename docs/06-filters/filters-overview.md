# Filters Overview

- **Spec ID:** `FILT-001`
- **Status:** `Draft`
- **Parity tier:** `Core` — the built-in Filter menu, Filter Gallery, Smart Filters, Fade, and the Third-party filter slot are all available in CS6 Standard. The `3D` filter submenu is `Extended-only`; its filters are catalogued elsewhere.
- **New in CS6:** `Changed` — CS6 adds **Adaptive Wide Angle**, **Camera Raw Filter**, the **Blur Gallery** (Field Blur, Iris Blur, Tilt-Shift), and **Oil Paint**; reworks **Lighting Effects** as a 64-bit gallery; widens 32-bit filter support; and makes Blur Gallery / (later CC) Liquify Smart-Filter-capable. The Filter Gallery and Smart Filter mechanism themselves are carried over from CS5.
- **Depends on:** `06-filters/blur-filters.md`, `06-filters/sharpen-filters.md`, `06-filters/noise-filters.md`, `06-filters/blur-gallery.md`, `06-filters/liquify.md`, `06-filters/other-filters.md`, `LAY-021` smart-filters, `LAY-020` smart-objects, `LAY-010` blend-modes, `IMG-005` bit-depth-and-conversion, `ARCH-006` gpu-rendering-pipeline, `ARCH-009` undo-history, `ARCH-011` plugin-and-scripting-abi, `ARCH-003` performance-targets, `02-ui-ux/menus.md`, `02-ui-ux/keyboard-shortcuts.md`.

> All crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Adobe's exact filter kernels are closed; this document
> targets **behavioral parity only, algorithm TBD**. Behavioral facts are from the
> fetched CS6 Help PDF unless marked *(inferred)*. Per-filter parameter tables
> live in the family specs; this overview covers the shared plumbing.

## CS6 behavior

### Where a filter applies

- A filter applies to the **active, visible layer**, or to the **current selection** on it. Locked layers, hidden layers, and empty (no-pixel) layers either refuse or no-op. The Blur and Blur Gallery families refuse an empty target with an error dialog (`FILT-010`, `FILT-092`).
- Filters **cannot** be applied to **Bitmap-mode** or **Indexed-color** images. Some filters work only on **RGB** images.
- Filters can be applied to **individual channels** (a different setting per channel, or the same filter with different settings), and some random filters produce visibly different results per channel.
- A filter selected while a **Smart Object** is active becomes a **Smart Filter** (non-destructive) — see `LAY-021`.
- `Filter > Convert For Smart Filters` converts a regular layer to a Smart Object so that subsequent filters can be applied non-destructively.

### Filter menu structure (CS6)

The Filter menu groups Adobe filters in submenus; third-party filters are appended (see *Third-party filters*). Documented submenus from the Filter effects reference: **Artistic, Blur, Brush Stroke, Distort, Noise, Pixelate, Render, Sharpen, Sketch, Stylize, Texture, Video, Other, Digimarc**, plus top-level single filters (**Adaptive Wide Angle, Camera Raw Filter, Lens Correction, Liquify, Oil Paint, Vanishing Point**) and the **Filter Gallery**, which hosts the cumulative-effect filters. Exact menu-vs-gallery membership is *(inferred)* where the source is silent; the category rosters are sourced.

| Menu location | Filter family | 8-bit | 16-bit | 32-bit |
|---|---|---|---|---|
| `Filter > Blur` | Average, Blur, Blur More, Box Blur, Gaussian Blur, Lens Blur, Motion Blur, Radial Blur, Shape Blur, Smart Blur, Surface Blur | Yes | Yes (except Smart Blur, Blur/Blur More, Lens Blur) | Yes (Average Blur, Box Blur, Gaussian Blur, Motion Blur, Radial Blur, Shape Blur, Surface Blur) |
| `Filter > Distort` | Diffuse Glow, Displace, Glass, Ocean Ripple, Pinch, Polar Coordinates, Ripple, Shear, Spherize, Twirl, Wave, ZigZag | Yes | No | No |
| `Filter > Noise` | Add Noise, Despeckle, Dust & Scratches, Median, Reduce Noise | Yes | Yes | Add Noise only |
| `Filter > Sharpen` | Sharpen, Sharpen Edges, Sharpen More, Unsharp Mask, Smart Sharpen | Yes | Yes | Smart Sharpen, Unsharp Mask only |
| `Filter > Pixelate` | Color Halftone, Crystallize, Facet, Fragment, Mezzotint, Mosaic, Pointillize | Yes | No | No |
| `Filter > Render` | Clouds, Difference Clouds, Fibers, Lens Flare, Lighting Effects | Yes | Clouds, Difference Clouds, Fibers, Lens Flare | Clouds, Lens Flare |
| `Filter > Stylize` | Emboss, Find Edges, Glowing Edges, Solarize, Tiles, Trace Contour, Wind | Yes | Emboss, Find Edges, Solarize | Emboss |
| `Filter > Texture`, `Sketch`, `Artistic`, `Brush Stroke` | Gallery-style effects | Yes | No | No |
| `Filter > Other` | Custom, High Pass, Maximum, Minimum, Offset | Yes | Yes | High Pass, Maximum, Minimum, Offset |
| `Filter > Video` | De-Interlace, NTSC Colors | Yes | Yes | De-Interlace, NTSC Colors |
| `Filter > Filter Gallery` | Cascaded subset of Artistic/Brush Stroke/Distort/Sketch/Stylize/Texture | Yes | No | No |
| `Filter > Adaptive Wide Angle`, `Camera Raw Filter`, `Lens Correction`, `Liquify`, `Oil Paint`, `Vanishing Point` | CS6 top-level filters | Yes* | Per filter | Per filter |

\* See the primary source list for the exact 16-bit and 32-bit rosters; this table is a summary and the authoritative lists are transcribed in *Parameters & ranges*.

### Filter Gallery

- `Filter > Filter Gallery` opens a dialog with a preview, filter **categories** with thumbnails, the **options** for the selected effect, and an **applied-filter list** (dialog parts A–J per the primary source).
- Multiple filters are applied **cumulatively and in the order listed**. Applied filters can be **rearranged** by dragging, **hidden** via the eye icon, and **deleted**. Clicking a category name shows thumbnails of its effects; the filter-thumbnail pane can be shown/hidden.
- The gallery is an **8-bit-per-channel** facility: per the Help, most filters can be applied cumulatively through the Filter Gallery on 8-bit-per-channel images. It is not offered for 16- or 32-bit documents.
- Effects applied through the gallery become a **single grouped "Filter Gallery" entry** in the Smart Filters list when applied to a Smart Object (`LAY-021`).

| Filter Gallery key | Windows | macOS |
|---|---|---|
| Apply a new filter on top of selected | Alt-click a filter | Option-click a filter |
| Open/close all disclosure triangles | Alt-click a disclosure triangle | Option-click a disclosure triangle |
| Change Cancel button to Default | Control | Command |
| Change Cancel button to Reset | Alt | Option |
| Undo / Redo | Ctrl+Z | Cmd+Z |
| Step forward | Ctrl+Shift+Z | Cmd+Shift+Z |
| Step backward | Ctrl+Alt+Z | Cmd+Option+Z |

### Filter blending / opacity — Fade

- `Edit > Fade` changes the **opacity and blending mode of the most recently applied filter**, painting tool, erasing tool, or color adjustment. It is described as equivalent to applying the effect on a separate layer and then using the layer's opacity and blend controls.
- Fade controls: a **Preview** option, an **Opacity** slider from **0% (transparent) to 100%**, and a **Mode** menu. The Fade mode list is a **subset of the painting/editing-tool modes, excluding Behind and Clear**.
- The Fade command also applies to the effects of `Liquify` and the Brush Strokes filters (and, by extension, to any destructive filter).
- **Lab caveat:** the Color Dodge, Color Burn, Lighten, Darken, Difference, and Exclusion modes do not work on Lab images.
- On a Smart Object the equivalent controls are the Smart Filter's **Edit Blending Options** (Blend Mode + Opacity), edited from the Layers panel (`LAY-021`).
- "Fade last filter" therefore only applies immediately after a filter/effect; the UI entry is disabled once another operation intervenes. *(inferred: Photoshop exposes only the single most recent effect.)*

### Previews

- Filter dialogs show a **preview window**: drag inside it to recenter, click the **+ / –** buttons (or a zoom percentage) to zoom, and in some filters click in the image to recenter there. Many dialogs also let the effect be previewed directly on the document canvas (moving the dialog aside).
- Preview cost scales with the document; the Help recommends experimenting on **a small, representative part** of the image first.
- `Fade` has its own **Preview** checkbox.

### Third-party / plug-in filters

- Adobe and third-party filter modules are loaded from the Photoshop `Plug-ins` folder. Once installed they appear **at the bottom of the Filter menu**, grouped by the plug-in's declared **category** (for example a category submenu), and file-format/import/export plug-ins appear in their respective menus.
- If the installed plug-in list is too long, Photoshop may fail to place all of them in their categories; **newly installed plug-ins then appear under `Filter > Other`**.
- Filters that declare Smart Filter support can be applied non-destructively to Smart Objects. Plug-ins may also be 16-/32-bit capable depending on the plug-in.
- Adobe `.8bf` binary compatibility is **`Non-goal (Linux)`**; the plug-in model is redefined in `ARCH-011` (native C ABI + in-tree Rust plugins). For this spec, "third-party filter" means an `ARCH-011` filter plug-in registered with `OP_CAP_FILTER`.

### Bit-depth availability (authoritative transcription)

- **All filters** can be applied to **8-bit** images.
- **16-bit** images accept: Liquify, Vanishing Point, Average Blur, Blur, Blur More, Box Blur, Gaussian Blur, Lens Blur, Motion Blur, Radial Blur, Surface Blur, Shape Blur, Lens Correction, Add Noise, Despeckle, Dust & Scratches, Median, Reduce Noise, Fibers, Clouds, Difference Clouds, Lens Flare, Sharpen, Sharpen Edges, Sharpen More, Smart Sharpen, Unsharp Mask, Emboss, Find Edges, Solarize, De-Interlace, NTSC Colors, Custom, High Pass, Maximum, Minimum, and Offset.
- **32-bit** images accept: Average Blur, Box Blur, Gaussian Blur, Motion Blur, Radial Blur, Shape Blur, Surface Blur, Add Noise, Clouds, Lens Flare, Smart Sharpen, Unsharp Mask, De-Interlace, NTSC Colors, Emboss, High Pass, Maximum, Minimum, and Offset.
- Filters may be **processed entirely in RAM**; insufficient RAM produces an error rather than a silent partial result.

### CPU / GPU split (Mercury Graphics Engine, CS6)

- CS6 introduced the **Mercury Graphics Engine**. Adobe statements (quoted by third parties) describe it as using **OpenGL and OpenCL** and delivering near-instant results for **Liquify, Warp/Puppet Warp, Lighting Effects, Oil Paint**, and related interactive operations. With an unsupported GPU, the acceleration is normally lost and the feature falls back to the standard CPU path.
- Independent testing found the **Blur Gallery** filters (Field Blur, Iris Blur, Tilt-Shift) require **OpenCL 1.1**, while **Reduce Noise is primarily CPU**. Adaptive Wide Angle and 3D are also listed as GPU-accelerated.
- Conventional convolution filters (Gaussian, Box, Median, Unsharp Mask, etc.) are **CPU** work in CS6; their dialogs benefit from GPU display/compositing but the pixel math is not the OpenCL path.
- Kooka Pictura interpretation (`ARCH-006`): GPU **display/compositing** via Qt QRhi/wgpu, GPU **compute** for filter kernels where beneficial, and a **CPU fallback** that is always correct. No filter may require a GPU.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Filter` menu bar entry | menu | — | Top-level single filters + families + Filter Gallery |
| `Filter > Last Filter` | menu | `Ctrl/Cmd+F` *(inferred)* | Re-applies the previous filter with its last settings |
| `Filter > Convert For Smart Filters` | menu | — | Converts layer to Smart Object first |
| `Filter > Filter Gallery` | dialog | *(inferred, unverified)* | Cumulative effect stack, 8-bit only |
| `Filter > <Family> > <Filter>` | dialog | — | Per-filter options + preview |
| `Edit > Fade <Filter>` | dialog | `Shift+Ctrl/Cmd+F` *(inferred; not stated in fetched PDF)* | Opacity + mode for the last filter/effect |
| `Filter > <Plugin Category> > <name>` | menu | — | `ARCH-011` filter plug-ins; overflow to `Filter > Other` |
| Layers panel, Smart Filters line | panel | — | Smart Filter list, mask, blending options (`LAY-021`) |
| Filter dialog preview pane | widget | +/- zoom | Drag to recenter; click-to-center where supported |
| Status bar / progress | status | — | Long filters show progress and are cancellable (`ARCH-003`) |

## Parameters & ranges

Global (every filter dialog unless noted):

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Preview | checkbox | on | on / off | Per-dialog; some preview on-canvas |
| Preview zoom | enum/numeric | fit | e.g. 12.5–1600% | Filter-specific; +/- buttons or percentage |
| Preview center | drag/click | — | any image point | Click-to-center not universal |
| Fade Opacity | int % | 100 | 0–100 | 0 = fully original |
| Fade Mode | enum | Normal | paint/edit modes minus Behind, Clear | Lab excludes Dodge/Burn/Lighten/Darken/Difference/Exclusion |
| Filter Gallery stack order | ordered list | — | add / reorder / hide / delete | Applied cumulatively, top-to-bottom list order |
| Bit-depth gate | enum | document depth | 8 / 16 / 32 | Per-filter matrix above; refusal, never silent convert |

Per-filter parameters (Amount, Radius, Threshold, Angle, Distance, Quality, Shape, Blade Curvature, Rotation, Distribution, Monochromatic, etc.) are tabulated in the family specs and are not repeated here. ISO standard parameters such as Unsharp Mask's `Amount/Radius/Threshold` and Smart Sharpen's `Remove/More Accurate` are in `FILT-020`.

## Algorithms & pipeline

The shared application pipeline for a destructive filter:

1. **Resolve target.** Determine the active layer, its pixel bounds, the active selection mask, and any clipping/mask that constrains the write region.
2. **Gate on document type.** Refuse on Bitmap/Indexed; refuse when the filter's color-depth capability does not match the document (per matrix), never converting.
3. **Expand the working region.** For neighborhood filters, the filter reads an **apron** outside the selection so that edge pixels have source data. Adobe notes Gaussian, Box, Motion, and Shape Blur **use data outside the selection**, which can smear foreground color into a background selection edge — the Help recommends **Smart Blur or Lens Blur** to avoid this. Filters that preserve edges (Surface, Smart, Lens) constrain the neighborhood by intensity/depth.
4. **Choose the backend.** GPU compute path when available and the kernel is GPU-ported; otherwise the CPU (`rayon`-parallel) kernel. Preview and final apply use the same kernel so the preview is representative.
5. **Iterate tiles.** Kernels operate tile-by-tile with an apron; progress and cancellation are checked at tile boundaries (`ARCH-003`).
6. **Composite through the selection/mask** and, for color-managed docs, in the document color space (`ARCH-007`); 32-bit float is kept float and not clamped (`IMG-009`).
7. **Commit as one history state**, storing the affected tiles' prior bytes for bit-exact undo (`ARCH-009`).

**Filter Gallery pipeline.** The gallery is an ordered stack of independent effects. Each effect runs its own kernel over the previous stage's result (or over a cached stage image), with visibility toggles; the final composed result is committed as a single history state and, on a Smart Object, serialized as one grouped entry. Because it is 8-bit-only, the gallery runs on the 8-bit representation.

**Fade.** Fade is a composite operation: `result = lerp(original, filtered, opacity)` under `blend(mode)`. It is mathematically the same as a duplicate of the original below/above the filtered layer with the given opacity and mode, which is why the Smart Filter blending options are equivalent. Fade does not re-run the filter; it only changes the last effect's composite.

**Smart Filter path.** A filter applied to a Smart Object is recorded as an **effect node** in the object's Smart Filter stack (`LAY-021`), not baked into pixels. Evaluation order is **bottom-up** in the Layers panel list; a single shared filter mask gates all effects on the object. Each effect carries its own parameters and its own blending options.

**CPU/GPU dispatch.** `ARCH-006` proposes a single compute interface with a GPU (`wgpu`) implementation and a CPU (`rayon`) fallback. HDR and 32-bit kernels keep float precision. Color management inserts ICC/LUT transforms around the kernel for non-RGB spaces.

**Plug-in dispatch.** An `ARCH-011` filter plug-in is invoked through the host API with an ROI; the in-tree `Filter` trait and the C-ABI adapter share one execution path. Capability `OP_CAP_FILTER` governs registration.

## Rust module mapping

Proposed under `pictura-filters` (`ARCH-002`):

- `pictura-filters::Filter` — `trait Filter { fn params_schema() -> &'static [ParamDesc]; fn apply(&self, roi: Rect, src: &TileView, dst: &mut TileView, ctx: &FilterCtx) -> Result<(), FilterError>; }`.
- `pictura-filters::registry` — `FilterRegistry` mapping a stable filter id (e.g. `blur.gaussian`, `noise.reduce`) to metadata (menu path, category, bit-depth mask, GPU capability, smart-filter capable, plugin-origin) and a constructor.
- `pictura-filters::pipeline` — region/apron computation, target resolution, selection mask, tile iteration, progress/cancel, and the single commit path.
- `pictura-filters::fade` — `Fade { opacity: u8, mode: BlendMode }` compositor operating on `(original, filtered)` tiles.
- `pictura-filters::gallery` — ordered `Vec<GalleryStage>` with visibility and reorder; runs 8-bit only.
- `pictura-filters::smart` — bridges to `LAY-021`; effect-node description and evaluation order.
- `pictura-filters::kernel::{blur, sharpen, noise, ...}` — family submodules (see family specs).
- `pictura-filters::gpu` — optional `wgpu` compute backend behind the same `Filter` interface.
- `pictura-filters::plugin` — adapter over `ARCH-011` filter plug-ins.

Crossing types: `Rect` ROI, `TileView`/`TileMap` borrows, `ParamValue` (a tagged union: `F32`, `U32`, `Enum(u16)`, `Bool`), `BlendMode`, `FilterId`. No Qt types; `pictura-filters` depends only on `pictura-core`, `pictura-render`, `pictura-color`.

## Qt6 component mapping

Proposal (Widgets, matching the CS6 docked/modal feel; dialog logic is Qt, pixel work is Rust):

- `FilterMenuBuilder` / `FilterMenuModel` — builds the `Filter` `QMenu` from `FilterRegistry` metadata; appends plug-in categories and the `Other` overflow.
- `FilterDialogBase` (`QDialog`) — hosts a `QImage`/`QLabel` preview, a +/- zoom control, drag-to-pan, a `Preview` checkbox, and an OK/Cancel/Reset button box; the concrete dialog embeds `QDoubleSpinBox`/`QSlider`/`QComboBox`/`QCheckBox` controls driven by `params_schema()`.
- `FilterGalleryDialog` (`QDialog`) — category tree + thumbnail `QListWidget`, effect options pane, and the applied-stack `QListWidget` with drag-reorder/visibility/delete and the gallery keyboard map above.
- `FadeDialog` (`QDialog`) — opacity slider + mode combo + preview.
- `FilterProgressProxy` (`QProgressDialog`) — connects to the Rust cancellation token.
- `SmartFilterListModel` — the Layers-panel Smart Filters list, filter mask, and blending-options editor (`LAY-021`, `02-ui-ux/panels/layers-panel.md`).
- `PluginFilterMenuModel` — plug-in filter categories from `ARCH-011`.

QML is not proposed: filter dialogs are modal tools with tight numeric layout and native menu integration, which Widgets handle directly.

## Data-model impact

- **Destructive filters add no persistent document fields.** They mutate channel tiles and add one `HistoryRecord` per committed apply (undo record: `FilterOp { filter_id, roi, params_blob, before_tiles, after_hash }`), per `ARCH-009`.
- **Fade** mutates only the last effect's compositing parameters; it does not re-run the kernel.
- **Smart Filters** are persistent: an effect stack on the Smart Object, each effect with `{ filter_id, params, blend_mode, opacity, enabled }`, plus one shared **filter mask** channel; serialized in the PSD/PSB additional-layer-information block (`LAY-021`, `ARCH-008`). Exact PSD keys for the stack are unsourced (see `LAY-021` Open questions).
- **Filter Gallery on a Smart Object** serializes as one grouped effect entry.
- **Registry/metadata** (menu paths, bit-depth masks, GPU capability) is compile-time for built-ins and runtime for plug-ins; not serialized.
- **Blend modes** referenced by Fade and Smart Filter blending map to `LAY-010`; no new modes.

## Edge cases

- **Selection-edge contamination.** Neighborhood blurs read outside the selection; the design must decide (per filter) between sampling the apron (Adobe behavior for Gaussian/Box/Motion/Shape) and edge-aware/selection-clamped behavior (Smart/Lens Blur). Documented in each family spec.
- **Locked transparent pixels / locked layer / empty layer.** Refuse or no-op; never write under transparency when the lock is set. Blurs refuse an empty layer (or a selection over only transparency) with "Could not complete the `<command>` command because the selected area is empty." — the Blur Gallery says "…the initial bounding rectangle is empty."
- **Bitmap / Indexed.** Refuse every filter.
- **CMYK / Lab / Grayscale / Multichannel / Duotone.** Only the filters documented for those modes run; Lab Fade mode restrictions apply; unsupported ops refuse rather than convert.
- **8/16/32-bit.** Enforce the availability matrix; 32-bit stays float and unclamped; never silently narrow.
- **Huge PSB documents.** Tile-local processing only; no whole-canvas copy; `apron` bounded; the 30,000 px note says some plug-in filters are unavailable on very large documents.
- **Insufficient RAM.** Return a filter error (Adobe's "not enough RAM" behavior) rather than partially writing.
- **GPU unavailable / device lost.** CPU fallback; the user-visible indicator per `ARCH-006`; no filter requires GPU.
- **Cancellation.** Check per tile; on cancel, discard the pending commit and leave no history state (`ARCH-003`).
- **Undo/redo.** Apply and Fade are atomic; cancel and errors are atomic no-ops.
- **Plug-in missing/disabled.** A previously saved Smart Filter referencing a missing plug-in must degrade to a warning/placeholder, not crash (`ARCH-011`, `LAY-021`).
- **Preview accuracy.** Preview and final must use the same kernel/precision; a lower-quality preview (e.g. Lens Blur "Faster") must be labelled and never committed.
- **Third-party filter overflow.** More plug-in filters than the menu can hold fall back to `Filter > Other`.

## Parity acceptance criteria

1. Given an 8-bit RGB document, every built-in Filter-menu entry appears in the documented submenu and opens a dialog with a working preview; given 16- or 32-bit, only the filters in the authoritative depth lists are enabled and the rest are disabled with a reason.
2. Given a Bitmap-mode or Indexed-color document, every filter is disabled/refused and no conversion occurs.
3. Given a selection, applying Gaussian Blur produces the documented edge behavior (reads outside the selection), while applying Smart Blur or Lens Blur does not contaminate the selection edge in the same way.
4. Given two filters applied through the Filter Gallery, reordering them changes the result; hiding one removes its contribution; deleting one restores the prior result; the whole stack commits as exactly one history state.
5. Given a filter applied to a layer, `Edit > Fade` at 50% with mode Normal yields a result within 1 LSB (8/16-bit) of compositing the filtered layer at 50% over the original; Fade at 0% equals the original; Fade on a Lab image disables the restricted modes.
6. Given a Smart Object, applying a filter adds a Smart Filter entry and leaves the underlying pixels unchanged; double-clicking the entry reopens the dialog with the saved parameters; disabling the entry or its mask removes the effect.
7. Given an `ARCH-011` filter plug-in declaring `OP_CAP_FILTER`, it appears under its category in the Filter menu and, when too many plug-ins are installed, overflow entries land under `Filter > Other`.
8. Given a filter apply, cancelling at the cancellation point leaves the document byte-identical and adds no history state; completing it adds exactly one state.
9. Given missing GPU support, every filter still applies on the CPU with numerically correct output within the family tolerance.
10. Given a 32-bit HDR document and a filter in the 32-bit list, values above 1.0 are not clamped and precision is preserved.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — Adobe Photoshop CS6 Help reference (downloaded to `/tmp` and text-extracted with `pdftotext`). Established: filter application rules (active visible layer / selection; no Bitmap or Indexed; some RGB-only); Filter Gallery overview, dialog parts A–J, cumulative/reorder/hide/delete, 8-bit-only statement; Filter Gallery keyboard table; Fade (opacity 0–100%, mode subset excluding Behind/Clear, Lab restrictions, applies to Liquify/Brush Strokes); preview behavior; third-party filters at the bottom of the Filter menu and the `Filter > Other` overflow when the list is too long; the authoritative 16-bit and 32-bit filter lists; the Filter effects reference submenus and per-family descriptions; CS6 What's-New filter additions (Adaptive Wide Angle, Blur Gallery, Lighting Effects 64-bit gallery, Oil Paint). Primary source.
- `https://en.wikipedia.org/wiki/Gaussian_blur` — Gaussian as separable FIR convolution, kernel normalization, O(w·h) separable cost, box-blur/IIR alternatives; used for the overview pipeline and the blur family. Standard reference.
- `https://en.wikipedia.org/wiki/Unsharp_masking` — USM as original + (original − blurred)·amount, amount/radius/threshold semantics, mask/deconvolution distinction. Standard reference.
- `https://en.wikipedia.org/wiki/Median_filter` — non-linear rank filter, edge preservation, histogram fast median. Standard reference.
- `https://en.wikipedia.org/wiki/Bilateral_filter` — edge-preserving bilateral definition; notes that Photoshop's **Surface Blur** is a bilateral filter. Standard reference.
- `https://en.wikipedia.org/wiki/Box_blur` — box blur as spatial linear moving average. Standard reference.
- `https://en.wikipedia.org/wiki/Motion_blur` — motion blur as intra-exposure streaking (direction + distance). Standard reference.
- `https://en.wikipedia.org/wiki/Bokeh` — bokeh as the aesthetic quality of out-of-focus blur (Lens Blur family). Standard reference.
- `https://en.wikipedia.org/wiki/Additive_white_Gaussian_noise` — AWGN model (Add Noise family). Standard reference.

Repository sources used for the design mapping (fetched locally):

- `docs/01-architecture/gpu-rendering-pipeline.md` — Mercury Graphics Engine scope (Liquify, Warp/Puppet Warp, Lighting Effects, Oil Paint, Adaptive Wide Angle, Blur Gallery, 3D), Blur Gallery OpenCL 1.1, Reduce Noise CPU, CPU fallback.
- `docs/01-architecture/performance-targets.md` — filter apply budgets, tile/128 KB model, cancellation ≤100 ms, `pictura-filters::cancel`.
- `docs/01-architecture/plugin-and-scripting-abi.md` — filter plug-in model, `OP_CAP_FILTER`, `.8bf` non-goal.
- `docs/01-architecture/rust-core-design.md` — `pictura-filters` crate and `pictura-core` types.
- `docs/05-layers/smart-filters.md` — Smart Filter stack, mask, blending options, gallery grouping.

Searches performed (not documents fetched): DuckDuckGo/Brave for CS6/CC blur-gallery GPU requirements and filter range confirmation; results are marked *(inferred)* where used.

## Open questions

- **Exact Filter-menu tree.** The Help PDF lists filter *categories* and top-level single filters but not a literal menu tree. Which categories (Artistic, Brush Strokes, Sketch) appear as menu submenus versus only inside Filter Gallery must be confirmed from a CS6 build. Resolve by capturing the CS6 Filter menu.
- **Full per-filter parameter ranges/defaults.** Only some are in the primary source (Dust & Scratches radius 1–16, threshold 0–255; USM radius 0.1–250 from current Adobe docs). The rest are marked *(inferred)* in the family specs. Resolve by scripted read of each CS6 dialog.
- **Fade and Last-Filter shortcuts.** `Shift+Ctrl/Cmd+F` (Fade) and `Ctrl/Cmd+F` (Last Filter) are widely reported but were not found in the fetched PDF text. Resolve against the CS6 Keyboard Shortcuts reference.
- **`Average` vs `Average Blur`.** The 16-/32-bit lists say "Average Blur"; the Blur submenu item is "Average". Confirm whether these are the same filter's internal/report name.
- **32-bit filter list completeness.** The list is transcribed verbatim but it is unclear whether it reflects CS6-at-launch or a later CS6 update. Resolve per dot release.
- **Filter Gallery thumbnail rendering.** Whether thumbnails are computed at a fixed small size on the full image or a proxy is undocumented. Resolve empirically; affects our preview cost budget.
- **GPU port set.** Adobe does not publish which kernels are OpenCL versus OpenGL versus CPU in CS6. `ARCH-006` proposes our own split; the parity question is only which filters *must* stay fast without a GPU. Resolve with the `ARCH-003` budgets.
- **Smart-filter capability of Liquify.** Shipped CS6 lists Liquify among non-smart filters; a later Creative Cloud update added support (`LAY-021`). Decide which CS6 build we target.
- **Whether Fade can apply to a Smart Filter without editing blending options.** UI evidence says Fade targets destructive effects only; confirm.
