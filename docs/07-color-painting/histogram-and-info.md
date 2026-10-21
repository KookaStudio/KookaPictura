# Histogram and Info Panels

- **Spec ID:** `CLR-004`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `No` — both panels are functionally unchanged. CS6's move of adjustments to the **Properties panel** means the Info panel's before/after readouts and the Histogram's adjustment preview are driven from there rather than from the CS5 Adjustments panel.
- **Depends on:** `CLR-001` color-models, `ARCH-007` color-management, `ARCH-008` document-model, `01-architecture/performance-targets.md`, `01-architecture/gpu-rendering-pipeline.md`, `02-ui-ux/panels/histogram-panel.md`, `02-ui-ux/panels/info-panel.md`, `03-tools/eyedropper-color-sampler-ruler.md`, `03-tools/note-and-count.md`, `04-image-ops/adjustments-overview.md`, `10-workflow-io/measurement-and-count.md`.

> All crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Facts not confirmed by a fetched CS6 source are marked
> *(inferred)*.

## CS6 behavior

### Histogram panel

`Window > Histogram` (or the Histogram tab) opens the panel. By default it opens in Compact View, which shows only the histogram with no controls or statistics. A histogram graphs the pixel count at each intensity level to show how pixels are distributed across the image.

**Reading it.** Shadows are on the left, midtones in the middle, highlights on the right. It also conveys the image **key type**: a **low-key** image concentrates detail in shadows, a **high-key** image in highlights, an **average-key** image in midtones, and a **full tonal range** image has pixels in all areas.

**Views** (from the panel menu):

| View | Contents |
|---|---|
| **Compact View** | Histogram only, no controls or statistics; represents the entire image. |
| **Expanded View** | Histogram with statistics, the channel menu, view options, Uncached Refresh, and the Source (layer) selector. |
| **All Channels View** | Individual histograms of the channels plus all Expanded View options; the per-channel histograms exclude alpha, spot, and mask channels. |

**Channel menu** (Expanded / All Channels):

- An individual **color channel**, or **alpha** or **spot** channels.
- **RGB**, **CMYK**, or **Composite** — composite histogram of all channels, by document mode.
- **Luminosity** (RGB/CMYK) — luminance/intensity of the composite.
- **Colors** (RGB/CMYK) — composite of the individual color channels drawn in color; the default for RGB/CMYK when entering Expanded or All Channels view.
- Photoshop keeps the channel selection when you switch from Expanded View or All Channels View back to Compact View. In All Channels View, the Channels menu affects only the **topmost** histogram.

**Show Channels In Color** is available from the panel menu in All Channels View or for an individually selected channel; the color persists in Compact View.

**Statistics** (Expanded / All Channels; shown by default, toggled by **Show Statistics**). Point at the histogram for a single level, or drag to select a range:

| Statistic | Meaning |
|---|---|
| **Mean** | Average intensity value. |
| **Std Dev** | How widely intensity values vary. |
| **Median** | Middle value in the range of intensity values. |
| **Pixels** | Total number of pixels used to calculate the histogram. |
| **Level** | Intensity level under the pointer. |
| **Count** | Number of pixels at the level under the pointer. |
| **Percentile** | Cumulative pixels at or below the level, as a percentage from 0% (left) to 100% (right). |
| **Cache Level** | Image cache level used to build the histogram (see below). |

**Source menu** (Expanded View only; "not available for single-layered documents"):

| Source | Meaning |
|---|---|
| **Entire Image** | Histogram of the whole image, all layers. |
| **Selected Layer** | Histogram of the layer selected in the Layers panel. |
| **Adjustment Composite** | Histogram of a selected adjustment layer **including all layers below it**. |

**Preview adjustments.** With **Preview** enabled in any color/tonal adjustment dialog, the Histogram panel shows how the adjustment affects the histogram (original vs adjusted). Changes made via the Adjustments (CS5) / Properties (CS6) panel "are automatically reflected in the Histogram panel."

**Refresh / cache.** When a histogram is read from the cache rather than the current document, the **Cached Data Warning** icon appears. Cache-based histograms are faster and are computed from a representative pixel sample whose density depends on the current magnification. The **original image is cache level 1**; each higher level averages four adjacent pixels into one (each level has 1/4 the pixels of the level below). The maximum cache level (2–8) is set in the **Performance** preference; higher levels speed redraw on large multi-layer files but use more RAM. Refresh uncached by:

