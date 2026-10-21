# Histogram Panel

- **Spec ID:** `PAN-015`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `No` — the Histogram panel is unchanged. The CS6 move of adjustments to the **Properties panel** means the adjustment-preview behavior is driven from there rather than from the CS5 Adjustments panel.
- **Depends on:** `CLR-004` histogram-and-info, `ARCH-003` qt6-ui-design, `ARCH-006` gpu-rendering-pipeline, `ARCH-008` document-model, `04-image-ops/adjustments-overview.md`, `02-ui-ux/preferences.md`.

> This document owns the **Histogram panel UI surface**. The binning, statistics
> formulas, cache/pyramid model, and source routing are specified in `CLR-004`
> (`histogram-and-info.md`); the Info panel is `PAN-013`. All crate, module,
> widget, and type names are **design proposals**. The repository contains a
> working implementation; this file stays the behavioural contract for the
> feature, and `docs/dev/doc-code-parity-gaps.md` records what shipped and what
> did not. Facts not confirmed by a fetched CS6 source are marked *(inferred)*.

## CS6 behavior

`Window > Histogram`, or clicking the **Histogram** tab, opens the panel. The panel opens in **Compact View** by default, showing the histogram with no controls or statistics; the view can be changed. A histogram graphs the number of pixels at each color intensity level, showing how the image's pixels are distributed; shadows are on the left, midtones in the middle, highlights on the right.

**Default placement.** The Histogram panel is not in the default Essentials workspace; it is supplied by the **Photography** workspace (with Info and Actions) in the right-hand column (`02-ui-ux/workspace-and-docks.md`, `UI-003`).

### Views (panel menu)

| View | Contents |
|---|---|
| **Compact View** | Histogram only, no controls or statistics; represents the entire image |
| **Expanded View** | Histogram with statistics, the channel menu, view options, **Uncached Refresh**, and the **Source** (layer) selector |
| **All Channels View** | Individual histograms of the channels **plus** all Expanded-View options; individual histograms omit alpha channels, spot channels, and masks |

The panel keeps the current channel setting when switching from Expanded View or All Channels View back to Compact View.

### Anatomy

From the CS6 Help figure labels for the Expanded view (A–E):

| Label | Element |
|---|---|
| A | **Channel** menu |
| B | Panel menu |
| C | **Uncached Refresh** button |
| D | **Cached Data Warning** icon |
| E | **Statistics** block |

The adjustment-preview figure adds **A. Original histogram**, **B. Adjusted histogram**, and the **C. Shadows / D. Midtones / E. Highlights** axis labels.

### Channel menu

Available in Expanded and All Channels views:

- An individual **color channel**, or **alpha** or **spot** channels.
- **RGB**, **CMYK**, or **Composite** — a composite histogram of all channels, by document mode.
- **Luminosity** (RGB/CMYK) — "the luminance or intensity values of the composite channel."
- **Colors** (RGB/CMYK) — "a composite histogram of the individual color channels in color"; the default for RGB/CMYK on first entering Expanded or All Channels view.

In All Channels View, choosing from the Channels menu affects only the **topmost** histogram in the panel.

### Show Channels In Color

From the panel menu: in **All Channels View**, it colorizes all channel histograms; in **Expanded** or **All Channels** view, selecting an individual channel and enabling it colors that channel — and the color is retained when switching to Compact View. Selecting **Colors** likewise keeps its color in Compact View.

### Statistics (Expanded / All Channels)

By default, statistics appear in the Expanded View and All Channels View. Toggle with **Show Statistics**. Hover a bin for one value or drag to select a range; the block shows:

| Statistic | Meaning |
|---|---|
| **Mean** | Average intensity value |
| **Std Dev** | How widely intensity values vary |
| **Median** | Middle value in the range of intensity values |
| **Pixels** | Total number of pixels used to calculate the histogram |
| **Level** | Intensity level of the area under the pointer |
| **Count** | Total number of pixels at the level under the pointer |
| **Percentile** | Cumulative pixels at or below the level, 0% (far left) … 100% (far right) |
| **Cache Level** | Current image cache used to create the histogram |

### Source menu (Expanded View only)

"The Source menu is not available for single-layered documents."

