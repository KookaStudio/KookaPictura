# Black & White Adjustment

- **Spec ID:** `ADJ-012`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `Changed` — the Black & White adjustment was introduced in CS3; CS6 moves it to the Properties panel and, per the Help, adds a **Preset menu** in that panel (CS5 presented Black & White presets as tiles in the Adjustments panel). The CS6 "What's New" list also notes new **Gradient Map** presets, which are a separate adjustment (`ADJ-015`).
- **Depends on:** `ARCH-008` document-model, `ARCH-009` undo-history, `ARCH-006` gpu-rendering-pipeline, `01-architecture/color-management.md`, `04-image-ops/image-modes.md`, `04-image-ops/adjustments-overview.md`, `04-image-ops/adjustments/channel-mixer.md`, `05-layers/adjustment-layers.md`, `07-color-painting/color-models.md`, `07-color-painting/color-picker.md`, `10-workflow-io/presets-manager.md`.

> All module and widget names below are **design proposals**. The repository
> contains a working implementation; this file stays the behavioural contract
> for the feature, and `docs/dev/doc-code-parity-gaps.md` records what shipped
> and what did not. Facts not confirmed by a fetched CS6 source are marked
> *(inferred)*.

## CS6 behavior

**Black & White** "lets you convert a color image to grayscale while maintaining
full control over how individual colors are converted. You can also tint the
grayscale by applying a color tone to the image, for example to create a sepia
effect." The Help states it "functions like the Channel Mixer, which also
converts color images to monochrome while allowing you to adjust color channel
input."

- **Two invocation paths** — `Image > Adjustments > Black & White` (destructive;
  the Help warns it "discards image information") and `Layer > New Adjustment
  Layer > Black & White` (non-destructive, with a mask).
- **Automatic default** — on creation "Photoshop applies a default grayscale
  conversion." The default slider values are **Reds 40, Yellows 60, Greens 40,
  Cyans 60, Blues 20, Magentas 80** (percent), summing to 300 in the B&W model
  (see `## Algorithms & pipeline`).
- **Six color sliders** — **Reds, Yellows, Greens, Cyans, Blues, Magentas**, in
  that order. Dragging left darkens, right lightens the gray tone that color maps
  to. Each slider ranges **-200% to +300%**.
- **On-image adjustment tool** — "select the On-image adjustment tool and then
  click in the image. Drag left or right to modify the color slider for the
  predominant color at that location." The Help notes the sampled color can be a
  blend (clicking grass also selected yellows in its example).
- **Auto** — "Sets a grayscale mix based on the color values of the images,
  maximizing the distribution of gray values." It is a starting point the user
  can then tweak.
- **Preset menu (CS6)** — choose a predefined grayscale mix or a saved custom
  mix; `Save Black & White Preset` saves the current mix.
- **Preview** — deselecting Preview shows the original color image.
- **Tint** — a checkbox that applies a color tone to the grayscale result; click
  the color swatch to open the Color Picker. The legacy dialog exposed the tint
  as hue/saturation controls; the CS6 Properties panel exposes a colour swatch
  *(the swatch form is what the CS6 Help describes; the slider form is
  *(inferred)* for the legacy dialog only)*.
