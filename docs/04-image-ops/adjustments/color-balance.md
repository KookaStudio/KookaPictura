# Color Balance Adjustment

- **Spec ID:** `ADJ-011`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `Changed` — the adjustment is long-standing; CS6 moves its UI from the CS5 Adjustments panel to the Properties panel and keeps the Preserve Luminosity option. CS6 does not add new sliders here (unlike Black & White, which gains a preset menu).
- **Depends on:** `ARCH-008` document-model, `ARCH-009` undo-history, `ARCH-006` gpu-rendering-pipeline, `01-architecture/color-management.md`, `04-image-ops/image-modes.md`, `04-image-ops/adjustments-overview.md`, `05-layers/adjustment-layers.md`, `07-color-painting/color-models.md`, `10-workflow-io/presets-manager.md`.

> All module and widget names below are **design proposals**. The repository
> contains a working implementation; this file stays the behavioural contract
> for the feature, and `docs/dev/doc-code-parity-gaps.md` records what shipped
> and what did not. Facts not confirmed by a fetched CS6 source are marked
> *(inferred)*.

## CS6 behavior

**Color Balance** "changes the overall mixture of colors in an image for
generalized color correction." It is the classic three-pair complementary
balance control (cyan↔red, magenta↔green, yellow↔blue) applied separately to
three tonal bands.

- **Composite-channel only** — the Help is explicit: "Make sure that the
  composite channel is selected in the Channels panel. This command is available
  only when you're viewing the composite channel." It is not applied to an
  individual channel.
- **Two invocation paths** — `Image > Adjustments > Color Balance` (destructive;
  the Help warns it "discards image information") and `Layer > New Adjustment
  Layer > Color Balance` (non-destructive, with a mask).
- **Tonal range selector** — **Shadows**, **Midtones**, or **Highlights** focuses
  the change on that band. Each band keeps its own three slider values, so up to
  nine values are stored.
- **Three sliders per band** — **Cyan–Red**, **Magenta–Green**, **Yellow–Blue**.
  Dragging toward a color increases it; dragging away decreases it. Values range
  **-100 to +100**.
- **Per-channel readout** — "The values above the color bars show the color
  changes for the red, green, and blue channels. (For Lab images, the values are
  for the A and B channels.)" This is the per-channel display the CS6 Help
  documents; the Lab case exposes only two effective axes.
- **Preserve Luminosity** — an option that "prevent[s] changing the luminosity
  values in the image while changing the color. This option maintains the tonal
  balance in the image." Default state is *(inferred)* On.
- **Presets** — Color Balance settings can be saved and reapplied, but the Help's
  preset-enumeration list does **not** name Color Balance among the adjustments
  with a Properties-panel Preset menu; the settings are still saveable via the
  load/save path. See `## Open questions`.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Image > Adjustments > Color Balance` | Menu command | `Ctrl/Cmd+B` | Destructive direct edit to the active layer. |
| `Layer > New Adjustment Layer > Color Balance` | Menu command | — | Non-destructive adjustment layer with mask. |
| Adjustments / Properties panel | Panel | — | Tonal-range selector, three sliders, Preserve Luminosity. |
| Properties panel — tonal range | Radio/segmented | — | Shadows / Midtones / Highlights. |
| Properties panel — color bars | Custom slider | — | Cyan–Red, Magenta–Green, Yellow–Blue with per-channel value readout. |
| `Channels` panel | Context | — | Composite channel must be active; command is disabled otherwise. |
| `Image > Adjustments` | Menu | — | Present as an item; direct path. |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Tonal range | enum | Midtones *(inferred)* | Shadows / Midtones / Highlights | Each range stores its own slider triple. |
| Cyan–Red (per range) | int | 0 | -100 … +100 | Positive toward red, negative toward cyan. |
| Magenta–Green (per range) | int | 0 | -100 … +100 | Positive toward green, negative toward magenta. |
| Yellow–Blue (per range) | int | 0 | -100 … +100 | Positive toward blue, negative toward yellow. |
| Preserve Luminosity | bool | On *(inferred)* | on / off | Keeps `Y` (luminance) effectively unchanged. |
| Displayed R/G/B values | read-only | — | same -100 … +100 | Shown above the bars; for Lab these are the A/B values. |

## Algorithms & pipeline

Adobe's exact operator is closed. The CS6 Help fixes the observable contract
(three tonal bands, three complementary axes, optional luminosity preservation);
the math below is a behavioural-parity model *(inferred)*.

### Tonal band weights

For a pixel with luminance `Y ∈ [0,1]` (in the document working space,
gamma/YCbCr `Y` or Lab `L`), each band contributes a smooth weight:

- **Shadows**: peaks near `Y = 0` and falls toward midtones.
- **Midtones**: peaks near `Y ≈ 0.5`.
- **Highlights**: peaks near `Y = 1`.

The CS6 Help does not publish the curves. A reference implementation uses three
overlapping Gaussian/raised-cosine windows; the exact shape is a calibration
parameter behind `## Parity acceptance criteria`, not a parity assertion.

