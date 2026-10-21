# Hue/Saturation Adjustment

- **Spec ID:** `ADJ-010`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `Changed` — the adjustment itself is long-standing, but CS6 moved its UI from the CS5 Adjustments panel to the Properties panel and exposes Hue/Saturation presets through the panel's Preset menu (CS5 surfaced presets as tiles in the Adjustments panel).
- **Depends on:** `ARCH-008` document-model, `ARCH-009` undo-history, `ARCH-006` gpu-rendering-pipeline, `01-architecture/color-management.md`, `04-image-ops/image-modes.md`, `04-image-ops/adjustments-overview.md`, `05-layers/adjustment-layers.md`, `07-color-painting/color-models.md`, `07-color-painting/color-picker.md`, `10-workflow-io/presets-manager.md`.

> All module and widget names below are **design proposals**. The repository
> contains a working implementation; this file stays the behavioural contract
> for the feature, and `docs/dev/doc-code-parity-gaps.md` records what shipped
> and what did not. Facts not confirmed by a fetched CS6 source are marked
> *(inferred)*.

## CS6 behavior

**Hue/Saturation** adjusts the hue, saturation, and lightness of an image, either
globally (Master) or within one of six selectable color ranges. The CS6 Help
describes it as "especially good for fine-tuning colors in a CMYK image so that
they are in the gamut of an output device."