- double-clicking anywhere in the histogram,
- clicking the **Cached Data Warning** icon,
- clicking the **Uncached Refresh** button, or
- choosing **Uncached Refresh** from the panel menu.

### Info panel

`Window > Info` (shortcut `F8`, which toggles show/hide). The panel reports the color values under the pointer and, for the active tool, other contextual information such as tool hints, document status, and 8-/16-/32-bit values.

**Contextual readouts:**

- Numeric color values beneath the pointer; depending on the option, **8-bit, 16-bit, or 32-bit**.
- When displaying **CMYK** values, an exclamation point marks a value when the color under the pointer or a color sampler falls outside the printable CMYK gamut.
- **Marquee tools**: x/y pointer position and width (W) / height (H) of the marquee while dragging.
- **Crop / Zoom**: W/H of the marquee and the crop marquee's angle of rotation.
- **Line / Pen / Gradient / moving a selection**: start x,y, change in X (DX), change in Y (DY), angle (A), and length (D).
- **2-D transform command**: percentage change in width (W) and height (H), rotation angle (A), and horizontal (H) or vertical (V) skew angle.
- **Any color adjustment dialog** (e.g. Curves): before-and-after values for pixels beneath the pointer and beneath color samplers. In CS6 this is fed by the Properties panel.
- **Show Tool Hints**: a hint for the selected tool.
- **Status information** (selectable): document size, document profile, document dimensions, scratch sizes, efficiency, timing, current tool, measurement scale.

**Info Panel Options** (`Panel Options` from the panel menu, or via the eyedropper icon):

- **First Color Readout** and **Second Color Readout**: **Actual Color** (current mode), **Proof Color** (output space), **a color mode**, **Total Ink** (sum of CMYK ink %, based on CMYK Setup), **Opacity** (current layer; not the background).
- **Ruler Units** (also set by clicking the crosshair icon).
- Status-information checkboxes.
- **Show Tool Hints**.

**Color samplers.** Up to **four** samplers can be placed; they are stored in the image, so they remain available as you work and survive closing and reopening the document. Place with the Color Sampler tool (or `Shift`-click with the Eyedropper), choose a **sample size** in the options bar (Point Sample or an N×N average), and view readings in the lower half of the Info panel. Samplers can be moved, deleted (drag out, or `Alt`/`Option`-click with scissors, or **Clear** in the options bar), hidden/shown via `View > Extras` and the panel's **Color Samplers** toggle, and each sampler's color space can be changed from its icon in the panel.

**Keys for the Info panel:** click the **eyedropper icon** to change color readout modes; click the **crosshair icon** to change measurement units. The 32-bit readout is selected from the eyedropper icon's pop-up menu.

### Cross-references