| Source | Meaning |
|---|---|
| **Entire Image** | Entire image, including all layers |
| **Selected Layer** | The layer selected in the Layers panel |
| **Adjustment Composite** | A selected adjustment layer including all layers below it |

### Adjustment preview

With **Preview** selected in any color/tonal adjustment dialog, "the Histogram panel shows how the adjustment affects the histogram" (original vs adjusted). CS6: adjustments made in the **Properties panel** "are automatically reflected in the Histogram panel" (`CLR-004`).

### Refresh and the cache warning

When a histogram is read from the cache, the **Cached Data Warning** icon appears; cache-based histograms render faster because they are built from a representative sample of the image's pixels. The original image is **cache level 1**; each higher level averages four adjacent pixels into one (1/4 the pixels). The maximum cache level (**2–8**) is set in `Edit > Preferences > Performance`. To refresh uncached:

- double-click anywhere in the histogram,
- click the **Cached Data Warning** icon,
- click the **Uncached Refresh** button, or
- choose **Uncached Refresh** from the panel menu.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Window > Histogram` | Panel | — | Opens the Histogram panel; Compact View default |
| Histogram tab | Tab | — | Opens when docked with other panels |
| Panel menu | Menu | — | Compact / Expanded / All Channels; Show Statistics; Show Channels In Color; Uncached Refresh |
| Channel menu | Combo | — | Channels / RGB / CMYK / Composite / Luminosity / Colors / alpha / spot |
| Source menu | Combo | — | Entire Image / Selected Layer / Adjustment Composite; Expanded only |
| Uncached Refresh | Button | — | Redraws from actual pixels |
| Cached Data Warning | Icon | click | Clears the warning via uncached refresh |
| Histogram area | 2-D control | hover / drag | Stats for a bin or a range |
| Histogram area | Gesture | double-click | Uncached refresh |
| `Edit > Preferences > Performance` | Pane | — | Max cache level 2–8 |
| Properties / adjustment dialogs | Context | — | Feed the original/adjusted preview |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| View | enum | Compact View | Compact / Expanded / All Channels | Panel menu |
| Channel | enum | Colors (RGB/CMYK) in Expanded | per-channel / RGB / CMYK / Composite / Luminosity / Colors / alpha / spot | Remembered when returning to Compact |
| Source | enum | Entire Image | Entire Image / Selected Layer / Adjustment Composite | Expanded only; hidden for single-layer docs |
| Show Statistics | bool | on (Expanded) | on / off | Statistics block visibility |
| Show Channels In Color | bool | off | on / off | Per-channel/composite color drawing |
| Cache Level | int (read-out) | 1 | 1 … max (2–8) | 1 = original; each level = 1/4 pixels |
| Max cache level | int (pref) | 4 | 2 … 8 | Performance preference; 4 is the Photoshop default, 1 disables caching |
| Hover level | int | — | 0 … (bins−1) | Statistics for the pointer bin |
| Selected range | range | — | bin a … bin b | Drag selection for range statistics |
| Bins | int (derived) | 256 (8-bit) | 256 / 65536 / display-mapped | Follows bit depth (`CLR-004`) |

## Algorithms & pipeline

All computation is `CLR-004`'s (`pictura_analysis::histogram`, `::stats`, `::cache`). The panel's own work is presentation and interaction:

- **Binning display** — map the computed bins onto the widget's x-axis; at 16-bit, reduce 65,536 bins to display resolution **without** reducing statistic precision; at 32-bit, use the document display/exposure mapping (`CLR-004`).
- **All-Channels layout** — stack one sub-histogram per channel; the channel menu drives only the topmost one.
- **Hover/range statistics** — convert pointer x → bin index, then read `HistogramStats::at_level` / `over_range`.
- **Color drawing** — a channel/composite drawn in color uses the channel's hue; "Show Channels In Color" is a render flag, not a recomputation.
- **Cache badge** — shown whenever the displayed data came from cache level > 1; any uncached-refresh path clears it.
- **Adjustment preview** — the panel listens for an in-progress adjustment (dialog Preview or Properties panel) and draws the original + adjusted pair; cancel restores the base.
- **Repaint** — coalesce histogram repaints to the UI frame rate; the uncached path is O(pixels) and must run off the UI thread, streamed by tile (`CLR-004`).

## Rust module mapping

- `pictura_analysis::histogram` — `Histogram { bins, channel, space }`, `compute(...)`, `composite(mode)`, `luminosity()` (`CLR-004`).
- `pictura_analysis::stats` — `HistogramStats { mean, std_dev, median, pixels, level, count, percentile, cache_level }`, `at_level(i)`, `over_range(a..b)` (`CLR-004`).
- `pictura_analysis::cache` — `ImagePyramid`, `decimate_2x2`, `max_level_from_prefs` (`CLR-004`).
- `pictura_analysis::histogram_panel` — `HistogramPanelModel { view, channel, source, show_stats, show_color, cached: bool, cache_level }`, plus `hit_test(x) -> BinIndex` and `range(a, b)`.
- `pictura_core::prefs` — max cache level, show-statistics/show-color defaults (`02-ui-ux/preferences.md`).
- `pictura_core::notify` — document/selection/state change subscription that triggers recompute.

Crossing types: `Histogram`, `HistogramStats`, `ChannelId`, `ReadoutSpace`, `ImagePyramid`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `HistogramPanel` | `QDockWidget` | Host; view switch, channel/source combos, refresh, stats |
| `HistogramView` | custom `QWidget` | Draws bins; hover/range selection; cached-warning badge |
| `HistogramStatsView` | `QFormLayout` | Mean / Std Dev / Median / Pixels / Level / Count / Percentile / Cache Level |
| `HistogramModel` | `QAbstractItemModel` | Bins + statistics; recompute on document/selection change |
| `HistogramChannelMenu` | `QComboBox` | Channel selection |
| `HistogramSourceMenu` | `QComboBox` | Source selection (Expanded only) |
| `HistogramRefreshButton` | `QToolButton` | Uncached refresh |
| `HistogramController` | `QObject` | Bridges document notifications to recompute; coalesces repaints |

Widgets, not QML: a compact, high-refresh instrument, consistent with `ARCH-003`. The histogram is a custom-painted `QWidget` (cheap, high-frequency) rather than a scene graph; the uncached computation runs on a worker thread and delivers bins to the widget.

## Data-model impact

- **Derived, never serialized.** The histogram and its cache/pyramid are recomputed from document state; the pyramid is runtime-only (`CLR-004`).
- **View/channel/source selection is UI state**, never persisted per document; the channel choice persists only within the panel session when returning to Compact.
- **Max cache level** is a preference (`02-ui-ux/preferences.md`).
- **Undo:** refreshing after undo must reflect restored pixels; neither refresh nor view change is a History state.
- **No new document fields.**

## Edge cases

- **8/16/32-bit** — 8-bit uses 256 bins; 16-bit must not lose statistic precision when the display is reduced; 32-bit floats need a bounded display histogram without integer clipping (`CLR-004`).
- **CMYK / Lab documents** — Actual channels follow the mode; Luminosity/Colors are offered only for RGB/CMYK.
- **Alpha / spot channels** — individual All-Channels histograms exclude alpha, spot, and masks, but the Channel menu can still select one in Expanded View; reproduce the asymmetry.
- **Single-layer documents** — the Source menu is hidden.
- **Empty / 1-pixel documents** — one populated bin; statistics must not divide by zero on an empty range.
- **Range drag inverting** — a right-to-left drag must normalize the range.
- **Cache at the preference maximum** — the Cache Level read-out must never exceed the 2–8 preference.
- **Stale cache** — after an edit the badge must appear until an uncached refresh; never silently show stale data as current.
- **No OpenGL** — histogram/readout compute on the CPU; results must match the GPU path within tolerance.
- **Huge (PSB) documents** — the uncached path streams tiles and the pyramid must not duplicate the full image beyond the memory budget.
- **High-frequency document edits** — recompute must be coalesced/cancellable so painting and histogram updates do not fight.
- **Adjustment cancelled** — the adjusted histogram overlay disappears and the original reappears.
- **Keyboard-only** — view switch, channel/source, refresh, and statistics toggle must be reachable without a pointer.

## Parity acceptance criteria

1. Given a document opened fresh, the Histogram panel is in Compact View with no controls or statistics.
2. Given an 8-bpc RGB document in Expanded View, the channel menu offers RGB, Composite, Luminosity, and Colors; selecting Luminosity matches the RGB-luminance histogram within tolerance.
3. Given a multilayered document, Source = Selected Layer shows that layer only; Source = Adjustment Composite includes all layers below the selected adjustment layer.
4. Given the statistics block and the pointer hovering a bin, Level and Count match that bin and Percentile equals the cumulative percentage at that level.
5. Given a cached histogram, the Cached Data Warning appears; clicking it, the Uncached Refresh button, double-clicking the histogram, or the menu's Uncached Refresh redraws from actual pixels and clears the warning.
6. Given the Performance preference max cache level N (2–8), the displayed Cache Level never exceeds N and level k has 1/4 the pixels of level k−1.
7. Given **Show Channels In Color**, channel/composite histograms render in color, and the color persists after switching to Compact View.
8. Given an adjustment dialog with Preview on, the panel shows the original and adjusted histograms and reverts on cancel.
9. Given All Channels View, the individual histograms omit alpha/spot/mask channels, and the channel menu affects only the topmost histogram.
10. Given a single-layer document, the Source menu is not shown.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — official CS6 Help reference (downloaded and text-extracted). Established: the histogram definition and shadow/midtone/highlight regions; the tonal-range/key-type description; `Window > Histogram` / Histogram tab and the Compact-View default; the anatomy labels for Expanded view (channel menu, panel menu, Uncached Refresh, Cached Data Warning, statistics) and for adjustment preview (original/adjusted, shadows/midtones/highlights); the three views and what each contains; the channel menu (individual/alpha/spot, RGB/CMYK/Composite, Luminosity, Colors) and the remembered-channel rule; the All-Channels topmost-histogram rule; Show Channels In Color and its persistence in Compact View; Show Statistics and the full statistic list; the Source menu (Entire Image / Selected Layer / Adjustment Composite) and its absence for single-layered documents; the Preview-adjustment behavior and the Adjustments/Properties panel note; the cache-level description (level 1 original, four adjacent pixels per level, 1/4 pixels) and the four uncached-refresh paths; the Performance preference maximum cache level 2–8. Bin/statistic formulas and the cache pyramid are specified in `CLR-004`.
- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — same file, "About histograms" / "How to read a histogram" figures (overexposed / properly exposed / underexposed examples).

Not used in this pass:

- `helpx.adobe.com` histogram pages (HTTP 403 from this environment); the archived CS6 PDF and `CLR-004` were used instead.

Fetched for this revision:

- `https://web.archive.org/web/20180801214950/https://helpx.adobe.com/photoshop/using/performance-preferences.html` — archived Adobe Performance-preferences page: "specify cache levels manually; the default value is 4"; up to eight levels; "Setting Cache Levels to 1 disables image caching."
- `http://www.photoshopforphotographers.com/3101-1901/Help_guide/tp/Histogram_palette.html` — Martin Evening CS6/CS5 support page: default composite RGB/CMYK view, expanded color-coded channel view and Luminosity view; the cache-level warning triangle and double-click / warning-triangle / refresh-button uncached paths. *(Page header is CS5; controls carry to CS6.)*
- `https://www.photoshopessentials.com/basics/photoshop-cs6-workspaces/` — CS6 Photography workspace supplies the Histogram panel (with Info and Actions).

## Open questions

- **All Channels View default channel and layout on first open.** *Resolves with:* a CS6 capture.
- **32-bit histogram binning and exposure mapping** — how unbounded HDR floats map to bins (`CLR-004`). *Resolves with:* a CS6 32-bit comparison.
- **Luminosity weights** (Rec.709 vs Adobe's display formula). *Resolves with:* a CS6 histogram comparison.
- **Range-selection statistics** — whether Percentile for a range is cumulative from the left even when the range starts mid-axis. *Resolves with:* a CS6 interaction test.
- **Whether the histogram tab/dock has its own shortcut.** *Resolves with:* `02-ui-ux/keyboard-shortcuts.md`.