- **Reset** — resets all six sliders to the default grayscale conversion.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Image > Adjustments > Black & White` | Menu command | `Ctrl/Cmd+Shift+Alt/Option+B` *(inferred)* | Destructive direct edit. |
| `Layer > New Adjustment Layer > Black & White` | Menu command | — | Non-destructive adjustment layer with mask. |
| Adjustments / Properties panel | Panel | — | Six sliders, Auto, on-image tool, Tint, Preview, Reset, Preset menu. |
| Properties panel — Preset menu | Menu | — | CS6: predefined mixes, saved mixes, Save Black & White Preset. |
| Properties panel — Tint swatch | Color button | — | Opens the Color Picker. |
| `Channels`/`Layers` panels | Context | — | Adjustment layer applies below unless clipped/grouped. |
| Toolbox / Color Picker | Swatch | — | Foreground color is the tint reference when Tint is toggled. |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Reds | int (percent) | 40 | -200 … +300 | Brightness contribution of red. |
| Yellows | int (percent) | 60 | -200 … +300 | |
| Greens | int (percent) | 40 | -200 … +300 | |
| Cyans | int (percent) | 60 | -200 … +300 | |
| Blues | int (percent) | 20 | -200 … +300 | |
| Magentas | int (percent) | 80 | -200 … +300 | |
| Auto | button | — | — | Computes a mix from image color statistics. |
| Tint | bool | Off *(inferred)* | on / off | Applies a colour tone to the gray. |
| Tint colour | RGB/CMYK/Lab | — | any pickable colour | Color Picker; legacy hue 0–360 / sat 0–100 *(inferred)*. |
| Preset | named mix | Default | shipped + user-saved | CS6 Properties-panel menu. |
| Preview | bool | On *(inferred)* | on / off | Toggles original-color preview. |
| On-image tool | toggle | Off | — | Click-drag modifies the predominant color's slider. |

### Reported preset names (unverified)

Community sources list the shipped B&W mixes as: **Default, Blue Filter, Darker,
Green Filter, High Contrast Blue Filter, High Contrast Green Filter, High
Contrast Red Filter, Infrared, Lighter, Maximum Black, Maximum White, Neutral
Density, Orange Filter, Red Filter, Yellow Filter**. The CS6 Help confirms a
Preset menu exists but does **not** enumerate these names; treat the list as
secondary (see `## Open questions`).

## Algorithms & pipeline

Adobe's Black & White operator is closed. The reference model below is the
community analysis published on Stack Overflow and cross-checked
against the CS6 default values *(community source, not Adobe)*.

### Reference conversion

Let each pixel be `(r,g,b) ∈ [0,255]` and the six slider values be fractions
`r_w, y_w, g_w, c_w, b_w, m_w` (default `0.40, 0.60, 0.40, 0.60, 0.20, 0.80`).
The transformation decomposes the color into a neutral component plus a
hue-sector pair, then applies the corresponding weights:

```
gray = min(r, g, b)
r -= gray; g -= gray; b -= gray
if r == 0:                      # cyan sector
    cyan = min(g, b)
    g -= cyan; b -= cyan
    gray += cyan*c_w + g*g_w + b*b_w
elif g == 0:                    # magenta sector
    magenta = min(r, b)
    r -= magenta; b -= magenta
    gray += magenta*m_w + r*r_w + b*b_w
else:                           # yellow sector
    yellow = min(r, g)
    r -= yellow; g -= yellow
    gray += yellow*y_w + r*r_w + g*g_w
out = clamp(round(gray), 0, 255)
```

Properties of this model that match observed CS6 behaviour:

- The result is a pure gray (R = G = B = out), so the adjustment is a luminance
  remap, not a chroma-preserving conversion.
- `min(r,g,b)` acts as the "neutral" lightness; the two remaining channels drive
  the primary/secondary weights.
- Because three weights of 1.0 sum to 3.0 (300%), the default `40+60+40+60+20+80
  = 300` reproduces neutral colors correctly, and the -200…+300 slider range lets
  a user zero or double a channel's contribution. The **"sum to 100" rule of
  thumb applies to how the slider weights are perceived, not to a hard sum**;
  Adobe does not enforce a total.
- The algorithm is per-pixel and order-independent, so it is trivially tileable.

The exact Adobe operator may differ in the neutral decomposition (e.g. using
`max`, HSL lightness, or a luminance-weighted neutral). See
`## Open questions`.

### Auto mix

The Help says Auto "maximizes the distribution of gray values" based on image
color statistics. A behavioural-parity proposal: build the hue-sector histogram,
compute the mean contribution of each of the six sectors, and set each weight so
the resulting gray histogram spans the tonal range (e.g. gray-component
normalization). The exact algorithm is closed — **behavioural parity only,
algorithm TBD**.

### Tint

