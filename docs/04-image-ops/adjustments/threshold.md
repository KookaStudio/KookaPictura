# Threshold

- **Spec ID:** `ADJ-022`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `No` — the Threshold adjustment is long-standing. CS6 changes: settings moved to the Properties panel (CS5 used the Adjustments panel), and CS6 added "Enable Invert and Threshold adjustments for masks in 32-bit/channel images". The 32-bpc adjustment-layer whitelist still excludes it.
- **Depends on:** `ARCH-004` rust-qt-interop, `ARCH-008` document-model, `ARCH-009` undo-history, `04-image-ops/image-modes.md`, `04-image-ops/bit-depth-and-conversion.md`, `05-layers/adjustment-layers.md`, `01-architecture/color-management.md`

> All module and widget names below are **design proposals**. The repository
> contains a working implementation; this file stays the behavioural contract
> for the feature, and `docs/dev/doc-code-parity-gaps.md` records what shipped
> and what did not. Facts not confirmed by a fetched CS6 source are marked
> *(inferred)*.

## CS6 behavior

**Threshold** binarizes the image. The CS6 Help: "The Threshold adjustment converts
grayscale or color images to high-contrast, black-and-white images. You can
specify a certain level as a threshold. All pixels lighter than the threshold are
converted to white; all pixels darker are converted to black."

