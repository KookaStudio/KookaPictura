# Channel Mixer Adjustment

- **Spec ID:** `ADJ-014`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `Changed` — the adjustment is long-standing; CS6 moves its UI to the Properties panel and exposes **Channel Mixer presets** through the panel's Preset menu. The CS6 "What's New" list does not announce new mixer features; the CS6 Help cross-references it from Black & White but — see `## Sources` — **the how-to page is missing from the CS6 reference PDF**.
- **Depends on:** `ARCH-008` document-model, `ARCH-009` undo-history, `ARCH-006` gpu-rendering-pipeline, `01-architecture/color-management.md`, `04-image-ops/image-modes.md`, `04-image-ops/adjustments-overview.md`, `04-image-ops/adjustments/black-white.md`, `05-layers/adjustment-layers.md`, `07-color-painting/color-models.md`, `10-workflow-io/presets-manager.md`.

> All module and widget names below are **design proposals**. The repository
> contains a working implementation; this file stays the behavioural contract
> for the feature, and `docs/dev/doc-code-parity-gaps.md` records what shipped
> and what did not. Facts not confirmed by a fetched CS6 source are marked
> *(inferred)*.

> **Source caveat.** The official CS6 Help reference PDF fetched for this
> corpus contains a cross-reference ("Channel Mixer command … See Mix color
> channels") but **not** the "Mix color channels" how-to page itself; the
> section appears to be omitted from that build. Behavioral details below are
> therefore established from a mirror of Adobe's official Channel Mixer help
> text (same wording, older Photoshop generation) plus community sources, and
> are marked accordingly. This is recorded in `## Open questions`.

## CS6 behavior

**Channel Mixer** "lets you modify a color channel using a mix of the current
color channels." It is a linear channel-matrix operator with a Monochrome mode.

- **Uses (from the help text):** make creative colour adjustments not easily done
  with other tools; create high-quality grayscale by choosing each colour
  channel's contribution; create sepia/tinted images; convert to/from alternative
  colour spaces such as YCbCr; and swap or duplicate channels.
- **Composite channel first** — as with Color Balance, the composite colour
  channel is selected in the Channels panel before invoking the command.
- **Two invocation paths** — `Image > Adjustments > Channel Mixer` (destructive)
  and `Layer > New Adjustment Layer > Channel Mixer` (non-destructive, with a
  mask). In CS6 a preset is chosen from the Properties-panel **Preset menu**.
- **Output Channel** — "choose the channel in which to blend one or more existing
  (or source) channels." The target channel is what gets written; selecting an
  output channel typically sets its own source slider to 100% and the others to
  0% *(the reset-on-select behaviour is documented in current Adobe help; the
  CS6 Help how-to page is missing)*.
- **Source sliders** — one per source channel. "Drag any source channel's slider
  to the left to decrease the channel's contribution to the output channel or to
  the right to increase it, or enter a value between **-200% and +200%**."
  Negative inverts the source before adding.
- **Constant** — "adds a black or white channel of varying opacity — negative
  values act as a black channel, positive values act as a white channel."
  Range *(inferred)*: -200% … +200%.
- **Monochrome** — "apply the same settings to all the output channels, creating
  a color image that contains only gray values." Used to control detail and
  contrast for grayscale conversion. If you select and then deselect Monochrome,
  you can "modify the blend of each channel separately, creating a handtinted
  appearance." When Monochrome is active the Output Channel becomes **Gray**
  *(community source)*.
- **Default mix** — with Monochrome on, the default is **40% Red, 40% Green,
  20% Blue** (the perceived-luminance weighting) *(community source)*.
- **The "100% rule"** — keeping the source weights summing to 100% preserves
  overall brightness; exceeding 100% risks blown highlights. Adobe does not
  enforce it, but the CS3+ UI shows the total and a warning icon when it is
  exceeded *(community source)*.

### How it differs by colour mode

| Document mode | Output Channel choices | Source sliders | Notes |
|---|---|---|---|
| RGB | Red, Green, Blue (+ Gray in Monochrome) | R, G, B (+ Constant) | Primary case. |
| CMYK | Cyan, Magenta, Yellow, Black *(inferred)* | C, M, Y, K (+ Constant) *(inferred)* | Used to reduce total ink, remix inks. |
| Grayscale | unavailable | — | Only one channel; community sources say the mixer is unavailable. |
| Lab | unavailable | — | Community sources say the mixer is unavailable. |
| Indexed / Bitmap / Multichannel | unavailable *(inferred)* | — | No continuous RGB/CMYK mix. |

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Image > Adjustments > Channel Mixer` | Menu command | — | Destructive direct edit. |
| `Layer > New Adjustment Layer > Channel Mixer` | Menu command | — | Non-destructive adjustment layer with mask. |
| Adjustments / Properties panel | Panel | — | Output Channel combo, per-source sliders, Constant, Monochrome, Preset, Total/warning. |
| Properties panel — Preset menu | Menu | — | CS6 Channel Mixer presets. |
| `Channels` panel | Context | — | Composite channel should be active; individual channels are the source semantics. |
| `10-workflow-io/presets-manager.md` | Manager | — | Save/load mixes. |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Output Channel | enum | Red *(inferred)* | RGB: Red/Green/Blue; CMYK: C/M/Y/K; Monochrome: Gray | Selecting a channel resets its own source to 100%. |
| Source Red | int (percent) | 100 (for Red output) | -200 … +200 | Negative inverts the source. |
| Source Green | int (percent) | 0 (for Red output) | -200 … +200 | |
| Source Blue | int (percent) | 0 (for Red output) | -200 … +200 | |
| Constant | int (percent) | 0 | -200 … +200 *(inferred)* | Negative = black channel, positive = white channel. |
| Monochrome | bool | Off *(inferred)* | on / off | Same settings for all outputs; Output Channel → Gray. |
| Monochrome default mix | int triple | 40 / 40 / 20 | -200 … +200 each | Standard luminance weighting *(community)*. |
| Total | read-only | 100% when balanced | — | Warning icon when the source weights exceed 100%. |
| Preset | named matrix | Default | shipped + user | CS6 Properties-panel menu. |

## Algorithms & pipeline

### Core matrix

For each pixel, per output channel `c`, where `{s}` are the source channels in
the document's mode:

`out_c = clamp( Σ_s (w_cs / 100) · in_s + (constant / 100) )`

- Weights are percentages in -200…+200; negative weights invert `in_s` before
  summing.
- `Constant` adds a flat black (negative) or white (positive) offset before the
  final clamp.
- The CS6 Help (via the mirror) and the community math confirm this
  additive/matrix form.
- **Monochrome** uses the same weight triple for every output channel, so
  `R' = G' = B' = Σ w_s · in_s + constant`, producing a gray. Toggling Monochrome
  off restores independent per-output triples (the "handtinted" path).

Example (community source): for green output with sources Red 50%, Green 100%,
Blue 0%, an input `RGB(50,100,200)` yields `G' = 50·0.5 + 100·1.0 + 200·0 = 125`
→ `RGB(50,125,200)`. Only the output channel changes.

### Operand space

The classic Channel Mixer operates on the channel values as stored (gamma-encoded
for RGB, ink values for CMYK), not on linear-light values. This matters for
Monochrome weighting: 40/40/20 applied to gamma-encoded RGB is the familiar
perceived-luminance approximation, not a linear-light luminance. Whether CS6
differs is *(inferred)*; the reference proposal is gamma-encoded.

### Mode-specific semantics

- **RGB:** three output channels; three source sliders + Constant each (12 values
  per non-monochrome output). Monochrome collapses to one triple.
- **CMYK:** output C/M/Y/K, sources C/M/Y/K + Constant. Used to rebalance inks
  and cap total ink (a documented CSS/Press workflow use).
- **No Lab / Grayscale / Indexed / Bitmap / Multichannel support** *(community
  sources; confirm per mode)*.

### Relationship to Black & White (`ADJ-012`)

Both convert colour to monochrome and both apply per-channel weights. The
differences:

- Channel Mixer weights are **linear source-channel contributions**; Black &
  White weights are applied to a **hue-sector decomposition** (see `ADJ-012`).
- They therefore produce different grays for the same conceptual intent, which is
  why CS6 ships both and the Help cross-references them.

### Placement in the pipeline

- Composite read; writes the active layer or feeds the adjustment-layer
  compositor.
- Per-pixel, order-independent, tileable.

## Rust module mapping

Proposals.

- `pictura_ops::adjust::channel_mixer` — `ChannelMixerSettings { output:
  OutputChannel, sources: Matrix, constant: i16, monochrome: bool }` where
  `Matrix` is mode-sized (3×3 or 4×4) and `OutputChannel` is an enum whose
  variants are gated by the document colour mode.
- `pictura_ops::adjust::channel_mixer::modes` — mode capability table
  (`ModeCaps { available: bool, outputs: &[OutputChannel], sources: &[Source] }`)
  so the UI and the operator share one source of truth. This is the same pattern
  required by `03-tools/dodge-burn-sponge.md` for per-mode availability.
- `pictura_core::command` — `ChannelMixerCommand { target, settings, mask,
  dirty_rect }`.
- `pictura_render::adjust` — optional GPU variant. Channel Mixer is in the CS6
  32-bpc supported list, so the GPU path must run in `f32`.

Crossing types: `AdjLayerId`, `LayerId`, `ColorSpace`, `OutputChannel`, `Matrix`,
`Rect`, `TileDelta`, `ChannelMixerSettings`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `ChannelMixerEditor` | `QWidget` | Output Channel combo, dynamic source-slider grid, Constant, Monochrome, Total/warning, Preset menu. |
| `MixerMatrixGrid` | `QGridLayout` in a `QWidget` | Rebuilds rows/columns when the document colour mode changes. |
| `TotalIndicator` | `QLabel` | Shows the per-output weight total and warns above 100%. |
| `AdjustmentPropertiesPanel` | `QStackedWidget` | Reused host; enables/disables per `ModeCaps`. |
| `AdjustmentLayerModel` | `QAbstractItemModel` | Reads/writes `ChannelMixerSettings`. |
| `PresetMenu` | `QMenu` | CS6 Channel Mixer presets. |

The editor must observe a document-mode signal and swap the slider set, mirroring
the mode capability table rather than hard-coding RGB.

## Data-model impact

- **Adjustment-layer node.** Stores `ChannelMixerSettings` (output channel,
  matrix, constant, monochrome, and — for the handtint path — the per-output
  triples) plus a mask.
- **Document-mode coupling.** The stored matrix is mode-specific; converting a
  document between RGB and CMYK must either remap or invalidate the mixer
  payload (`04-image-ops/image-modes.md`).
- **PSD serialization.** Adjustment-layer content via the layer's
  additional-layer-information block; exact tags *(inferred)*, tracked in
  `01-architecture/file-formats.md`.
- **Undo.** Destructive: one history state per committed edit. Adjustment layer:
  one per settings change.
- **No new channels / no alpha change.**

## Edge cases

- **Mode gating.** The command must be disabled in Grayscale, Lab, Indexed,
  Bitmap, and Multichannel (confirm each); never silently convert the document.
- **Monochrome toggle round-trip.** Selecting then deselecting Monochrome must
  preserve or reset per-output triples exactly as CS6 does; define the rule.
- **Negative weights** invert the source, which can push values out of range;
  clamp at the document bit depth, do not wrap.
- **Total > 100%** produces blown highlights by design; the UI warns, the
  operator does not clamp the intent.
- **8/16/32-bit.** In the CS6 32-bpc supported list; must run in `f32` at 32 bpc.
- **CMYK total-ink use case** requires K to be remixed knowingly; the default
  CMYK matrix is *(inferred)* and must be defined.
- **1-px / empty / huge documents.** Tile-based; no full-canvas scratch.
- **GPU unavailable.** CPU/GPU agreement within tolerance.
- **Undo mid-drag.** Coalesce scrub into one committed history state.

## Parity acceptance criteria

1. Given RGB and Output Channel = Green with sources R=50, G=100, B=0, an input
   `RGB(50,100,200)` yields `RGB(50,125,200)` (community-documented example).
2. Given Output Channel = Red with sources R=100, G=0, B=0, the output equals the
   input bit-exactly.
3. Given Monochrome on with 40/40/20, the output has R = G = B per pixel and
   matches the reference matrix within tolerance.
4. Given Monochrome off after a monochrome edit, per-output triples are editable
   and the handtint result differs from the monochrome result.
5. Given Constant = -50, the output is uniformly darker by the modeled offset;
   given +50, uniformly lighter, clamped at the document ceiling.
6. Given weights summing to exactly 100% for every output, a neutral gray input
   maps to the same gray within tolerance (no colour cast).
7. Given a Grayscale/Lab document, the adjustment is unavailable.
8. Given a CMYK document, the output channels are C/M/Y/K and the result is
   deterministic and in range.
9. Given a 32-bpc document, the adjustment is available and applied without
   integer quantization.
10. Given a committed edit, History shows exactly one new state and undo restores
    the prior pixels bit-exactly.
11. Given an adjustment-layer instance, opacity, blend mode, and mask modulate
    the result per `05-layers/adjustment-layers.md`.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  official CS6 Help reference; downloaded and text-extracted. Establishes:
  the Channel Mixer command summary ("Modifies a color channel … See Mix color
  channels"); that Channel Mixer / Photo Filter / Hue/Saturation are among the
  CS6 adjustment-layer presets; and that Channel Mixer is in the CS6 32-bpc
  supported-adjustment list. **The "Mix color channels" how-to page is not
  present in this PDF build** (only the cross-reference), so behavioral details
  below come from the mirror and community sources.
- `https://www.underwaterphotography.com/PhotoShop/PhotoShop/1_13_5_0.html` —
  a mirror of Adobe's official "Mixing color channels (Photoshop)" help text
  (older Photoshop generation). Establishes: the purpose list (creative
  adjustment, grayscale, sepia, YCbCr, swap/duplicate channels); the
  composite-channel requirement; `Image > Adjustments > Channel Mixer`; the
  Output Channel concept; source sliders -200%…+200% with negative inverting the
  source; Constant as a black/white channel; and Monochrome applying one setting
  to all outputs, with the select/deselect handtint note. Secondary mirror of a
  primary source.
- `https://www.photoshopessentials.com/photo-editing/black-and-white-tutorials/channel-mixer` —
  community tutorial. Establishes the Monochrome Output Channel becoming Gray,
  the default 40% Red / 40% Green / 20% Blue mix, the arrow-key 1%/Shift-10%
  steps, and the total-100% rule with a warning icon. Secondary source.
- `https://www.tourboxtech.com/en/news/channel-mixer.html` — community tutorial.
  Establishes the per-output matrix math (`out = Σ source·weight`) and that
  selecting an output channel initially sets R=0/G=100/B=0-family defaults.
  Secondary source.

Not fetched (HTTP 403 from this environment): `helpx.adobe.com` "Color and
monochrome adjustments using channels"; a search-result snippet from that page
(the output-channel reset-to-100% behaviour) agrees with the above but the page
itself was not retrieved. A community source stating the mixer is unavailable in
Lab/Grayscale was seen only as a search-result snippet; the original page was not
fetched.

## Open questions

- **The CS6 how-to page is missing from the reference PDF.** Whether this is an
  extraction artefact or a genuine omission is unresolved. Resolves with: another
  official CS6 Help build (e.g. an archived online Help page) or a page-by-page
  PDF inspection.
- **Exact Constant range** (assumed -200…+200). Resolves with: a CS6 panel
  capture.
- **Output Channel reset-on-select behaviour in CS6** (100% own source, 0
  others). Documented in current Adobe Help, not confirmed for CS6. Resolves
  with: a CS6 panel capture.
- **CMYK output/source channel names and default matrix.** Resolves with: a CMYK
  CS6 capture.
- **Mode availability for Multichannel / Bitmap / Indexed.** Resolves with:
  per-mode CS6 tests.
- **Operand space** (gamma-encoded vs linear) for the matrix. Resolves with: a
  CS6 pixel-accuracy test on a gray gradient.
- **Shipped Channel Mixer preset names.** Referenced by the CS6 preset list but
  not enumerated. Resolves with: the CS6 Preset menu capture.
- **Document-mode conversion semantics** for an existing mixer payload. Resolves
  with: an RGB↔CMYK round-trip test; feeds `04-image-ops/image-modes.md`.
- **PSD keys** for the adjustment-layer payload. Resolves with:
  `01-architecture/file-formats.md`.