### Colour shift

For each output channel `c ∈ {R,G,B}`:

`out_c = clamp(in_c + Σ_band band_weight(Y) · slider_c_band · k)`

where `k` converts the -100…+100 range to a channel delta. Whether the shift is
applied to gamma-encoded values (classic behaviour) or linear values is
*(inferred)*; the gamma-encoded model is the closer match to historical
Photoshop output. The Help's "values for the red, green, and blue channels"
confirms a per-channel additive change.

**Lab documents.** The Help states that for Lab images the displayed values are
the **A and B** channels. The sane model is to add the Cyan–Red and Yellow–Blue
deltas to `a` and `b` respectively (with Magenta–Green folding onto the
`a`/`b` plane or being inert — see Open questions).

### Preserve Luminosity

When on, compute the luminance `Y` of the input pixel, apply the colour shift,
then scale/re-bias the result so its luminance matches the input `Y` (a
chroma-only edit). The exact renormalization (Rec. 601 vs 709 vs Lab `L`, and
whether clipping is applied before or after) is *(inferred)*. When off, the
colour shift is free to move luminance, which is why the Help frames it as a
"tonal balance" control.

### Placement in the pipeline

- Composite-channel read; writes the active layer (destructive) or produces the
  adjustment-layer output (non-destructive).
- Because it needs the composite rather than a single channel, the operator runs
  after the document is composited to the working space and before the
  adjustment layer's blend/mask.

## Rust module mapping

Proposals.

- `pictura_ops::adjust::color_balance` — `ColorBalanceSettings { shadows: [i16;3],
  midtones: [i16;3], highlights: [i16;3], preserve_luminosity: bool,
  active_range: ToneRange }` and the `PixelEdit` implementation.
- `pictura_ops::adjust::tonal_bands` — `ToneRange { Shadows, Midtones,
  Highlights }` and `band_weight(luma) -> [f32;3]`; shared with
  `04-image-ops/adjustments/levels.md`, `curves.md`, and the dodge/burn tools.
- `pictura_color::luma` — workspace luminance (`Y`) and the Lab `a`/`b` bridge;
  single source of the Rec.-matrix choice.
- `pictura_core::command` — `ColorBalanceCommand { target, settings, mask,
  dirty_rect }`.
- `pictura_render::adjust` — optional GPU variant.

Crossing types: `AdjLayerId`, `LayerId`, `ColorSpace`, `ToneRange`, `Rect`,
`TileDelta`, `ColorBalanceSettings`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `ColorBalanceEditor` | `QWidget` | Tonal-range selector, three colour-pair sliders, Preserve Luminosity, Reset. |
| `BalanceSlider` | custom `QWidget` | One complementary-axis slider with a coloured gradient track and a numeric scrub field above it. |
| `TonalRangeSelector` | `QButtonGroup` | Shadows / Midtones / Highlights radio group. |
| `AdjustmentPropertiesPanel` | `QStackedWidget` | Reused host; swaps to `ColorBalanceEditor`. |
| `AdjustmentLayerModel` | `QAbstractItemModel` | Reads/writes `ColorBalanceSettings` on the selected layer. |
| `PresetMenu` | `QMenu` | Optional; Color Balance is not in the CS6 named-preset list. |

Widgets over QML for the same reason as `ADJ-010`.

## Data-model impact

- **Adjustment-layer node.** Stores `ColorBalanceSettings` (nine slider values +
  Preserve Luminosity + last-selected tonal range) and a mask.