Tint maps the resulting gray `G` onto the chosen tint colour's hue and
saturation. The Help's contract: the image is tinted toward the tint colour while
lightness structure is preserved. A reference model converts the tint colour to
HSL and outputs `HSL(H_tint, S_tint, G)` per pixel (or applies the tint as a
solid-color multiply/overlay at full opacity). The exact model is *(inferred)*.

### Placement in the pipeline

- Runs on the composite in the document working space; outputs a single gray
  value written to R = G = B.
- In an adjustment-layer stack it is a normal pixel operator feeding the layer's
  blend mode, opacity, fill, and mask.
- It is **not** the same code path as `ADJ-014` Channel Mixer Monochrome,
  although the two are conceptually related and the Help cross-references them.

## Rust module mapping

Proposals.

- `pictura_ops::adjust::black_white` — `BlackWhiteSettings { weights: [i16;6],
  tint: Option<Color>, preview: bool }` and the `PixelEdit` implementation.
  `weights` order fixed as `[red, yellow, green, cyan, blue, magenta]`.
- `pictura_ops::adjust::black_white::auto` — `auto_mix(stats) -> [i16;6]`; the
  closed Auto heuristic, isolated so it can be calibrated without touching the
  operator.
- `pictura_ops::adjust::hue_sector` — shared decomposition of an RGB pixel into
  `(neutral, sector, pair)`; reused by `ADJ-014` and by `Auto`.
- `pictura_color::hsl` — tint math and the colour-space bridge.
- `pictura_core::command` — `BlackWhiteCommand { target, settings, mask,
  dirty_rect }`.
- `pictura_render::adjust` — optional GPU variant.

Crossing types: `AdjLayerId`, `LayerId`, `ColorSpace`, `Color`, `Rect`,
`TileDelta`, `BlackWhiteSettings`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `BlackWhiteEditor` | `QWidget` | Six scrub sliders (named colourized tracks), Auto, Tint check + swatch, Preview, Reset, Preset menu. |
| `ColorSlider` | custom `QWidget` | A labeled horizontal slider whose track shows the source colour family. |
| `OnImageSliderController` | `QObject` | Maps canvas click-drag to the predominant-color slider. |
| `ColorPickerDialog` | dialog | Tint colour selection; reused from `07-color-painting/color-picker.md`. |
| `AdjustmentPropertiesPanel` | `QStackedWidget` | Reused host. |
| `PresetMenu` | `QMenu` | Ships the reported B&W mix presets; Save Black & White Preset. |

## Data-model impact

- **Adjustment-layer node.** Stores `BlackWhiteSettings` (six weights, tint
  colour, preview flag) and a mask.
- **PSD serialization.** Adjustment-layer content via the layer's
  additional-layer-information block; exact tags *(inferred)*, tracked in
  `01-architecture/file-formats.md`.
- **Undo.** Destructive: one history state per committed edit. Adjustment layer:
  one per settings change.
- **No new channels / no alpha change.** The output intentionally discards
  chroma, but only downstream of the adjustment; the underlying pixel data on a
  non-destructive adjustment layer is untouched.

## Edge cases

- **8/16/32-bit.** Black & White is **not** in the CS6 32-bpc supported list, so
  at 32 bpc it must be unavailable; 8/16-bit supported.
- **CMYK.** Works on CMYK (the Help positions B&W as the friendly alternative to
  Channel Mixer, which is CMYK-capable); the CMYK→sector decomposition must be
  defined and tested.
- **Lab.** Same concern; whether the six ranges are on Lab-derived or
  RGB-bridged hues is unverified.
- **Grayscale / Bitmap / Indexed.** No color to mix; expected unavailable/inert.
- **Tint + Preview off** must show the original colour while still remembering the
  tint state.
- **Weights far negative/positive** may drive `gray` out of range; clamp at the
  document bit depth; do not wrap.
- **On-image sampling across a hue boundary** should follow the "predominant
  color" rule; exact sector-selection rule *(inferred)*.
- **Adjustment layer merged** rasterizes to RGB gray; parity requires the merged
  result to equal the previewed composite.
- **1-px / empty / huge documents.** Tile-based, no full-canvas scratch.
- **Undo mid-drag.** Coalesce scrub into one committed history state.