- **Two invocation paths** — `Image > Adjustments > Hue/Saturation` makes a
  destructive, direct edit (the Help warns it "makes direct adjustments to the
  image layer and discards image information"); `Layer > New Adjustment Layer >
  Hue/Saturation` (or the Adjustments/Properties panel icon) creates a
  non-destructive adjustment layer with a mask.
- **Two color bars** — the upper bar shows the color before the adjustment; the
  lower bar "shows how the adjustment affects all of the hues at full
  saturation."
- **Edit menu (the channel selector)** — the menu to the right of the on-image
  adjustment tool offers **Master** plus six ranges: **Reds, Yellows, Greens,
  Cyans, Blues, Magentas**. Master adjusts all colors at once.
- **Hue** — rotates the color around the wheel. The displayed value is degrees of
  rotation from the original pixel color: positive = clockwise, negative =
  counter-clockwise, range **-180 to +180**.
- **Saturation** — moves color away from / toward the wheel center, range
  **-100 to +100** (`-100` = fully desaturated, `+100` = maximum increase).
- **Lightness** — adds white (`+`) or black (`-`) to a color, range
  **-100 to +100**.
- **On-image (targeted) adjustment tool** — with the tool active, `Ctrl`/`Cmd`
  click a color and drag left/right to change **hue**; plain click-drag left/right
  changes **saturation** of the color range containing the clicked pixel.
- **Range editing** — selecting a color range shows **four color-wheel values in
  degrees**. The two inner vertical sliders define the range; the two outer
  triangle sliders define the fall-off (feathering). Defaults: the range is
  **30° wide with 30° of fall-off on either side**. Drag the triangles to change
  fall-off without changing range, drag the region between triangle and bar to
  change range without changing fall-off, drag the center to move the whole
  slider, or `Ctrl`/`Cmd`-drag the color bar to re-center it. The Help warns that
  "setting the fall-off too low can produce banding."
- **Range naming** — if a range is moved into a different color region the name
  changes (e.g. Yellow moved into the red part of the bar becomes **Red 2**). Up
  to six variants of a range can exist (Red through Red 6).
- **Eyedropper sampling** — the **Eyedropper** sets the range, **Add To Sample**
  expands it, **Subtract From Sample** shrinks it. While an eyedropper is active,
  `Shift` adds and `Alt`/`Option` subtracts.
- **Colorize** — an absolute (not relative) recolor mode. "If the foreground
  color is black or white, the image is converted to a red hue (0°). If the
  foreground color is not black or white, the image is converted to the hue of
  the current foreground color. The lightness value of each pixel does not
  change." Colorize suppresses the original-color tint that a relative
  Saturation-only change would leave. The Help advises converting a Grayscale
  document to RGB first.
- **Presets** — settings are savable/reloadable (Save Preset / Load Preset); in
  CS6 a preset is chosen from the **Preset menu** in the Properties panel.
- **Reset** — the panel Reset button restores the adjustment to neutral
  (Master, Hue 0, Saturation 0, Lightness 0).

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Image > Adjustments > Hue/Saturation` | Menu command | `Ctrl/Cmd+U` | Destructive direct edit to the active layer. |
| `Layer > New Adjustment Layer > Hue/Saturation` | Menu command | — | Non-destructive adjustment layer with mask. |
| Adjustments / Properties panel | Panel | — | Adjustment-layer settings: Edit menu, Hue/Saturation/Lightness, Colorize, Reset, Preset. |
| Properties panel — Edit menu | Combo | — | Master / Reds / Yellows / Greens / Cyans / Blues / Magentas. |
| Properties panel — color bars | Custom slider | — | Before/after bars; four range handles when a color range is selected. |
| Properties panel — on-image tool | Tool toggle | — | `Ctrl`/`Cmd`-click-drag = hue; click-drag = saturation. |
| Properties panel — Preset menu | Menu | — | CS6 new location for stored Hue/Saturation presets. |
| `Channels`/`Layers` panels | Context | — | Adjustment layer applies to layers below unless clipped/grouped. |
| Color Picker / Swatches | Dialog / panel | — | Supplies the Colorize hue reference (foreground color). |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Edit (color range) | enum | Master | Master / Reds / Yellows / Greens / Cyans / Blues / Magentas | Help calls it the menu "to the right of the On-image adjustment tool". |
| Hue | int (degrees) | 0 | -180 … +180 | Rotation from original pixel hue; positive clockwise. |
| Saturation | int (percent) | 0 | -100 … +100 | -100 = desaturate to gray. |
| Lightness | int (percent) | 0 | -100 … +100 | + = add white, - = add black. |
| Colorize | bool | Off *(inferred)* | on / off | Absolute H/S replacement; fixed foreground-derived hue. |
| Range width | degrees | 30° | 0° … (bounded by neighbors) *(inferred)* | Inner vertical bars. |
| Range fall-off | degrees | 30° per side | 0° … *(inferred)* | Outer triangle bars; low values cause banding. |
| Range center | degrees | per-color | 0° … 360° | `Ctrl`/`Cmd`-drag the color bar to move. |
| Eyedropper mode | enum | Eyedropper | Eyedropper / Add To Sample / Subtract From Sample | `Shift` add, `Alt`/`Option` subtract while active. |
| Preset | named setting | Default | user-defined + shipped | Persisted via Preset Manager. |

## Algorithms & pipeline

Adobe's exact operator is closed; the following is a behavioural-parity model
*(inferred)* unless a CS6 Help statement is quoted. The Help's color-wheel
description establishes an HSL-family model but not the precise math.

### Working model

The adjustment is naturally expressed in a cylindrical (hue-bearing) space. For
each pixel:

1. Convert the working-space color to a hue angle `H ∈ [0,360)`, saturation
   `S ∈ [0,1]`, lightness `L ∈ [0,1]` (HSL is the CS6 Help's model; the exact
   transform and whether saturation is HSL or HSV is *(inferred)*).
2. Compute the per-color weight `w(H)` from the active range: a trapezoid set by
   the two inner bars (range edges) and two outer triangles (fall-off), i.e. a
   plateau of width 30° by default with linear/feathered 30° shoulders. The exact
   shoulder curve is not documented; a smooth (linear or raised-cosine)
   trapezoid is the reference proposal.
3. Apply, with `Δhue`, `Δsat`, `Δlight` = slider values / 100:
   - `H' = (H + Δhue·360/360·w(H)) mod 360` (Hue is already in degrees).
   - Master uses `w(H) = 1` for all hues.
   - `S' = clamp(S · (1 + Δsat·w(H)))` *(inferred; the exact saturate law is
     unknown)*.
   - `L' = clamp(L + Δlight·w(H)·f(L))` where `f(L)` is the black/white
     weighting that lets `+` "add white" and `-` "add black" *(inferred)*.
4. Convert back to the document working space and clamp to the document bit
   depth.

### Colorize

Colorize overrides the relative math. The Help gives the observable contract:
output hue is the current foreground hue (or 0° red when the foreground is pure
black/white), saturation is taken from the Saturation slider, and each pixel's
lightness is preserved. A reference implementation is:
`(H,S,L) → (H_fg, Δsat_abs, L_orig)`. Exact saturation mapping is *(inferred)*.

### Range selection

The four degree values are the outer triangle left, inner bar left, inner bar
right, outer triangle right (in the Help's figure description: A = hue slider
values, B = fall-off, C = range, D = range+fall-off, E = whole slider)
*(inferred for the exact tuple order)*. Range and fall-off may overlap neighbour
ranges; up to six ranges (`Red`…`Red 6`) can share a color family.

### Placement in the pipeline

- Destructive path writes the active layer's pixels.
- Adjustment-layer path evaluates below the layer, in the document working
  space, then feeds the result through the adjustment layer's blend mode,
  opacity, fill, and mask (`05-layers/adjustment-layers.md`).
- The six-range model is per-hue and non-parametric, so it can be applied per
  tile without cross-tile state.

## Rust module mapping

Proposals; the panel and kernels are shared with the other HSL-family
adjustments.

- `pictura_ops::adjust::hue_saturation` — `HueSaturationSettings { range,
  hue_deg, saturation, lightness, colorize }`, `HueSaturationRange` enum, and a
  `PixelEdit` trait implementation used by both the destructive and
  adjustment-layer paths.
- `pictura_ops::adjust::hue_range` — `HueRange { center_deg, width_deg,
  falloff_deg, left_span, right_span }` plus `weight(hue) -> f32`; one function
  shared by the six ranges.
- `pictura_color::hsl` — documented RGB↔HSL (and, if required, HSV) transforms
  and the CMYK/Lab→HSL bridge; all colour-model conversions live here, not in the
  operator.
- `pictura_core::command` — `HueSaturationCommand { target: LayerId | AdjLayerId,
  settings, mask, dirty_rect }`; `settings` is the serializable payload.
- `pictura_render::adjust` — optional wgpu compute variant for interactive
  latency; the CPU kernel in `pictura_ops` is the reference.

Crossing types: `AdjLayerId`, `LayerId`, `ColorSpace`, `Rect`, `TileDelta`,
`HueSaturationSettings`.

## Qt6 component mapping

Table proposals for the Properties panel.

| Proposal | Base | Responsibility |
|---|---|---|
| `AdjustmentPropertiesPanel` | `QStackedWidget` | Hosts one editor widget per adjustment type; rebuilt when the selected adjustment layer changes. |
| `HueSaturationEditor` | `QWidget` | Edit menu, Hue/Saturation/Lightness scrub fields, Colorize check, Reset, Preset menu, on-image tool toggle. |
| `ColorRangeBar` | custom `QWidget` | Draws the two colour bars and the four range handles; emits `rangeChanged(HueRange)`. |
| `ScrubField` | custom `QWidget` | Numeric field with drag-scrub and `Up`/`Down`/`Shift` step semantics shared with other panels. |
| `OnImageToolController` | `QObject` | Routes canvas press/drag to hue or saturation based on modifiers; samples the canvas for a range. |
| `AdjustmentLayerModel` | `QAbstractItemModel` | Selection model; exposes `layer.settings(HueSaturationSettings)`. |
| `PresetMenu` | `QMenu` | Loads/saves named settings through `10-workflow-io/presets-manager.md`. |

Widgets rather than QML: the panel is a docked, dense, keyboard-driven form that
must match `02-ui-ux/application-frame.md` and the existing widget-based shell
*(design decision; see `01-architecture/qt6-ui-design.md`)*.

## Data-model impact

- **Adjustment-layer node.** A Hue/Saturation adjustment layer stores
  `HueSaturationSettings` (range, three sliders, colorize, per-range geometry)
  as its content payload plus a mask.
- **PSD serialization.** Photoshop stores adjustment-layer settings in
  `Lr16`/`Lr32`-style "additional layer information" blocks and a legacy
  `Adjustment` block; the exact tags are *(inferred)* and belong in
  `01-architecture/file-formats.md`. The CS6 Help does not document PSD keys.
- **Preset serialization.** Named presets are `.AHU`/`.ahu`-family files in
  Photoshop *(inferred)*; the independent format is a project decision, not a
  parity requirement.
- **Undo.** Destructive edit: one history state holding tile deltas. Adjustment
  layer: settings edits are individual history states; the layer node itself is
  the durable record.
- **No new channels.** The adjustment is a pure colour transform; alpha is
  untouched.

## Edge cases

- **8/16/32-bit.** Hue/Saturation is in the CS6 32-bpc supported list, so it must
  work at all three depths. At 32 bpc the operator must not quantize to 8-bit
  integer; run in `f32`.
- **CMYK.** The Help explicitly recommends it for CMYK gamut fitting. The
  CMYK→HSL→CMYK round-trip must be deterministic and must not drift across
  repeated edits.
- **Lab.** Whether the six color ranges are expressed on Lab-derived hue or on a
  round-tripped RGB hue is unverified; the conversion must be defined.
- **Grayscale / Bitmap / Indexed / Multichannel.** Bitmap and Indexed have no
  continuous hue; the adjustment is expected to be unavailable or inert. In
  Grayscale, the Help recommends converting to RGB before colorizing; a
  non-colorizing adjustment moves lightness only.
- **Saturated out-of-gamut values.** `+Saturation` on already-saturated pixels
  must clamp gracefully without hue shift.
- **Fall-off = 0.** Help warns of banding; reproduce, do not silently clamp.
- **Range overlap.** Adjacent ranges may overlap; the composite must be the
  product/sum of per-range effects for a pixel in the overlap (exact rule
  *(inferred)*).
- **1-px / empty / huge documents.** Tile-based, dirty-rect only; empty documents
  are rejected at open; PSB sizes must not allocate full-canvas scratch.
- **GPU unavailable.** CPU and GPU must agree within tolerance; CPU is the
  reference.
- **Undo mid-edit.** Scrubbing sliders coalesces into one history state per
  committed edit (release / focus-out), not per pixel of mouse movement.

## Parity acceptance criteria

1. Given an RGB document, Master Hue `+180` then `-180` returns every pixel to
   its original value within one quantization step at the document bit depth.
2. Given a saturated red patch, selecting **Reds** with Saturation `-100`
   desaturates it while leaving a blue patch effectively unchanged, and vice
   versa.
3. Given the default 30° range and 30° fall-off, a hue sweep shows a plateau over
   the range and monotonically tapering effect across the fall-off.
4. Given **Colorize** on with a non-neutral foreground, output hue equals the
   foreground hue within a small tolerance for all pixels, and per-pixel
   lightness ordering is preserved.
5. Given **Colorize** on with foreground black or white, output hue is 0° (red).
6. Given the on-image tool, click-dragging without modifiers changes saturation
   of the sampled color family; `Ctrl`/`Cmd`-drag changes hue.
7. Given a CMYK document, a Master saturation increase places values inside the
   soft-proof gamut marker as expected by `01-architecture/color-management.md`.
8. Given a 32-bpc document, the adjustment is available and applied without
   integer quantization.
9. Given a completed adjustment edit, History shows exactly one new state and
   undo restores the prior values bit-exactly.
10. Given an adjustment-layer instance, opacity, blend mode, and mask modulate
    the result per `05-layers/adjustment-layers.md`.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  official CS6 Help reference; downloaded and text-extracted. Establishes:
  the purpose statement and CMYK-gamut note; the destructive-vs-adjustment-layer
  paths; the two color bars; the Master + six preset ranges; Hue -180…+180 with
  clockwise-positive degrees; Saturation -100…+100; Lightness -100…+100; the
  on-image tool modifier behaviour; the four-degree range model with inner
  vertical bars and outer fall-off triangles; the 30° range / 30° fall-off
  default and the banding warning; range renaming (Red 2 … Red 6); the
  Eyedropper / Add To Sample / Subtract From Sample tools; the Colorize rules
  (red at 0° for black/white foreground, otherwise foreground hue, lightness
  preserved); the Reset button; the 32-bpc supported-adjustment list naming
  Hue/Saturation.
- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  same file, "What's New in CS6" / Properties-panel context establishes the CS6
  panel move and the Preset-menu placement (see the Hue/Saturation, Black &
  White, and preset sections of that text).

Not fetched (HTTP 403 from this environment; snippets only, used to cross-check
panel placement and preset availability): `helpx.adobe.com` Hue/Saturation pages.

## Open questions

- **Precise saturation and lightness laws** are not documented. Resolves with:
  a CS6 saturation/lightness sweep compared against a candidate HSL model.
- **True working space of the operator** (HSL vs HSV; whether Lab/CMYK are
  bridge-converted through RGB). Resolves with: pixel-level tests on CMYK and Lab
  documents against CS6.
- **Master weighting formula** (does Master apply a flat weight or a hue-dependent
  one?). Resolves with: a saturated-hue ramp test.
- **Default state of Colorize** and whether it persists in presets. Resolves
  with: a first-run CS6 panel capture.
- **Shipped Hue/Saturation preset names** are referenced by the Help but not
  enumerated. Resolves with: the CS6 Preset menu capture.
- **Overlap composition rule** for adjacent color ranges. Resolves with: a CS6
  test using two deliberately overlapping ranges.
- **PSD keys** for adjustment-layer settings. Resolves with:
  `01-architecture/file-formats.md` and PSD documentation.
