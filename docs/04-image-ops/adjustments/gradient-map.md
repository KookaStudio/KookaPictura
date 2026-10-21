# Gradient Map Adjustment

- **Spec ID:** `ADJ-015`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `Yes` (library) — the Gradient Map adjustment is long-standing, but the CS6 "What's New" list includes "Added new Gradient Map presets for traditional print toning and split-toning." CS6 also moves the panel UI to the Properties panel.
- **Depends on:** `ARCH-008` document-model, `ARCH-009` undo-history, `ARCH-006` gpu-rendering-pipeline, `01-architecture/color-management.md`, `04-image-ops/image-modes.md`, `04-image-ops/adjustments-overview.md`, `05-layers/adjustment-layers.md`, `05-layers/blend-modes.md`, `07-color-painting/color-models.md`, `07-color-painting/gradient-presets.md`, `10-workflow-io/presets-manager.md`.

> All module and widget names below are **design proposals**. The repository
> contains a working implementation; this file stays the behavioural contract
> for the feature, and `docs/dev/doc-code-parity-gaps.md` records what shipped
> and what did not. Facts not confirmed by a fetched CS6 source are marked
> *(inferred)*.

## CS6 behavior

**Gradient Map** "maps the equivalent grayscale range of an image to the colors
of a specified gradient fill. If you specify a two-color gradient fill, for
example, shadows in the image are mapped to one of the endpoint colors of the
gradient fill, highlights are mapped to the other endpoint color, and midtones
are mapped to the gradations in between."

