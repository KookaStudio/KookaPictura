# Photo Filter Adjustment

- **Spec ID:** `ADJ-013`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `Changed` — the adjustment is long-standing (introduced in CS2); CS6 moves its UI to the Properties panel. The CS6 Help describes the same preset set and the Density / Preserve Luminosity controls. No new filters are documented in the CS6 "What's New" list.
- **Depends on:** `ARCH-008` document-model, `ARCH-009` undo-history, `ARCH-006` gpu-rendering-pipeline, `01-architecture/color-management.md`, `04-image-ops/image-modes.md`, `04-image-ops/adjustments-overview.md`, `05-layers/adjustment-layers.md`, `07-color-painting/color-models.md`, `07-color-painting/color-picker.md`, `10-workflow-io/presets-manager.md`.

> All module and widget names below are **design proposals**. The repository
> contains a working implementation; this file stays the behavioural contract
> for the feature, and `docs/dev/doc-code-parity-gaps.md` records what shipped
> and what did not. Facts not confirmed by a fetched CS6 source are marked
> *(inferred)*.

## CS6 behavior

**Photo Filter** "mimics the technique of placing a colored filter in front of
the camera lens to adjust the color balance and color temperature of the light
transmitted through the lens and exposing the film." It also applies a hue
adjustment from a preset or a custom colour.

- **Two invocation paths** — `Image > Adjustments > Photo Filter` (destructive;
  the Help warns it "discards image information") and `Layer > New Adjustment
  Layer > Photo Filter` (non-destructive, with a mask).
- **Filter colour source** — either a **preset** (the **Filter** option) or a
  **custom** colour (the **Color** option, opening the Adobe Color Picker via the
  colour square).
- **Preset families (CS6 Help wording):**
  - **Warming Filter (85)** and **Cooling Filter (80)** — "Color conversion
    filters that tune the white balance in an image." Warming (85) makes colors
    warmer when the shot was taken under bluish/high-colour-temperature light;
    Cooling (80) makes them bluer when shot under yellowish/low-temperature
    light. **Warming Filter (LBA)** and **Cooling Filter (LBB)** are the
    corresponding Kodak Wratten light-balancing variants in the same group.
  - **Warming Filter (81)** and **Cooling Filter (82)** — "light-balancing filters
    for minor adjustments"; 81 warms (more yellow), 82 cools (bluer).
  - **Individual Colors** — "Apply a hue adjustment to the image depending on the
    color preset you choose." Used to neutralize a cast (pick the complementary
    colour) or for special effects; the Help names **Underwater** as the
    greenish-blue example.
- **Density** — "To adjust the amount of color applied to the image, use the
  Density slider or enter a percentage in the Density box. A higher density
  results in a stronger color adjustment." Default is **25%** *(inferred;
  community source)*.
- **Preserve Luminosity** — "If you don't want the image darkened by adding the
  color filter, be sure that the Preserve Luminosity option is selected."
  Default is **On** *(inferred; community source)*.
- **Preview** — "Make sure that Preview is selected to view the results."
- **Presets** — the adjustment settings are saveable like the other adjustments.
  The Help's preset-enumeration line does not list Photo Filter in the named
  Properties-panel Preset menu group.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Image > Adjustments > Photo Filter` | Menu command | — | Destructive direct edit to the active layer. |
| `Layer > New Adjustment Layer > Photo Filter` | Menu command | — | Non-destructive adjustment layer with mask. |
| Adjustments / Properties panel | Panel | — | Filter/Color choice, filter list, Density, Preserve Luminosity, Preview. |
| Properties panel — Filter dropdown | Combo | — | Preset list (Warming/Cooling/Individual Colors). |
| Properties panel — Color square | Color button | — | Opens the Adobe Color Picker for a custom filter. |
| Properties panel — Density | slider + field | — | 1–100% (default 25%). |
| Toolbox / Color Picker | Swatch | — | Reference for the custom-filter colour. |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Filter source | radio | Filter | Filter / Color | "Filter" = preset; "Color" = custom picker colour. |
| Filter preset | enum | Warming Filter (85) *(inferred)* | Warming 85 / Warming LBA / Cooling 80 / Cooling LBB / Warming 81 / Cooling 82 / Individual Colors* | Full list not enumerated in the CS6 Help; "*" names reported by community sources. |
| Custom filter colour | Colour | — | any RGB/CMYK/Lab pickable colour | Adobe Color Picker. |
| Density | int (percent) | 25 *(inferred)* | 1 … 100 *(inferred)* | Strength of the applied tone. |
| Preserve Luminosity | bool | On *(inferred)* | on / off | Keeps the image from being darkened by the filter. |
| Preview | bool | On *(inferred)* | on / off | Live preview. |

## Algorithms & pipeline

Adobe's operator is closed. The CS6 Help fixes the observable contract (a tinted
filter simulating an optical filter, a strength control, optional luminosity
preservation); the math below is a behavioural-parity model *(inferred)*.

### Filter as a colour transform

A photo filter is a tint applied toward the filter colour. The natural density
model is a linear interpolation between the input colour `C` and the filter
colour `F`, with Density `d ∈ [0,1]` as the interpolation weight:

`C' = C · (1 − d) + F · d`