- **PSD serialization.** Adjustment-layer content goes into the layer's
  additional-layer-information block; exact tags *(inferred)* and tracked in
  `01-architecture/file-formats.md`.
- **Undo.** Destructive: one history state per committed edit (tile deltas).
  Adjustment layer: each settings change is a history state.
- **No new channels / no alpha change.**

## Edge cases

- **Individual channel selected in Channels panel.** The command must be disabled
  or transparently switch to the composite; the Help requires the composite.
- **Lab.** Only two meaningful chroma axes; the mapping of the three sliders onto
  `a`/`b` must be explicit and tested.
- **CMYK.** The three axes map onto C/M/Y with K held; the model must be defined
  (see Open questions).
- **8/16/32-bit.** Color Balance is **not** in the CS6 32-bpc supported list, so
  at 32 bpc it must be unavailable; 8/16-bit supported.
- **Preserve Luminosity on + extreme shift** must not introduce hue-cycling or
  division by near-zero luminance; guard the renormalization.
- **Grayscale.** No hue axes; the adjustment is either inert or maps to
  lightness — define and test.
- **Bitmap / Indexed / Multichannel.** Expected unavailable/inert.
- **1-px / empty / huge documents.** Dirty-rect tile processing; no full-canvas
  scratch. PSB safe.
- **GPU unavailable.** CPU/GPU agreement within tolerance.
- **Undo mid-drag.** Coalesce scrub into a single committed history state.

## Parity acceptance criteria

1. Given a neutral gray image, a Midtones Cyan–Red shift leaves the extreme black
   and white ends nearly unchanged while shifting the mid-gray toward red.
2. Given the same slider value applied separately to Shadows, Midtones, and
   Highlights, the measured channel change peaks in the corresponding tonal band.
3. Given **Preserve Luminosity** on, a large color shift leaves per-pixel
   luminance within a small tolerance of the input; with it off, luminance moves
   measurably.
4. Given a Lab document, the Help-documented A/B readout matches the applied
   chroma change within tolerance.
5. Given a single channel is selected in the Channels panel, the command is
   disabled (or forces the composite) and no per-channel edit occurs.
6. Given a 32-bpc document, the adjustment is unavailable.
7. Given a committed edit, History shows exactly one new state and undo restores
   the prior pixel values bit-exactly at the document bit depth.
8. Given an adjustment-layer instance, opacity, blend mode, and mask modulate the
   result per `05-layers/adjustment-layers.md`.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  official CS6 Help reference; downloaded and text-extracted. Establishes:
  "Apply the Color Balance adjustment" (p. 284): the composite-channel-only
  requirement; the destructive and adjustment-layer paths; the Shadows /
  Midtones / Highlights selector; the Preserve Luminosity wording ("maintains
  the tonal balance"); the Cyan–Red / Magenta–Green / Yellow–Blue sliders; the
  -100…+100 range; and the per-channel readout including the Lab A/B note.

Not fetched (HTTP 403 from this environment): `helpx.adobe.com` Color Balance
pages; a current-Adobe Help snippet ("Move the Cyan/Red, Magenta/Green, or
Yellow/Blue slider toward a color that you want to add") appeared in a search
result and agrees with the CS6 text, but the page itself was not retrieved.

## Open questions

- **Tonal-band weight curves** are undocumented. Resolves with: fitting a gray
  ramp against CS6 output.
- **Preserve Luminosity renormalization** (luminance formula, clipping order) is
  closed. Resolves with: a saturated-patch calibration.
- **Preserve Luminosity default** (on/off) and **default active tonal range**
  (Midtones?) are inferred. Resolves with: a first-run CS6 capture.
- **CMYK axis mapping** (do the three pairs move C/M/Y, and is K affected?).
  Resolves with: a CMYK CS6 test.
- **Lab mapping of the three sliders** onto `a`/`b` (Help names only two
  channels). Resolves with: a Lab CS6 test.
- **Whether Color Balance participates in the CS6 Properties-panel Preset menu**
  — the Help's preset list omits it. Resolves with: a CS6 panel capture.
- **PSD keys** for the adjustment-layer payload. Resolves with:
  `01-architecture/file-formats.md`.