## Parity acceptance criteria

1. Given an RGB document with the default weights, neutral grays map to
   themselves within one quantization step.
2. Given the default weights, a pure red patch maps to a value consistent with
   `r_w`; changing only the Reds slider changes only red-family pixels' gray
   tone.
3. Given the Reds slider at -200, red patches become black; at +300 they become
   white (clamped), with no wrap-around.
4. Given **Auto**, the resulting gray histogram spans a wider distribution than a
   fixed default on a low-contrast colourful test image.
5. Given **Tint** on with a sepia colour, output pixels are gray-scale ordered and
   tinted toward the chosen hue within tolerance.
6. Given **Preview** off, the document shows the original colour.
7. Given a CMYK document, the conversion completes without out-of-range values
   and is deterministic.
8. Given a 32-bpc document, the adjustment is unavailable.
9. Given a committed edit, History shows exactly one new state and undo restores
   the prior pixels bit-exactly.
10. Given an adjustment-layer instance, opacity, blend mode, and mask modulate
    the result per `05-layers/adjustment-layers.md`.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  official CS6 Help reference; downloaded and text-extracted. Establishes:
  "Convert a color image to black and white" (p. 260): the purpose statement;
  the tint/sepia capability; the "functions like the Channel Mixer" cross-ref;
  the destructive vs adjustment-layer paths; that a default grayscale conversion
  is applied; the CS6 Preset menu; the Auto description ("maximizing the
  distribution of gray values"); the color sliders (darken left / lighten right);
  the on-image adjustment tool; the Reset button; and the Preview option.
  Also establishes that the CS6 32-bpc supported-adjustment list does **not**
  name Black & White.
- `https://www.photoshopessentials.com/photo-editing/black-and-white-cs3` —
  community tutorial (CS3, the version B&W was introduced). Establishes the six
  slider names and order (Reds, Yellows, Greens, Cyans, Blues, Magentas), the
  observed default values ("Reds set to 40%, Yellows set to 60%"), the
  click-drag on-image behaviour, and the banding warning. Secondary source.
- `https://api.stackexchange.com/2.3/questions/55185251/answers?site=stackoverflow&filter=withbody&order=desc&sort=votes`
  — Stack Overflow API for question 55185251, "What is the algorithm behind
  Photoshop's Black and White adjustment layer". Establishes the reference
  decomposition algorithm (neutral = `min(r,g,b)`, hue-sector weights) used in
  `## Algorithms & pipeline`. Community analysis, not Adobe.
- `https://stackoverflow.com/questions/55185251/what-is-the-algorithm-behind-photoshops-black-and-white-adjustment-layer` —
  canonical question URL (HTML page returned HTTP 403 to direct fetch; the
  answer body was retrieved through the Stack Exchange API above).

Not fetched (HTTP 403 from this environment): `helpx.adobe.com` "Convert color
images to black and white" page.

## Open questions

- **Shipped preset names and their exact weights.** The list above is community
  knowledge; the CS6 Help only says a Preset menu exists. Resolves with: a CS6
  Properties-panel capture and per-preset slider readout.
- **Neutral decomposition.** Whether CS6 uses `min(r,g,b)`, `max`, HSL
  lightness, or a luminance-weighted neutral. Resolves with: a pixel-level
  comparison against CS6 output on a hue/saturation grid.
- **Auto algorithm** is closed. Resolves with: a CS6 calibration over standard
  test images; ship as behavioural parity until then.
- **Exact tint math** (HSL set vs multiply vs overlay) and whether tint
  saturation is user-adjustable in the CS6 Properties panel. Resolves with: a
  CS6 panel capture.
- **Default / range of the six sliders** — defaults are confirmed for Reds and
  Yellows only; the -200…+300 range and the other four defaults are from
  community sources. Resolves with: a CS6 options capture.
- **CMYK/Lab conversion path.** Resolves with: mode-specific CS6 tests.
- **PSD keys** for the adjustment-layer payload. Resolves with:
  `01-architecture/file-formats.md`.