Panel anatomy, docking, and theme are owned by `02-ui-ux/panels/histogram-panel.md` and `02-ui-ux/panels/info-panel.md`; the sampling tools by `03-tools/eyedropper-color-sampler-ruler.md`; the measurement-log integration by `03-tools/note-and-count.md` and `10-workflow-io/measurement-and-count.md`.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Window > Histogram` | Panel | — | Opens Histogram panel; Compact View default |
| Histogram panel menu | Menu | — | Expanded / Compact / All Channels; Show Statistics; Show Channels In Color; Uncached Refresh |
| Histogram Channel menu | Combo | — | Channels / RGB / CMYK / Composite / Luminosity / Colors |
| Histogram Source menu | Combo | — | Entire Image / Selected Layer / Adjustment Composite |
| Histogram Uncached Refresh | Button | — | Redraws from actual pixels |
| Histogram — double-click | Gesture | — | Uncached refresh |
| `Window > Info` | Panel | `F8` | Toggle Info panel |
| Info panel eyedropper icon | Menu | click | First/Second readout + 8/16/32-bit |
| Info panel crosshair icon | Menu | click | Ruler units |
| Info panel menu | Menu | — | Panel Options; Color Samplers toggle |
| `Edit > Preferences > Performance` | Pane | — | Max cache level 2–8 |
| Options bar (Color Sampler) | Controls | — | Sample Size; Clear |
| `View > Extras` | Toggle | `Ctrl/Cmd+H` | Show/hide samplers and other extras |
| Color Sampler tool | Tool | — | Place up to 4 samplers |
| Eyedropper tool + `Shift`-click | Gesture | — | Adds a sampler |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Histogram view | enum | Compact View | Compact / Expanded / All Channels | Panel menu |
| Channel | enum | Colors (RGB/CMYK, in Expanded) | per-channel / RGB / CMYK / Composite / Luminosity / Colors / alpha / spot | Remembered when returning to Compact |
| Source | enum | Entire Image | Entire Image / Selected Layer / Adjustment Composite | Expanded only; hidden for single-layer docs |
| Show Statistics | bool | on (Expanded) | on / off | Statistics block visibility |
| Show Channels In Color | bool | off | on / off | Per-channel/composite color drawing |
| Cache Level | int (read-out) | 1 | 1 … max (2–8) | 1 = original; each level = 1/4 pixels |
| Max cache level | int (pref) | 4 *(inferred)* | 2 … 8 | Performance preference |
| First Color Readout | enum | Actual Color *(inferred)* | Actual Color / Proof Color / a mode / Total Ink / Opacity | Info options |
| Second Color Readout | enum | mode *(inferred)* | same set | Info options |
| Color bit-depth readout | enum | 8-bit *(inferred)* | 8-bit / 16-bit / 32-bit | Eyedropper icon menu |
| Ruler Units | enum | pixels *(inferred)* | pixels / inches / cm / mm / points / picas / percent | Crosshair icon |
| Status information | checkbox set | Document Sizes *(inferred)* | Document Sizes / Profile / Dimensions / Scratch Sizes / Efficiency / Timing / Current Tool / Measurement Scale | Per item |
| Show Tool Hints | bool | off *(inferred)* | on / off | Info options |
| Color samplers | count | 0 | 0 … 4 | Saved in the image |
| Sample size | enum | Point Sample | Point / 3×3 / 5×5 / 11×11 / 31×31 / 51×51 / 101×101 | Options bar (`CLR-002`-adjacent) |

## Algorithms & pipeline

### Histogram computation

- **Per-channel histogram** — bin the working-space channel values. At **8 bpc**: 256 bins indexed directly. At **16 bpc**: 65,536 bins (reduce to a display resolution only, never for statistics). At **32 bpc**: values are unbounded floats; the panel uses the document's display/exposure mapping to produce a bounded histogram *(inferred — the Help does not define 32-bit histogram binning)*. Luminosity follows the RGB→luminance weighting used by the display pipeline.
- **Statistics** are computed from the histogram itself: `Mean = Σ(i·count_i)/N`, `StdDev = sqrt(Σ count_i·(i−Mean)²/N)`, `Median` from the cumulative distribution, `Pixels = N`, `Percentile` from the cumulative sum at the hovered level.
- **Cache / decimation** — cache level `k > 1` is the level-`(k−1)` image with each 2×2 block averaged to one sample (four adjacent pixels → one). The **max cache level** (2–8) bounds the pyramid. The histogram from cache `k` is "a representative sampling of pixels," hence the warning badge and the Uncached Refresh path. Uncached refresh reads "the actual image layer."
- **Source routing** — Entire Image flattens the visible composite (or the document histogram); Selected Layer reads that layer's pixels; Adjustment Composite evaluates the adjustment layer against all layers below. All are computed against the current state, not saved.

### Info readout pipeline

On each pointer move the panel produces a `Readout` record:

- Sample the pixel(s) at the pointer with the active **sample size** (point or N×N average), from the appropriate source (all layers / current layer / current-and-below, per the CS6 Eyedropper options).
- Convert to the selected **readout space**: Actual (document mode), Proof (proof-space transform), a named mode, **Total Ink** (sum of CMYK percentages from the CMYK setup), or **Opacity** (active layer, not background).
- Emit the numeric string at the chosen 8/16/32-bit precision, with the CMYK out-of-gamut flag when the proof/profile transform marks the color unprintable.
- Geometric readouts (x/y, W/H, DX/DY, angle, length, skew) come from the active tool's gesture state, not from pixel data.
- **Color samplers** run the same readout at up to four fixed image coordinates, stored per document.

### Performance

Histogram and statistical accumulation are O(pixels) for the uncached path and O(pixels/4^k) cached. They must stream by tile, respect the scratch/cache budget (`01-architecture/performance-targets.md`), and be cancellable. The Info readout is per-pointer-move and must not allocate per event.

## Rust module mapping

- `pictura_analysis::histogram` — `Histogram { bins: Vec<u64>, channel: ChannelId, space: ReadoutSpace }`, `compute(tile_source, channel, cache_level)`, `composite(mode)`, `luminosity()`.
- `pictura_analysis::stats` — `HistogramStats { mean, std_dev, median, pixels, level, count, percentile, cache_level }`; `at_level(i)` and `over_range(a..b)`.
- `pictura_analysis::cache` — `ImagePyramid { levels }`, `decimate_2x2`, `max_level_from_prefs`.
- `pictura_analysis::sampler` — `ColorSampler { doc_id, index, point, sample_size, readout_space }`, `DocumentSamplers([Option<ColorSampler>; 4])`.
- `pictura_analysis::readout` — `Readout { color: Option<ColorReadout>, geometry: Option<GeometryReadout>, status: StatusBits }`, `ColorReadout { space, precision, cmyk_out_of_gamut }`.
- `pictura_color::transform` — proof/actual conversions reused from `ARCH-007`.
- `pictura_core::prefs` — histogram max cache level, readout defaults, ruler units, status toggles.

Crossing types: `ChannelId`, `ReadoutSpace`, `HistogramStats`, `ColorSampler`, `Readout`, `DocumentId`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `HistogramPanel` | `QDockWidget` | Host; view switch; channel/source combos; refresh |
| `HistogramView` | custom `QWidget` | Draws bins; hover/range selection; cached-warning badge |
| `HistogramStatsView` | `QFormLayout` | Mean / Std Dev / Median / Pixels / Level / Count / Percentile / Cache Level |
| `HistogramModel` | `QAbstractItemModel` | Bins + statistics; recompute on document/selection change |
| `InfoPanel` | `QDockWidget` | Host; readout grid; eyedropper/crosshair icons; sampler block |
| `InfoReadoutGrid` | `QGridLayout` | First/Second readout rows, x/y, W/H, DX/DY, A, D, skew |
| `InfoOptionsDialog` | `QDialog` | First/Second readout, ruler units, status checkboxes, tool hints |
| `ColorSamplerOverlay` | Canvas overlay | Draws the up-to-four sampler marks; drag/delete |
| `ColorSamplerModel` | `QAbstractListModel` | Sampler list + per-sampler color space |
| `StatusBarReadout` | `QWidget` | Document size/profile/dimensions/scratch/efficiency/timing/tool |

Widgets, not QML: dense, always-visible instrument panels consistent with `ARCH-003`. The histogram is a custom-painted `QWidget` (cheap, high-refresh) rather than a scene graph.

## Data-model impact

- **Histogram is derived, never serialized.** It is recomputed from document state; the histogram cache/pyramid is runtime-only.
- **Color samplers are document data.** Up to four `(point, sample_size, readout_space)` records are saved in the image and must round-trip through PSD/open-save (PSD image-resource / measurement support; exact key *(inferred)* — see `01-architecture/file-formats.md`).
- **Info panel options and histogram cache level are preferences**, not document data (`11-cross-cutting/preference-storage.md`).
- **Undo:** neither placing/moving a sampler nor changing a readout is a History state; samplers are not image pixels *(inferred; matches CS6 panel behavior)*. Measurement-log entries from `10-workflow-io/measurement-and-count.md` are a separate data set.
- **Source selection and view mode are UI state** and never serialized.
- **CMYK out-of-gamut flag** is derived from the proof/CMYK working space each readout (`ARCH-007`).

## Edge cases

- **8/16/32-bit** — 8-bit uses 256 bins; 16-bit needs full-precision bins; 32-bit floats must be mapped to a bounded display histogram without integer clipping, and 32-bit readouts are selected explicitly from the eyedropper menu.
- **CMYK / Lab documents** — Actual Color follows the document mode; Total Ink applies only to CMYK; the out-of-gamut exclamation uses the CMYK working space.
- **Alpha / spot channels in All Channels View** — the Help excludes alpha, spot, and masks from the *individual channel histograms*; the Channel menu can still select an alpha/spot channel in Expanded View. Reproduce that asymmetry.
- **Single-layer documents** — the Source menu is hidden.
- **Empty / 1-pixel documents** — histogram has one populated bin; statistics must not divide by zero on empty selections.
- **Huge (PSB) documents** — uncached histogram must stream tiles; the pyramid must not duplicate the full image per level beyond the memory budget.
- **Cache staleness** — after an edit the badge appears until Uncached Refresh; the panel must not silently show a stale histogram as current.
- **GPU unavailable** — histogram/readout computed on CPU; results must match the GPU path within tolerance.
- **Pointer at canvas edge** — N×N sampler averages must clamp or reject out-of-bounds pixels consistently with the Eyedropper (`CLR-002`).
- **Five or more samplers** — the fifth placement is refused; Clear removes all.
- **Undo/redo** — refreshing the histogram after undo must reflect the restored pixels.
- **NaN / infinite HDR values** — must not produce non-finite statistics.
- **High-frequency pointer moves** — readout updates must be coalesced to the UI frame rate.

## Parity acceptance criteria

1. Given an 8-bpc RGB document, the Expanded histogram's channel menu offers RGB, Composite, Luminosity, and Colors; selecting Luminosity matches the RGB-luminance histogram within tolerance.
2. Given a multilayered document, choosing Source = Selected Layer changes the histogram to that layer only, and Source = Adjustment Composite includes all layers below the selected adjustment layer.
3. Given the stats block and the pointer hovering a bin, Level and Count correspond to that bin and Percentile equals the cumulative percentage at that level.
4. Given a cached histogram, the Cached Data Warning appears; clicking it or the Uncached Refresh button redraws from actual pixels and clears the warning.
5. Given the Performance preference max cache level set to N (2–8), the displayed Cache Level never exceeds N, and level k has 1/4 the pixels of level k−1.
6. Given an adjustment dialog with Preview on, the Histogram panel shows the original and adjusted histograms and reverts on cancel.
7. Given the Info panel in CMYK readout over an out-of-gamut pixel, the CMYK values carry the exclamation warning.
8. Given a marquee drag, the Info panel shows x/y and W/H; given a transform, it shows the percentage W/H, angle, and skew.
9. Given four color samplers, they persist across save/close/reopen; a fifth placement is refused and **Clear** removes all four.
10. Given the Info panel with the eyedropper menu set to 32-bit on an HDR document, color readouts are shown at 32-bit precision.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — official CS6 Help reference. Established: histogram definition, shadow/midtone/highlight regions, and low-/high-/average-key and full-tonal-range terms; the three Histogram views and what each shows; the channel menu (individual/alpha/spot, RGB/CMYK/Composite, Luminosity, Colors) and the "remembered in Compact View" rule; the All-Channels topmost-histogram rule; Show Channels In Color; the full statistics list (Mean, Std Dev, Median, Pixels, Level, Count, Percentile, Cache Level) and the 1/4-pixels-per-level cache description; the Source menu (Entire Image / Selected Layer / Adjustment Composite) and its single-layer-document absence; the Preview-adjustment behavior; the four uncached-refresh paths and the Performance cache-level preference (2–8); the Info panel's full contextual readout list (8/16/32-bit, CMYK exclamation, marquee, crop/zoom, line/pen/gradient/selection, transform, adjustment before/after, status information, tool hints); Info Panel Options (First/Second Color Readout with Actual Color, Proof Color, a color mode, Total Ink, Opacity; Ruler Units; status checkboxes; Show Tool Hints); the eyedropper/crosshair icon shortcuts and 32-bit selection; the up-to-four color-sampler model, its save-in-image behavior, sample sizes, move/delete/hide operations, per-sampler color space, and `View > Extras`/panel-menu toggles; `F8` for the Info panel; the CS6 Properties-panel context for before/after readouts and adjustment preview.
- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — same file, "Productivity enhancements (JDI's) in CS6 → Eyedropper": added "ignore adjustment layers" and "current layer and below" Sample options and sample-size context menus (the sampling sources the Info panel reads through).

Not fetched (HTTP 403 from this environment): `helpx.adobe.com` histogram/Info pages linked from the CS6 PDF.

## Open questions

- **32-bit histogram binning** — how the panel maps unbounded HDR floats to bins and which exposure mapping it uses. *Resolves with:* a CS6 32-bit histogram capture / comparison.
- **Exact default readout modes, ruler units, and status toggles** (assumed Actual Color / pixels / Document Sizes). *Resolves with:* a CS6 first-run Info panel capture.
- **Whether samplers are reorderable or carry names**, and their exact PSD serialization keys. *Resolves with:* PSD inspection and `01-architecture/file-formats.md`.
- **Whether sampler/readout changes are undoable.** *Resolves with:* a CS6 experiment.
- **Luminosity weights** (Rec.709 vs Adobe's display formula). *Resolves with:* a CS6 histogram comparison.
- **All Channels View default channel and layout on first open.** *Resolves with:* a CS6 capture.
- **Cache level default** (assumed 4) and its interaction with the scratch/memory budget. *Resolves with:* the Performance preference defaults and `01-architecture/performance-targets.md`.