A multiplicative model (`C' = C · F` normalised) reproduces the "colour
conversion filter" behaviour of the 85/80 family more closely, while the linear
model reproduces the "light-balancing" 81/82 family. The Help does not
distinguish them, and the exact model is *(inferred)*. The reference proposal is
the linear interpolation with a per-channel multiply option; calibration against
CS6 output decides which is shipped per filter family.

### Preset filter colours

The named presets are Kodak Wratten filter equivalents:

| Preset | Family | Expressed as |
|---|---|---|
| Warming Filter (85) | colour conversion | amber/orange |
| Warming Filter (LBA) | light balancing | amber |
| Cooling Filter (80) | colour conversion | blue |
| Cooling Filter (LBB) | light balancing | blue |
| Warming Filter (81) | light balancing | yellow/amber |
| Cooling Filter (82) | light balancing | blue |
| Individual Colors | hue adjustment | per-colour presets (e.g. Underwater = greenish blue) |

The exact sRGB values of each Wratten filter are **not** published by Adobe in
the CS6 Help; they must be sourced from the Wratten filter specification or
fitted to CS6 output *(inferred)*.

### Preserve Luminosity

When on, apply the filter to the chroma while keeping the per-pixel luminance
unchanged (compute the input luminance `Y`, apply the tint, then rescale/re-bias
toward the original `Y`). The Help's framing ("if you don't want the image
darkened") confirms a luminance-restoring step. The exact formula and the
luminance space are *(inferred)*. When off, the tint is allowed to darken the
result, which is the classic look of a physical filter.

### Placement in the pipeline

- Composite read; writes the active layer (destructive) or feeds the
  adjustment-layer compositor (non-destructive).
- Runs in the document working space; the filter colour is converted into that
  space, so a CMYK document applies a CMYK-filter equivalent.
- The operator is per-pixel and order-independent; tileable.

## Rust module mapping

Proposals.

- `pictura_ops::adjust::photo_filter` — `PhotoFilterSettings { source:
  FilterColorSource, preset: Option<PhotoFilterPreset>, custom: Option<Color>,
  density: f32, preserve_luminosity: bool }`, `FilterColorSource { Preset,
  Custom }`, and the `PixelEdit` implementation.
- `pictura_ops::adjust::photo_filter::presets` — `PhotoFilterPreset` enum and a
  `filter_color() -> Color` table for the Wratten equivalents. The table is data,
  so it can be recalibrated without code changes.
- `pictura_color::luma` and `pictura_color::convert` — luminance and colour-space
  bridge shared with `ADJ-011`.
- `pictura_core::command` — `PhotoFilterCommand { target, settings, mask,
  dirty_rect }`.
- `pictura_render::adjust` — optional GPU variant; Photo Filter is in the 32-bpc
  supported list, so the GPU path must run in `f32`.

Crossing types: `AdjLayerId`, `LayerId`, `ColorSpace`, `Color`, `Rect`,
`TileDelta`, `PhotoFilterSettings`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `PhotoFilterEditor` | `QWidget` | Filter/Color radio, preset combo, colour swatch, Density scrub, Preserve Luminosity, Preview. |
| `FilterPresetCombo` | `QComboBox` | Lists Warming/Cooling/Individual Colors with colour chips. |
| `ColorPickerDialog` | dialog | Custom filter colour; reused. |
| `AdjustmentPropertiesPanel` | `QStackedWidget` | Reused host. |
| `AdjustmentLayerModel` | `QAbstractItemModel` | Reads/writes `PhotoFilterSettings`. |

## Data-model impact

- **Adjustment-layer node.** Stores `PhotoFilterSettings` (source, preset id or
  custom colour, density, preserve-luminosity) plus a mask.
- **PSD serialization.** Adjustment-layer content via the layer's
  additional-layer-information block; exact tags *(inferred)*, tracked in
  `01-architecture/file-formats.md`.