- **Paths** (per the Help):
  - Adjustments panel icon (CS5) / Properties panel (CS6) → creates a
    non-destructive **Threshold adjustment layer**; also
    `Layer > New Adjustment Layer > Threshold`.
  - `Image > Adjustments > Threshold` — destructive ("discards image
    information").
- **Histogram.** Opening the adjustment "displays a histogram of the luminance
  levels of the pixels in the current selection"; the user "drag[s] the slider
  below the histogram until the threshold level you want appears", with live
  preview.
- **Control.** A single **Threshold Level** (the Help's "threshold level").
- **Use as a measuring tool.** The Help also recommends Threshold to "identify
  representative highlights and shadows before accessing Levels or Curves",
  because the output is two clean populations.
- **CS6 mask support.** CS6 enables the Threshold *adjustment* on masks in
  32-bit/channel documents (What's-new list), separate from the 32-bpc
  adjustment-layer whitelist.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Image > Adjustments > Threshold` | Menu command | — | Destructive; acts on active layer/selection |
| Adjustments panel (CS5) / Properties panel (CS6) | Panel icon | — | Creates a Threshold adjustment layer |
| `Layer > New Adjustment Layer > Threshold` | Menu | — | Non-destructive |
| Properties panel body | Histogram + slider | — | Luminance histogram with a single level slider |
| Layer palette row | Context menu | — | Blend mode, opacity, mask, clip |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Threshold Level | integer | 128 *(inferred; conventional mid-gray)* | 1–255 | Luminance cut point |
| Blend mode | enum | Normal | 27 CS6 modes | Adjustment-layer path |
| Opacity | percent | 100 | 0–100 | Adjustment-layer path |
| Layer mask | gray | white | per-pixel | Adjustment-layer path |
| Clip to layer below | bool | Off | on/off | Adjustment-layer path |

The Help does not print the numeric range or default. The 1–255 range is the
8-bit domain of the slider below the histogram; the conventional default is 128
(middle gray). Both are *(inferred)* pending a CS6 capture.

## Algorithms & pipeline

The Help states the contract in luminance terms but not Adobe's exact luminance
formula or boundary rule. The following is the standard binarization model
*(inferred)*.

### Binarization

For a pixel with composite luminance `Y` (in the channel range `0 … M`,
`M = 2^b − 1`) and threshold `T`:

```
out_channel = { M (white)  if Y > T
              { 0 (black)  if Y < T
```

- The result is neutral: every color channel is set to the same 0 or `M`, so the
  output is genuinely black-and-white, not a per-channel binarization (which
  would yield up to `2^3 = 8` colors in RGB).
- **Boundary.** The Help says "lighter than → white, darker than → black" and is
  silent on `Y == T`. Adobe's tier rule (white vs black at equality) is
  *(inferred)* and must be fixed by test.
- **Luminance.** The Help calls the histogram a "luminance levels" histogram,
  implying a single weighted luminance `Y = w_r·R + w_g·G + w_b·B`. The exact
  weights and whether they are the document working-space luma or a fixed
  Rec.601/Rec.709 set are undocumented. *(inferred)*
- **Grayscale** uses the single channel directly (`Y` = gray sample).
- **CMYK** must map a luminance to either white (`0/0/0/0`) or black
  (`K = M`, C = M = Y = 0). The exact CMYK output encoding is *(inferred)*.
- **Lab** must map to either an L-only white or black; the documented output is
  black/white, not a Lab binarization of a/b. *(inferred)*

### Precision

- 8-bit: output samples are exactly `{0, 255}`.
- 16-bit: output samples are exactly `{0, 65535}`.
- 32-bit: no Threshold adjustment layer (whitelist excludes it). Mask-level
  Threshold in 32-bpc is allowed per the What's-new list; its float rule is
  undocumented. *(inferred)*
- Threshold is idempotent and idempotent under re-application at the same `T`.

### Where it fits

Adjustment-layer path: replaces the layer's color contribution with the binarized
luminance under the usual blend/opacity/mask compositing. Destructive path:
overwrites the active layer/selection and records tile deltas.

## Rust module mapping

- `pictura_adjust::threshold` — `ThresholdOp { level: u16 }` with
  `fn apply_inplace(&self, tile: &mut TilePatch, depth: BitDepth)`.
- `pictura_adjust::luma` — `fn composite_luma(pixel: &PixelValue, space: ColorSpace) -> f32`
  isolating the single luminance definition so the weights are tunable
  (calibration knob; see "Hardware"-style tuning note below).
- `pictura_adjust::traits` — `Adjustment`, `AdjustmentKind::Threshold`,
  `ParameterSpec` (1–255 integer, histograms supplied by the UI).
- `pictura_render::adjust` — optional wgpu compute variant; CPU is the reference.

Crossing types: `ThresholdParams { level: u16 }`, `BitDepth`, `TilePatch`,
`LumaWeights` (calibratable).

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `ThresholdPropertiesWidget` | `QWidget` | Luminance histogram + single slider/numeric field with live preview |
| `HistogramView` | `QWidget` paint | Renders the selection/source luminance histogram under the slider |
| `AdjustmentPropertiesPanel` | `QWidget` stack | Selects the Threshold page for the active layer |
| `AdjustmentListModel` | `QAbstractItemModel` | Adjustment-layer list/thumbnails |

Widgets over QML for the dense panel; the histogram widget is shared with Levels
and Curves.

## Data-model impact

- **Layer node.** `AdjustmentKind::Threshold` with `ThresholdParams { level: u16 }`.
- **Serialization.** Standard adjustment-layer records plus the single threshold
  key (exact PSD key to confirm in `01-architecture/file-formats.md`). No XMP.
- **Undo.** Layer create/delete separate from `level` edits; a slider drag
  coalesces to one record on release. Destructive apply records tile deltas.
- **Histogram source.** The histogram is derived from the source pixels below the
  adjustment (or the selection), not serialized.
- **Bit depth.** Parameter is depth-independent; output snaps to `{0, M}`.

## Edge cases

- **`T = 1` and `T = 255`** — boundary extremes: `T = 255` should produce no white
  (nothing lighter than 255), `T = 1` should produce no black. Verify extremes
  don't fall back to the original image.
- **Equality rule** — pixels exactly at `T` must be classified deterministically;
  fix and test the tie rule.
- **Grayscale / RGB** — output is neutral; R = G = B.
- **CMYK / Lab** — output must be a valid black or white in that model, not a
  per-channel binarization; total-ink must be sane.
- **Indexed / Bitmap** — expected unavailable (Indexed) or degenerate (Bitmap);
  grey out and document.
- **32-bpc** — no adjustment layer; mask-level Threshold allowed per CS6.
- **Selection** — histogram reflects the selection; destructive apply touches only
  selected pixels.
- **1-px / empty / huge documents** — tile-local; empty no-op; PSB sizes need no
  special path.
- **GPU unavailable** — identical CPU result, higher latency.
- **Undo/redo** — exact round-trip; idempotent at equal `T`.

## Parity acceptance criteria

- Given an 8-bit grayscale ramp and `Threshold Level = 128`, all pixels with gray
  > 128 become 255 and all with gray < 128 become 0, with no intermediate values.
- Given an RGB document, the output uses only two neutral colors (black and
  white); no chromatic pixel remains.
- Given `T = 255`, the output is entirely black; given `T = 1`, entirely white.
- Given the same level twice, the result equals one application (idempotence).
- Given a selection, the histogram reflects only selected pixels and only selected
  pixels change on the destructive path.
- Given an adjustment layer, dragging the slider updates the canvas live and undo
  restores the previous level as one state per commit.
- Given a 16-bit document, output samples are exactly `{0, 65535}`.
- Given a CMYK document, output is either `0/0/0/0` or maximal K with no residual
  C/M/Y.
- Given a 32-bpc document, no Threshold adjustment layer is offered, and the
  mask-level behavior described in the CS6 What's-new list is available.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  official CS6 Help reference; downloaded and text-extracted. Establishes: the
  "converts grayscale or color images to high-contrast, black-and-white images"
  description; "All pixels lighter than the threshold are converted to white; all
  pixels darker are converted to black"; the luminance-histogram display and
  slider interaction; the three application paths and destructive warning; the
  highlight/shadow identification use; the CS6 "Enable Invert and Threshold
  adjustments for masks in 32-bit/channel images" note; the 32-bpc
  adjustment-layer whitelist that omits Threshold.

Not parsed in this pass: `helpx.adobe.com` (HTTP 403 from this environment).

## Open questions

- **Default and range.** Default 128 and range 1–255 are inferred; confirm the CS6
  slider bounds (whether 0 is selectable). Resolves with: a CS6 capture.
- **Equality rule.** Which side `Y == T` falls on. Resolves with: a CS6 gray-ramp
  probe at exactly `T`.
- **Luminance weights.** Whether the composite luminance uses fixed Rec.601/709
  coefficients or working-space luma. Resolves with: a color-patch probe.
- **CMYK/Lab output encoding.** Exact channel values for "white" and "black" in
  non-RGB modes. Resolves with: a CS6 mode test.
- **Per-channel targeting.** Whether targeting a single channel (e.g. after
  selecting R in the Channels panel) binarizes that channel instead of luminance.
  Resolves with: a CS6 channel-target test.
- **32-bpc mask rule.** Exact float behavior of mask-level Threshold. Resolves
  with: a 32-bpc mask probe.
