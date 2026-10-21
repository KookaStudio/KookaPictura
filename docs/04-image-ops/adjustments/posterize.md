# Posterize

- **Spec ID:** `ADJ-021`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `No` — the Posterize adjustment is long-standing. CS6 moved the settings UI into the Properties panel (CS5 used the Adjustments panel); the 32-bpc adjustment-layer whitelist still excludes it.
- **Depends on:** `ARCH-004` rust-qt-interop, `ARCH-008` document-model, `ARCH-009` undo-history, `04-image-ops/image-modes.md`, `04-image-ops/bit-depth-and-conversion.md`, `05-layers/adjustment-layers.md`

> All module and widget names below are **design proposals**. The repository
> contains a working implementation; this file stays the behavioural contract
> for the feature, and `docs/dev/doc-code-parity-gaps.md` records what shipped
> and what did not. Facts not confirmed by a fetched CS6 source are marked
> *(inferred)*.

## CS6 behavior

**Posterize** reduces each channel to a fixed number of tonal levels. The CS6
Help: "The Posterize adjustment lets you specify the number of tonal levels (or
brightness values) for each channel in an image and then maps pixels to the
closest matching level."

- **Paths** (per the Help):
  - Adjustments panel icon (CS5) / Properties panel (CS6) → creates a
    non-destructive **Posterize adjustment layer**; also
    `Layer > New Adjustment Layer > Posterize`.
  - `Image > Adjustments > Posterize` — destructive ("discards image
    information").
- **Control.** A single **Levels** slider (or numeric entry) setting the number of
  tonal levels. The Help's worked example: "choosing two tonal levels in an RGB
  image gives six colors: two for red, two for green, and two for blue."
  *(The Help counts the per-channel primaries; the full combinatorial count for
  `n` levels in RGB is `n^3` — see Algorithms.)*
- **Typical use.** Creating large flat tonal areas and special effects; "its
  effects are most evident when you reduce the number of gray levels in a
  grayscale image, but it also produces interesting effects in color images."
- **Color-count workflow.** The Help recommends converting to Grayscale, setting
  the level count, then converting back and recoloring when an exact number of
  colors is wanted.
- **Adjustment-layer behavior.** Posterize is not in the CS6 preset-save list
  (Levels, Curves, Exposure, Hue/Saturation, Black & White, Channel Mixer,
  Selective Color), so it has no named preset UI; it does have editable layer
  settings.
- **Interaction with blend modes.** Applying the adjustment layer with the
  **Luminosity** blend mode posterizes brightness while leaving original color
  *(secondary source, CC-era; behavior is generic layer compositing, not
  Posterize-specific)*.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Image > Adjustments > Posterize` | Menu command | — | Destructive; acts on active layer/selection |
| Adjustments panel (CS5) / Properties panel (CS6) | Panel icon | — | Creates a Posterize adjustment layer |
| `Layer > New Adjustment Layer > Posterize` | Menu | — | Non-destructive |
| Properties panel body | Numeric slider | — | Single `Levels` control (2–255) |
| Layer palette row | Context menu | — | Blend mode, opacity, mask, clip |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Levels | integer | 4 *(secondary source)* | 2–255 *(secondary source)* | Number of tonal levels per channel |
| Blend mode | enum | Normal | 27 CS6 modes | Adjustment-layer path |
| Opacity | percent | 100 | 0–100 | Adjustment-layer path |
| Layer mask | gray | white | per-pixel | Adjustment-layer path |
| Clip to layer below | bool | Off | on/off | Adjustment-layer path |

The CS6 Help does not print the numeric range; the 2–255 range and default 4 come
from a CC-era secondary source that also documents the modern slider. The lower
bound of 2 is structurally necessary (1 level would collapse a channel to a single
value), and 255 is the 8-bit identity. *(default/range: secondary source)*

## Algorithms & pipeline

The Help documents the contract (quantize each channel to `n` nearest levels)
but not Adobe's kernel. The following is the standard uniform quantizer
*(inferred)*.

### Uniform quantization

Let `n` be the Levels count, `v` the input sample, and `M = 2^b − 1` the channel
maximum (`255` at 8 bpc, `65535` at 16 bpc). Define `n` evenly spaced target
levels:

```
level_k = k · M / (n − 1),   k = 0 … n − 1
```

Then map each sample to the nearest target level:

```
out = round( v · (n − 1) / M ) · M / (n − 1)
```

Equivalently, with `q = round(v · (n−1) / M)` (an integer in `0 … n−1`), the
output is `q · M / (n − 1)`. The mapping is per channel and independent.

- **`n = 2`** — output samples are `{0, M}`; an RGB image yields `2^3 = 8`
  possible colors (the Help's "six colors" counts the two reds, two greens, and
  two blues individually).
- **`n = 4`** (default) — RGB yields up to `4^3 = 64` colors (secondary source
  states the same 4×4×4 = 64).
- **`n = 255`** at 8 bpc — target levels coincide with every integer, so the op is
  the identity; this matches a secondary source's "essentially turning the
  Posterize adjustment off".
- **`n = M`** generalizes the 8-bit identity to 16 bpc (`n = 65535`).

### Precision and rounding

- 8-bit and 16-bit integer: compute in a wider integer or float, round
  half-to-even or half-up consistently, then clamp to `0 … M`. Adobe's exact
  rounding mode is undocumented. *(inferred)*
- 32-bit float: no CS6 Posterize adjustment layer exists at 32 bpc (whitelist
  excludes it). If a direct command is offered, the float rule is undocumented.
  *(inferred)*
- Because output levels are exactly representable, the operation is idempotent:
  applying Posterize with the same `n` twice equals one application.

### Per-mode behavior

| Mode | Channels quantized | Notes |
|---|---|---|
| RGB | R, G, B | Up to `n^3` colors |
| Grayscale | 1 | The most visually obvious case (Help) |
| CMYK | C, M, Y, K | Up to `n^4` ink combinations; total-ink not re-limited *(inferred)* |
| Lab | L, a, b | Quantizing a/b creates hue banding; L quantizes lightness *(inferred)* |
| Bitmap | 1 bit | Already 2-valued; Posterize is degenerate/unavailable *(inferred)* |
| Indexed | — | Palette-based; expected unavailable *(inferred)* |

## Rust module mapping

- `pictura_adjust::posterize` — `PosterizeOp { levels: u16 }` with
  `fn apply_inplace(&self, tile: &mut TilePatch, depth: BitDepth)`; the quantizer
  is shared across depths via `PixelValue`.
- `pictura_adjust::quantize` — pure functions `uniform_levels(n, max)` and
  `quantize_sample(v, n, max)` with unit tests for `n ∈ {2, 3, 4, 255}`.
- `pictura_adjust::traits` — `Adjustment`, `AdjustmentKind::Posterize`,
  `ParameterSpec` describing the 2–255 integer range for UI generation.
- `pictura_render::adjust` — optional wgpu compute variant; CPU is the reference.

Crossing types: `PosterizeParams { levels: u16 }`, `BitDepth`, `TilePatch`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `PosterizePropertiesWidget` | `QWidget` | Scrubby integer field / slider bound to 2–255, live preview |
| `AdjustmentPropertiesPanel` | `QWidget` stack | Selects the Posterize page when the layer is active |
| `ScrubbyIntegerField` | `QWidget` | Shared drag-to-scrub + spin control matching CS6 numeric fields |
| `AdjustmentListModel` | `QAbstractItemModel` | Adjustment-layer list/thumbnails |

Widgets over QML for a dense dockable properties panel consistent with the CS6
layout.

## Data-model impact

- **Layer node.** Adjustment-layer node with `AdjustmentKind::Posterize` and
  `PosterizeParams { levels: u16 }`. Value range 2–255; validation at the model
  boundary.
- **Serialization.** Standard adjustment-layer records plus the single levels key
  (PSD adjustment-layer type key; exact key name to confirm in
  `01-architecture/file-formats.md`). No XMP fields.
- **Undo.** Creating/deleting the layer and changing `levels` are separate history
  records; a slider drag should coalesce into one record on release. Destructive
  apply records tile deltas.
- **Bit depth.** Parameter is depth-independent; kernel scales `M` to the
  document depth.

## Edge cases

- **`n = 2`** (minimum) — strongest effect; must produce exactly two output levels
  per channel, not one.
- **`n = 255`** at 8 bpc must be the identity; `n = 65535` at 16 bpc likewise.
- **Rounding at midpoints** — a sample exactly between two levels must land
  deterministically; the two nearest-neighbor tie rule must be fixed and tested.
- **16-bit banding** — quantizing 16-bit data to few levels is expected; do not
  "smooth" it.
- **CMYK total ink** — quantizing can push total ink past profile limits; the
  output conversion, not Posterize, is responsible.
- **Lab a/b** — quantization of signed/offset chroma can introduce color casts;
  document as CS6-consistent.
- **Indexed / Bitmap** — expected unavailable; grey out and document.
- **32-bpc** — no adjustment layer (whitelist); confirm direct command
  availability.
- **Selection** — only selected pixels change on the destructive path.
- **1-px / empty / huge documents** — tile-local; empty is a no-op; PSB sizes need
  no special path.
- **GPU unavailable** — identical CPU result, higher latency.
- **Undo/redo** — exact round-trip; idempotence holds for equal `n`.

## Parity acceptance criteria

- Given an 8-bit grayscale ramp and `Levels = 4`, output has exactly four distinct
  values at `0, 85, 170, 255` (`round(k·255/3)`); pixels are mapped to the
  nearest.
- Given an RGB image and `Levels = 2`, at most eight distinct colors appear
  (each channel ∈ {0, 255}).
- Given `Levels = 255` at 8 bpc, the output is identical to the input.
- Given the same `Levels` twice, the result equals one application (idempotence).
- Given a selection, pixels outside it are unchanged on the destructive path.
- Given an adjustment layer, changing `Levels` updates the canvas live and undo
  restores the previous value as one state per commit.
- Given a 16-bit document, quantization uses `M = 65535`; `Levels = 2` yields
  exactly `{0, 65535}`.
- Given a CMYK document, all four channels are quantized independently.
- Given a 32-bpc document, no Posterize adjustment layer is offered.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  official CS6 Help reference; downloaded and text-extracted. Establishes: the
  "specify the number of tonal levels … then maps pixels to the closest matching
  level" definition; the two-levels-in-RGB "six colors" example; the grayscale and
  color use notes; the grayscale-then-reconvert color-count workflow; the three
  application paths and the destructive warning; the 32-bpc adjustment-layer
  whitelist that omits Posterize; the preset-save whitelist that omits Posterize.
- `https://www.photoshopessentials.com/photo-effects/how-to-posterize-a-photo-in-photoshop`
  — CC-era tutorial used as a secondary source for: the Levels control, default
  value **4**, minimum **2**, maximum **255**, the 255 = "off" behavior, the
  4×4×4 = 64-color example, and the Luminosity-blend-mode color-preserving trick
  (CC-era; CS6 behavior inferred from generic layer compositing).

Not parsed in this pass: `helpx.adobe.com` (HTTP 403 from this environment).

## Open questions

- **CS6 default Levels.** The value 4 is from a CC-era source, not the CS6 Help.
  Resolves with: a first-run CS6 Posterize panel capture.
- **Numeric range as printed in CS6.** 2–255 is from a secondary source; confirm
  the CS6 UI bounds. Resolves with: a CS6 slider capture.
- **Rounding/tie rule.** Adobe's exact midpoint rounding is undocumented.
  Resolves with: a CS6 ramp probe at a midpoint sample.
- **Per-mode availability.** Posterize for Bitmap, Indexed, Lab, Multichannel, and
  32-bpc documents. Resolves with: a mode-by-mode test.
- **"Six colors" wording.** Whether the Help is loosely counting per-channel
  primaries (8 actual combinations) or describing a different behavior. Resolves
  with: a CS6 two-level RGB pixel count.
- **PSD adjustment-layer key.** Exact serialization key for the levels value.
  Resolves with: the Adobe file-format specification.