- **Preset table as data.** The Wratten colour table must be a versioned,
  loadable resource so it can be calibrated; it is not per-document state.
- **Undo.** Destructive: one history state per committed edit. Adjustment layer:
  one per settings change.
- **No new channels / no alpha change.**

## Edge cases

- **8/16/32-bit.** Photo Filter is in the CS6 32-bpc supported list; it must work
  at all three depths without integer quantization at 32 bpc.
- **CMYK.** Filter colours converted into CMYK; density semantics in an
  ink-subtractive space must be defined (interpolating toward an ink mix).
- **Lab.** Filter colour converted to `a`/`b`; Preserve Luminosity manipulates
  `L`.
- **Grayscale / Bitmap / Indexed.** A colour filter on a grayscale image either
  tints (Grayscale supports colour?) or is inert; expected unavailable/inert for
  Bitmap/Indexed — define and test.
- **Density 0** = no-op; must not create a history state.
- **Density 100** fully replaces colour with the filter; with Preserve
  Luminosity off this can collapse shadow/highlight detail — reproduce, do not
  "fix".
- **Custom colour equal to neutral gray** under a multiplicative model must not
  darken the image; guard the normalization.
- **1-px / empty / huge documents.** Tile-based; no full-canvas scratch.
- **GPU unavailable.** CPU/GPU agreement within tolerance.
- **Undo mid-drag.** Coalesce density scrubbing into one committed history state.

## Parity acceptance criteria

1. Given a neutral gray image and Warming Filter (85) at 25% density with
   Preserve Luminosity on, the result is warmer (R > B) with luminance unchanged
   within tolerance.
2. Given Cooling Filter (80) and Warming Filter (85) at equal density, the two
   produce opposite-direction hue shifts on the same pixel.
3. Given Density 0, the output equals the input bit-exactly and no history state
   is created.
4. Given Preserve Luminosity on, a large-density filter leaves per-pixel
   luminance within a small tolerance of the input; with it off, the mean
   luminance drops (a physical filter darkens).
5. Given the same tint colour supplied as a preset and as a custom colour of the
   same value, the results match within tolerance.
6. Given a Lab and a CMYK document, the filter completes without out-of-range
   values and is deterministic.
7. Given a 32-bpc document, the filter is available and applied without integer
   quantization.
8. Given a committed edit, History shows exactly one new state and undo restores
   the prior pixels bit-exactly.
9. Given an adjustment-layer instance, opacity, blend mode, and mask modulate the
   result per `05-layers/adjustment-layers.md`.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  official CS6 Help reference; downloaded and text-extracted. Establishes:
  "Change the color balance using the Photo Filter command" (pp. 284–285): the
  film-filter analogy and colour-temperature purpose; the destructive vs
  adjustment-layer paths; the Filter vs Color (custom, Adobe Color Picker)
  choice; the Warming 85 / Cooling 80 colour-conversion description and their
  temperature behaviour; Warming 81 / Cooling 82 light-balancing description;
  the Individual Colors hue-adjustment family and the Underwater example; the
  Density slider/box; Preserve Luminosity ("if you don't want the image
  darkened…"); and Preview. Also establishes that Photo Filter is in the CS6
  32-bpc supported-adjustment list.
- `https://www.tourboxtech.com/en/news/photoshop-photo-filters.html` —
  community tutorial. Establishes the Density default of 25%, that Preserve
  Luminosity is recommended on, the Filter/Color controls, and that the filter
  dropdown contains 21 presets grouped as Warming/Cooling and colour names.
  Secondary source.

Not fetched (HTTP 403 from this environment): `helpx.adobe.com` Photo Filter
pages.

## Open questions

- **Exact Wratten colour values** for each named preset are not published. Resolves
  with: the Wratten filter specification and/or a CS6 sample-pixel fit.
- **Interpolation vs multiply model** per filter family. Resolves with: density
  sweeps against CS6 output.
- **Full preset list** (the Help names only a subset; a community source claims
  21). Resolves with: a CS6 Filter-dropdown capture.
- **Preserve Luminosity default and exact renormalization.** Resolves with: a CS6
  first-run capture and a luminance calibration.
- **Density minimum** — is it 1 or 0? Resolves with: a CS6 panel capture.
- **Whether Photo Filter participates in the CS6 Properties-panel Preset menu**
  (the Help's preset list omits it). Resolves with: a CS6 panel capture.
- **CMYK/Lab filter-colour conversion rules.** Resolves with: mode-specific CS6
  tests.
- **PSD keys** for the adjustment-layer payload. Resolves with:
  `01-architecture/file-formats.md`.