- **Two invocation paths** — `Image > Adjustments > Gradient Map` (destructive;
  the Help warns it "applies the adjustment directly to the image layer and
  discards image information") and `Layer > New Adjustment Layer > Gradient Map`
  (non-destructive, with a mask).
- **Default mapping** — "By default, the shadows, midtones, and highlights of the
  image are mapped respectively to the starting (left) color, midpoint, and
  ending (right) color of the gradient fill."
- **Gradient picker** — "To choose from a list of gradient fills, click the
  triangle to the right of the gradient fill. Click to select the desired
  gradient fill, and then click in a blank area of the … panel to dismiss the
  list." The list is the shared gradient preset library (Preset Manager).
- **Gradient Editor** — "To edit the currently-displayed gradient fill, click the
  gradient fill, and then modify the existing gradient fill or create a gradient
  fill in the Gradient Editor." Editing covers color stops, opacity stops,
  midpoints, smoothness, and the interpolation colour space.
- **Dither** — "Adds random noise to smooth the appearance of the gradient fill
  and reduces banding effects."
- **Reverse** — "Switches the direction of the gradient fill, reversing the
  gradient map."
- **CS6 preset library** — new print-toning and split-toning presets shipped with
  CS6 (names not enumerated in the Help; see `## Open questions`).
- **Presets** — the adjustment is not in the Help's named Properties-panel Preset
  group; the gradient itself is the reusable asset and lives in the gradient
  library.

### Per-channel and compositing behaviour

- **Per-channel gradients.** A gradient ramp is inherently per-channel: at
  position `t` the ramp yields output `(R(t), G(t), B(t), A(t))` (and, for
  document modes with more channels, an equivalent mapping). The Gradient Editor
  lets each colour stop carry a full colour, so the ramp's three (or four)
  channel curves are independent. Applying the map to **one** document channel
  (e.g. selecting a single channel in the Channels panel and using
  `Image > Adjustments > Gradient Map`) requires confirmation; the CS6 Help's
  how-to does not document it *(inferred; see Open questions)*. The
  adjustment-layer form maps the composite luminance.
- **Blend with underlying.** As an adjustment layer, the mapped result feeds the
  layer's **blend mode**, **opacity**, **fill**, **blend-if** ranges, and mask
  (`05-layers/adjustment-layers.md`). At 100% opacity Normal the map fully
  replaces the underlying colour; below 100% the mapped colour blends with the
  underlying colour by the layer opacity. In addition, the gradient's **opacity
  stops** produce partial alpha at that ramp position, letting the underlying
  image show through where the gradient is not fully opaque. Dither and Reverse
  change the mapping, not the blend.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Image > Adjustments > Gradient Map` | Menu command | — | Destructive direct edit. |
| `Layer > New Adjustment Layer > Gradient Map` | Menu command | — | Non-destructive adjustment layer with mask. |
| Adjustments / Properties panel | Panel | — | Gradient swatch + dropdown, Dither, Reverse. |
| Properties panel — gradient dropdown | Pop-up | — | Shared gradient preset library; Preset Manager-editable. |
| Gradient Editor | Dialog | — | Edit stops, midpoints, smoothness, per-channel colours, interpolation. |
| `Window > Gradient` / `Gradient Presets` | Panel | — | Shared gradient assets. |
| Layers panel | Context | — | Blend mode, opacity, fill, blend-if, mask. |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Gradient | gradient asset | foreground→background *(inferred)* | any gradient preset or custom | Picker list + Gradient Editor. |
| Colour stops | list | per gradient | position 0–100%, colour | Per-channel output defined here. |
| Opacity stops | list | per gradient | position 0–100%, opacity 0–100% | Partial opacity lets the underlying show. |
| Stop midpoint | int (percent) | 50 | 1 … 99 *(inferred)* | Bias between adjacent stops. |
| Smoothness | int (percent) | 100 | 0 … 100 *(inferred)* | Gradient Editor interpolation smoothing. |
| Dither | bool | Off *(inferred)* | on / off | Random noise to break banding. |
| Reverse | bool | Off *(inferred)* | on / off | Swaps gradient direction. |
| Layer blend mode | enum | Normal | CS6 blend list | Adjustment-layer compositing. |
| Layer opacity / fill | int (percent) | 100 / 100 | 0 … 100 | Blends mapped result with underlying. |

## Algorithms & pipeline

### Core mapping

For each pixel, compute the source luminance `L ∈ [0,1]` ("the equivalent
grayscale range of an image"), optionally reverse it (`L ← 1 − L` when Reverse is
on), optionally perturb it for dither, then sample the gradient:

`out = gradient(L)`

- The gradient is a piecewise interpolation through its colour stops; between
  stops it uses the Gradient Editor's colour model and smoothness. The exact
  interpolation space (sRGB vs linear-light, and the smoothness function) is
  *(inferred)*.
- For a two-stop black→white ramp, `out = mix(stop0, stop1, L)`; for a
  shadow→highlight toning ramp, the midtones land on the intermediate stops.
- The Help's "shadows → starting (left), midtones → midpoint, highlights →
  ending (right)" is the default identity placement; custom stop positions move
  the tonal breakpoints.

### Which luminance

The CS6 Help says "equivalent grayscale range" without naming the space. A
reference implementation computes `L` as the document working-space luminance
(Rec. 709 for sRGB-like spaces, Lab `L*/100` for Lab, the ink/`K`-derived
lightness for CMYK). The choice is *(inferred)* and must be fixed and tested, as
it changes toning placement.

### Dither

Dither adds a small random (or ordered) perturbation to `L` before sampling to
break up quantisation banding in smooth gradients. The amplitude and the noise
distribution are *(inferred)*; a one-LSB triangular-PDF dither is a standard
reference. Dither must be deterministic if reproducibility is required (seed
policy is a design decision).

### Per-channel form

If (and only if) the adjustment is applied with a single document channel
targeted, the same ramp samples that channel's value instead of the composite
luminance, producing a per-channel curve. Applying a Gradient Map adjustment
**layer** always maps the composite. This distinction — and whether CS6 supports
targeting a single channel via `Image > Adjustments` — is *(inferred)*.

### Toning presets (CS6)

"Traditional print toning and split-toning" presets are ordinary gradients whose
stops reproduce darkroom selenium/sepia/cyanotype toning and split-toning
(a warm highlight/neutral shadow or vice versa). Their exact stop colours are
not published in the Help; they ship in the gradient library
(`07-color-painting/gradient-presets.md`).

### Placement in the pipeline

- Composite luminance read; writes the active layer (destructive) or feeds the
  adjustment-layer compositor (blend/opacity/fill/mask/blend-if).
- Per-pixel and order-independent apart from the dither seed; the deterministic
  (no-dither) path is tileable.

## Rust module mapping

Proposals.

- `pictura_ops::adjust::gradient_map` — `GradientMapSettings { gradient:
  GradientId, dither: bool, reverse: bool }` and the `PixelEdit` implementation.
- `pictura_ops::adjust::gradient_map::sample` — `Gradient::sample(t) -> Color`
  with the interpolation model; shared with `ADJ-015`'s relatives (Gradient Fill
  layer, gradient tool) via `07-color-painting/gradient-presets.md`.
- `pictura_color::luma` — the single luminance definition used by Gradient Map,
  Color Balance (`ADJ-011`), and Photo Filter (`ADJ-013`).
- `pictura_color::gradient` — gradient asset type (stops, midpoints, smoothness,
  interpolation space), loadable from the preset library.
- `pictura_core::command` — `GradientMapCommand { target, settings, mask,
  dirty_rect }`.
- `pictura_render::adjust` — optional GPU variant.

Crossing types: `AdjLayerId`, `LayerId`, `ColorSpace`, `GradientId`, `Gradient`,
`Rect`, `TileDelta`, `GradientMapSettings`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `GradientMapEditor` | `QWidget` | Gradient swatch + dropdown, Dither, Reverse, Reset. |
| `GradientRampWidget` | custom `QWidget` | Paints a `Gradient`; used by the editor, the gradient tool, and fill layers. |
| `GradientEditorDialog` | dialog | Stop editing, per-channel colours, opacity stops, midpoint, smoothness; reused asset editor. |
| `GradientPresetModel` | `QAbstractItemModel` | Shared gradient library. |
| `AdjustmentPropertiesPanel` | `QStackedWidget` | Reused host. |
| `AdjustmentLayerModel` | `QAbstractItemModel` | Reads/writes `GradientMapSettings`; layer blend/opacity/fill live on the layer node. |

## Data-model impact

- **Adjustment-layer node.** Stores `GradientMapSettings` (a gradient reference,
  Dither, Reverse) plus a mask; blend mode, opacity, fill, and blend-if are
  layer-level fields (`05-layers/layers-overview.md`).
- **Gradient asset.** `Gradient` is a first-class shared asset (stops, midpoints,
  smoothness, interpolation, opacity stops) used by the tool, fill layers, and
  this adjustment. CS6 press/toning presets ship in the library.
- **PSD serialization.** Adjustment-layer content via the layer's
  additional-layer-information block; the gradient may be stored inline or by
  reference. Exact tags *(inferred)*, tracked in
  `01-architecture/file-formats.md`.
- **Undo.** Destructive: one history state per committed edit. Adjustment layer:
  one per settings change; editing a shared gradient asset is a library change
  with its own history semantics (define).
- **No new channels / no alpha change.**

## Edge cases

- **8/16/32-bit.** Gradient Map is **not** in the CS6 32-bpc supported list, so
  at 32 bpc it must be unavailable; 8/16-bit supported. Dither must scale its
  amplitude to the document bit depth.
- **CMYK / Lab.** The luminance used to index the ramp must be defined per mode;
  output stops are converted into the document space, and CMYK output can go
  out of gamut — clamping behaviour must be explicit.
- **Grayscale.** Natural fit; output may be colour if the gradient is colour,
  which requires the document to be RGB (define behaviour in Grayscale).
- **Bitmap / Indexed / Multichannel.** Expected unavailable/inert.
- **Reverse** must be equivalent to sampling `1 − L`; verify on asymmetric ramps.
- **Dither reproducibility.** Decide seeded vs unseeded; snapshots/history must
  not change between renders of the same state.
- **Gradient with >2 stops** must place midtones on the correct interior stop, not
  linearly between the endpoints.
- **Opacity stops** create transparency in the mapped output; the adjustment-layer
  compositor must honour them rather than treating the result as opaque.
- **1-px / empty / huge documents.** Tile-based; no full-canvas scratch.
- **GPU unavailable.** CPU/GPU agreement within tolerance (dither excepted unless
  the seed/pattern is shared).
- **Undo mid-drag.** Coalesce scrubbing into one committed history state.

## Parity acceptance criteria

1. Given a grayscale ramp and a black→white gradient, the output is the same ramp
   within tolerance (identity mapping).
2. Given a two-colour gradient, input black maps to the left endpoint and input
   white to the right endpoint; mid-gray maps to the ramp midpoint.
3. Given **Reverse** on, black maps to the right endpoint and white to the left.
4. Given a three-stop toning gradient, a midtone input maps to the interior stop
   rather than a linear blend of the outer endpoints.
5. Given **Dither** on, a smooth ramp does not band at 8-bit output; off, banding
   is visible; dithered output does not shift the mean luminance beyond a small
   tolerance.
6. Given a colour gradient in a Grayscale document, either the output is rendered
   per the documented rule or the adjustment is unavailable — no silent document
   conversion.
7. Given an adjustment-layer instance at 50% opacity, the output is a 50/50 blend
   of the mapped colour and the underlying colour within tolerance; a gradient
   opacity stop below 100% lets the underlying show through.
8. Given a 32-bpc document, the adjustment is unavailable.
9. Given a committed edit, History shows exactly one new state and undo restores
   the prior pixels bit-exactly.
10. Given an adjustment-layer instance, blend mode, fill, blend-if, and mask
    modulate the result per `05-layers/blend-modes.md` and
    `05-layers/adjustment-layers.md`.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  official CS6 Help reference; downloaded and text-extracted. Establishes:
  "Apply a gradient map to an image" (p. 292): the grayscale-mapping definition
  and two-colour example; the destructive vs adjustment-layer paths; the gradient
  picker dropdown and dismissal behaviour; the Gradient Editor edit path; the
  default shadow→start / midtone→midpoint / highlight→end mapping; **Dither**
  ("random noise to smooth the appearance … reduces banding effects"); and
  **Reverse**. The CS6 "What's New" list (p. 10) establishes "Added new Gradient
  Map presets for traditional print toning and split-toning." The 32-bpc
  supported list establishes that Gradient Map is not available at 32 bpc.
- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  same file, "Create a fill layer" (p. 288): establishes the shared gradient
  options (Style, Angle, Scale, Reverse, Dither, Align With Layer) for the
  Gradient **fill**, which shares the `Gradient` asset but is a distinct feature.

Not fetched (HTTP 403 from this environment): `helpx.adobe.com` gradient-map
pages.

## Open questions

- **Luminance definition** used to index the ramp ("equivalent grayscale range").
  Resolves with: a CS6 toning test on a hue/saturation grid.
- **Per-channel application** — whether CS6 supports applying a Gradient Map to a
  single channel via `Image > Adjustments` with a channel targeted. Resolves
  with: a CS6 Channels-panel test.
- **Gradient interpolation space and smoothness function** in the Gradient
  Editor. Resolves with: a CS6 ramp sample extraction.
- **Dither algorithm, amplitude, and seed policy.** Resolves with: a CS6
  banding test and a noise-statistics comparison.
- **Shipped CS6 toning/split-toning preset names and stops.** Resolves with: the
  CS6 gradient library.
- **Default gradient** for a freshly created Gradient Map (foreground→background
  is assumed). Resolves with: a CS6 first-run capture.
- **Blend-if and fill interaction** specific to Gradient Map layers. Resolves
  with: a CS6 compositing test; feeds `05-layers/blend-modes.md`.
- **PSD keys** for the adjustment-layer payload and inline-vs-referenced gradient.
  Resolves with: `01-architecture/file-formats.md`.
