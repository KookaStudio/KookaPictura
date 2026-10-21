# HDR Toning

- **Spec ID:** `ADJ-025`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `No` — HDR Toning was introduced in Photoshop CS5 (with HDR Pro). CS6 keeps the command, adds new HDR Pro presets, and keeps the same four tone-mapping methods. The CS6 Properties panel is not involved (HDR Toning is a direct command, not an adjustment layer).
- **Depends on:** `ARCH-004` rust-qt-interop, `ARCH-008` document-model, `ARCH-009` undo-history, `04-image-ops/32-bit-hdr.md`, `04-image-ops/image-modes.md`, `04-image-ops/bit-depth-and-conversion.md`, `01-architecture/color-management.md`, `05-layers/smart-filters.md`

> All module and widget names below are **design proposals**. The repository
> contains a working implementation; this file stays the behavioural contract
> for the feature, and `docs/dev/doc-code-parity-gaps.md` records what shipped
> and what did not. Facts not confirmed by a fetched CS6 source are marked
> *(inferred)*.

## CS6 behavior

**HDR Toning** applies high-dynamic-range contrast/exposure tone mapping to a
single image. The CS6 Help: "The Exposure and HDR Toning adjustments are primarily
designed for 32-bit HDR images, but you can also apply them to 16- and 8-bit images
to create HDR-like effects." And: "The HDR Toning command lets you apply the full
range of HDR contrast and exposure settings to individual images."

- **Menu.** `Image > Adjustments > HDR Toning`.
- **Source depth.** "Open a 32-, 16-, or 8-bit image in RGB or Grayscale color
  mode." So unlike most adjustments it operates on 32-bpc float HDR data; it also
  works on 8/16-bit for faux-HDR.
- **Flattened layers required.** "Note: HDR toning requires flattened layers." The
  command is not available as an adjustment layer or Smart Filter.
- **Options apply at all depths.** "For detailed information about each setting, see
  Options for 16- or 8-bit images. (In the HDR Toning dialog box, these options
  apply to images of all bit depths.)" The same method menu and parameter tabs are
  presented regardless of source bit depth.
- **Methods (tone mapping).** The Help's tone-mapping method menu lists four:
  - **Local Adaptation** — "Adjusts HDR tonality by adjusting local brightness
    regions throughout the image." Exposes the full Edge Glow / Tone and Detail /
    Color / Toning Curve tabs.
  - **Equalize Histogram** — "Compresses the dynamic range of the HDR image while
    trying to preserve some contrast. No further adjustments are necessary; this
    method is automatic."
  - **Exposure and Gamma** — "Lets you manually adjust the brightness and contrast
    of the HDR image. Move the Exposure slider to adjust gain and the Gamma slider
    to adjust contrast."
  - **Highlight Compression** — "Compresses the highlight values in the HDR image
    so they fall within the luminance values range of the 8- or 16-bpc image file.
    No further adjustments are necessary; this method is automatic."
- **Presets.** A Preset menu offers ready-made looks (CS5-era secondary lists Flat,
  Monochromatic, More Saturated, Photorealistic, Saturated, Surrealistic; CS6 adds
  "New HDR Pro presets from RC Concepcion and Scott Kelby" per the What's-new list).
- **Local Adaptation controls** (Help wording):
  - **Edge Glow** — "Radius specifies the size of the local brightness regions.
    Strength specifies how far apart two pixels' tonal values must be before
    they're no longer part of the same brightness region."
  - **Tone and Detail** — "Dynamic range is maximized at a Gamma setting of 1.0;
    lower settings emphasize midtones, while higher settings emphasize highlights
    and shadows. Exposure values reflect f-stops. Drag the Detail slider to adjust
    sharpness and the Shadow and Highlight sliders to brighten or darken these
    regions."
  - **Color** — "Vibrance adjusts the intensity of subtle colors, while minimizing
    clipping of highly saturated colors. Saturation adjusts the intensity of all
    colors from –100 (monochrome) to +100 (double saturation)."
  - **Toning Curve** — "Displays an adjustable curve over a histogram showing
    luminance values in the original, 32-bit HDR image. The red tick marks along
    the horizontal axis are in one EV (approximately one f-stop) increments."
    With a **Corner** option: by default the curve "limit[s] and equalize[s] your
    changes from point to point"; selecting Corner after inserting a point removes
    the limit and makes the curve angular.
- **Related but distinct:** `View > 32-Bit Preview Options` (and the status-bar
  32-Bit Exposure slider) only adjust the *display* of a 32-bpc image and offer
  just Exposure and Gamma / Highlight Compression; they do not edit the file. The
  conversion `Image > Mode > 16 Bits/Channel` (or 8) presents the same four-method
  tone-mapping dialog to bake 32-bit HDR down to a lower depth.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Image > Adjustments > HDR Toning` | Menu command | — | Direct command; requires flattened layers |
| `Image > Mode > 16/8 Bits/Channel` on a 32-bpc doc | Conversion dialog | — | Same four-method dialog |
| `View > 32-Bit Preview Options` | Menu command | — | Display-only preview (Exposure and Gamma / Highlight Compression) |
| Status-bar 32-Bit Exposure slider | Slider | — | Display-only white point for 32-bpc |
| Dialog — Preset menu | Combo | — | Ready-made tone-mapping looks |
| Dialog — Method menu | Combo | Local Adaptation *(inferred)* | Four methods; parameter tabs gate on Local Adaptation |
| Dialog — Edge Glow tab | Sliders | — | Radius, Strength |
| Dialog — Tone and Detail tab | Sliders | — | Gamma, Exposure, Detail, Shadow, Highlight |
| Dialog — Color tab | Sliders | — | Vibrance, Saturation |
| Dialog — Toning Curve and Histogram tab | Curve editor | — | EV-tick histogram; Corner option |

## Parameters & ranges

The CS6 Help names the controls but prints no numeric ranges. Ranges/defaults below
come from CS5/CS6-era secondary tutorials and are *(inferred)* unless noted.

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Preset | enum | Default *(inferred)* | Default / Flat / Monochromatic / More Saturated / Photorealistic / Saturated / Surrealistic (+ CS6 presets) | Secondary |
| Method | enum | Local Adaptation *(inferred)* | Local Adaptation / Equalize Histogram / Exposure and Gamma / Highlight Compression | Gates the tabs below |
| Edge Glow Radius | pixels | e.g. 12–100 in tutorials *(inferred)* | unknown max *(inferred)* | Size of local brightness regions |
| Edge Glow Strength | float | e.g. 0.51–1.40 in tutorials *(inferred)* | unknown *(inferred)* | Edge separation threshold |
| Gamma | float | 1.0 (Help: dynamic range maximized at 1.0) | lower = midtones, higher = highlights/shadows *(inferred range)* | Tone and Detail |
| Exposure | EV / f-stops | 0 *(inferred)* | negative/positive EVs *(inferred)* | Gain; Help: "values reflect f-stops" |
| Detail | percent | ~+97 … +175 in tutorials *(inferred)* | unknown *(inferred)* | Sharpness / local detail |
| Shadow | percent | ~+27 … +45 in tutorials *(inferred)* | unknown *(inferred)* | Brighten/darken shadows |
| Highlight | percent | ~−15 … +23 in tutorials *(inferred)* | unknown *(inferred)* | Brighten/darken highlights |
| Vibrance | percent | 0 *(inferred)* | −100…+100 *(inferred)* | Subtle-color intensity, clipping-minimized |
| Saturation | percent | 0 *(inferred)* | **−100…+100 (Help: −100 monochrome, +100 double)** | All-color intensity |
| Toning Curve | control points | identity *(inferred)* | 0–255 / EV ticks; Corner toggle | Over luminance histogram |
| Exposure (Exposure and Gamma method) | float | 0 *(inferred)* | gain *(inferred)* | Manual method |
| Gamma (Exposure and Gamma method) | float | 1.0 *(inferred)* | contrast *(inferred)* | Manual method |

The Help explicitly documents only the Saturation range (−100…+100) and the Gamma
1.0 neutral point; every other bound is a secondary/inferred value and must be
calibrated against CS6.

## Algorithms & pipeline

Tone mapping is a standard HDR discipline; Adobe's exact kernels are closed.
Behavioral parity only; the models below are *(inferred)*.

### Input domain

- 32-bpc sources are linear-light floats with values `≥ 0` (often well above 1.0).
  HDR Toning should operate in linear light, then encode to the output.
- 8/16-bit sources are gamma-encoded and limited; the "HDR-like" path must still
  produce a plausible compression without inventing dynamic range.

### Local Adaptation

A local (edge-aware) operator, consistent with the Help's Edge Glow language:

1. **Base/detail decomposition.** Split luminance `L` into a blurred base
   `B = filter(L)` and detail `D = L / B` (or `L − B`). The bilateral/edge-aware
   filter radius is **Edge Glow Radius**; **Strength** controls the edge
   threshold so strong edges are preserved as detail and do not glow.
2. **Base compression.** Apply a tone curve to `B` (Shadow/Highlight/Gamma), then
   recombine `L' = B' · D^(detail_gain)`. **Detail** is the local-contrast gain;
   **Gamma** is the base power curve (`1.0` neutral); **Exposure** is a linear gain
   `2^EV`.
3. **Color.** Convert to a luma/chroma space; apply **Saturation** as a chroma
   scale, **Vibrance** as a chroma-dependent scale that attenuates change near high
   saturation to minimize clipping.
4. **Toning Curve.** A final 1-D remap `L'' = curve(L')` over the EV histogram
   (red ticks at 1 EV). The default curve is limited/equalized point-to-point;
   the Corner option removes the monotone-slope constraint, allowing angular
   segments.

A plausible base curve is a Reinhard/`L/(1+L)`-style compress; the exact function
is unknown and must be tuned. **Edge Glow halos** are the expected failure mode
when Radius/Strength are too large.

### Equalize Histogram (automatic)

Map luminance through its cumulative distribution (histogram equalization) to
spread tonal values across the output range, "trying to preserve some contrast."
No parameters.

### Exposure and Gamma (manual)

```
L' = ( L · 2^Exposure ) ^ (1 / Gamma)
```

Exposure is gain in stops; Gamma is contrast with `1.0` neutral. Clamp to the
output range.

### Highlight Compression (automatic)

Compress only the highlight end so the brightest values fall inside the 8/16-bpc
range (e.g. a log/Reinhard roll-off above a knee). No parameters.

### Output

- For `Image > Mode` conversion, the result is quantized/encoded to 16- or 8-bpc.
- For HDR Toning on a 32-bpc document that remains 32-bpc, the result is written
  back as float and is still previewed via the 32-Bit Preview Options.
- The Toning Curve histogram always refers to the original 32-bit HDR luminance,
  so the UI needs the pre-toning histogram (not the live output).

## Rust module mapping

- `pictura_adjust::hdr_toning` — `HdrToningOp { method: ToningMethod, local: Option<LocalAdaptationParams>, toning_curve: Option<Curve> }`.
- `pictura_adjust::hdr_toning::local` — `LocalAdaptationParams { edge_glow: EdgeGlow,
  tone_detail: ToneDetail, color: ColorParams }`; base/detail edge-aware filter.
- `pictura_adjust::hdr_toning::methods` — `equalize_histogram`, `exposure_gamma`,
  `highlight_compression` pure functions.
- `pictura_adjust::hdr_toning::histogram` — EV-binned luminance histogram used by
  the curve editor.
- `pictura_color` — linear-light conversion and luma/chroma spaces.
- `pictura_render::adjust` — GPU is attractive here (large blurs, float math), with
  a CPU reference to keep parity exact.

Crossing types: `HdrToningParams`, `ToningMethod`, `EdgeGlow { radius_px: f32, strength: f32 }`,
`ToneDetail { gamma, exposure_ev, detail, shadow, highlight }`, `ColorParams { vibrance, saturation }`,
`Curve` (shared with Curves).

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `HdrToningDialog` | `QDialog` | Preset + Method menus, tabbed parameter pages, Preview |
| `ToneCurveEditor` | `QWidget` | Curve over an EV-tick luminance histogram with Corner toggle (shared with Curves) |
| `EdgeGlowTab` / `ToneDetailTab` / `ColorTab` | `QWidget` | Parameter groups |
| `HistogramModel` | `QAbstractItemModel` | Pre-toning 32-bit luminance histogram data |
| `MethodComboModel` | `QAbstractItemModel` | Four tone-mapping methods; enables/disables tabs |

A modal `QDialog` with a `QTabWidget` matches CS6. The curve editor is reused from
Curves, extended with EV ticks and the Corner constraint toggle.

## Data-model impact

- **No layer.** HDR Toning is a direct destructive command; it flattens/requires a
  single layer. It is not an adjustment layer and not a Smart Filter.
- **32-bpc write-back.** On a 32-bpc document the result stays float; the document
  keeps 32-bpc storage and the preview settings remain separate.
- **Mode conversion.** `Image > Mode > 16/8 Bits/Channel` uses the same dialog and
  then changes the document bit depth; the conversion is a document-state change
  with its own history entry.
- **Undo.** One history state (tile deltas) per applied HDR Toning. Preset
  selection inside the dialog is transient until OK.
- **Presets.** Presets are named parameter blocks in the application preset store
  (`10-workflow-io/presets-manager.md`), not the PSD.
- **Histogram caching.** The EV histogram is computed from the source; cache and
  invalidate on flatten/bit-depth change.
- **Toning curve serialization.** A standard curve (point list) plus the Corner
  flags; version the record for future curve formats.

## Edge cases

- **Flattened-layers requirement** — on a multi-layer document, grey out the
  command or prompt to flatten; CS6 refuses without flattening.
- **32 / 16 / 8 bpc** — all three source depths allowed; the same tabs appear. On
  8/16-bit there is no true HDR range, so results are "HDR-like" and must not
  invent clipped detail.
- **Color modes** — the Help restricts to **RGB and Grayscale**; other modes
  (CMYK, Lab, Indexed, Bitmap, Multichannel) should be unavailable. Confirm.
- **Negative / zero luminance** — 32-bit data may contain 0; the base/detail
  division and log/gamma curves must guard against `log(0)` and `0/0`, using the
  preview white point for scaling.
- **Highlight Compression / Equalize Histogram** — automatic methods must be
  idempotent-safe and not oscillate; flat images must not divide by zero in the
  histogram CDF.
- **Toning Curve monotonicity** — the default limited/equalized mode must keep a
  monotone curve; Corner allows non-monotone segments by design.
- **Edge Glow halos** — large Radius/Strength produce halos, which are
  CS6-consistent artifacts, not bugs.
- **Color in Grayscale** — Vibrance/Saturation should be no-ops or hidden on
  Grayscale documents.
- **Preview vs committed** — cancelling must leave the document untouched; the
  32-bit preview settings are not modified by HDR Toning.
- **Empty / 1-px / huge (PSB) documents** — tile-local filters with correct halo
  margins; 1×1 degenerates gracefully; PSB float buffers can be very large, so
  stream tiles and avoid full-canvas float copies where possible.
- **GPU unavailable** — identical CPU result; float rounding differences must fall
  within tolerance or be avoided by keeping the reference path authoritative.
- **Undo/redo** — exact round-trip of the committed result at the document depth.
- **Memory** — float 32-bpc multi-tile blur can be memory-heavy; cap the edge-aware
  filter working set and spool to scratch.

## Parity acceptance criteria

- Given a 32-bpc HDR image, each of the four methods completes and produces an
  image whose highlights are inside the output range (for Highlight Compression and
  Equalize Histogram) without user parameters.
- Given `Gamma = 1.0` in Exposure and Gamma, the result differs from the source only
  by the Exposure gain; Exposure 0 / Gamma 1 is the identity within tolerance.
- Given Local Adaptation with a small Edge Glow Radius, fine detail is preserved and
  halos are small; increasing Radius/Strength produces larger local-contrast regions
  and visible halos, as described.
- Given Saturation at −100, the output is monochrome; at +100 the chroma is
  approximately doubled; at 0 it is neutral.
- Given Vibrance > 0, already-saturated colors change less than with an equal
  Saturation increase (clipping-minimized behavior).
- Given a monotone Toning Curve with Corner off, no segment can invert; enabling
  Corner after inserting a point permits a non-monotone (angular) segment.
- Given a multi-layer document, HDR Toning is unavailable until layers are
  flattened.
- Given a 16-bit RGB or Grayscale document, the command is available and the same
  parameter tabs are shown; given CMYK/Lab/Indexed, it is unavailable.
- Given `View > 32-Bit Preview Options`, changing the preview does not modify pixel
  data and is preserved after cancel/OK.
- Given an applied HDR Toning, History records exactly one new state and undo
  restores the prior pixels exactly at the document bit depth.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  official CS6 Help reference; downloaded and text-extracted. Establishes: the
  Exposure/HDR-Toning purpose statement; "HDR toning requires flattened layers";
  the 32/16/8-bit RGB/Grayscale source rule; the HDR Toning menu path; the
  cross-reference that all options follow "Options for 16- or 8-bit images" and
  apply at all bit depths; the four tone-mapping methods with their quoted
  descriptions; the Edge Glow Radius/Strength semantics; the Tone and Detail
  Gamma/Exposure/Detail/Shadow/Highlight semantics; the Color Vibrance and
  Saturation (−100…+100) semantics; the Toning Curve EV-tick histogram and Corner
  behavior; the View > 32-Bit Preview Options display-only methods; the CS6 What's-
  new HDR Pro preset note.
- `https://www.photoshoproadmap.com/easily-enhance-your-photos-with-hdr-toning-in-photoshop-cs5`
  — CS5/CS6-era secondary source: confirms the **Method** menu with **Local
  Adaptation**, and example parameter values (Edge Glow Radius 12–40 px / Strength
  0.51–1.40; Gamma 1.10–1.60; Exposure −1.00; Detail +97…+175 %; Shadow +27…+30 %;
  Highlight −5…+23 %; Vibrance +30…+70 %; Saturation +20 %) used only to bound the
  inferred ranges.
- `https://www.astronomy.com/science/tone-your-image-using-hdr` (May 2012) —
  secondary source for the HDR Toning dialog: "Preset = Default; Method = Local
  Adaptation", Edge Glow Radius/Strength guidance (~100 / ~0.50), and the five Tone
  and Detail sliders (Gamma, Exposure, Detail, Shadow, Highlight).
- `https://www.sitepoint.com/simulating-high-dynamic-range-hdr-with-photoshop`
  (March 2012) — secondary source for the same controls and Edge Glow/Tone-Detail
  descriptions, plus preset names and the Merge to HDR Pro context.
- `https://astropix.com/html/processing/ps_hdr.html` — secondary source confirming
  the four-method HDR conversion dialog (Exposure and Gamma, Highlight Compression,
  Equalize Histogram, Local Adaptation) on 32→16-bit conversion.

Not parsed in this pass: `helpx.adobe.com` (HTTP 403 from this environment).

## Open questions

- **Control ranges and defaults.** Every numeric bound except Saturation
  (−100…+100) and the Gamma 1.0 neutral is unverified. Resolves with: a full CS6
  HDR Toning dialog capture (all tabs, slider endpoints).
- **Method default and gating.** Whether the CS6 single-image dialog defaults to
  Local Adaptation and which tabs appear for the automatic methods. Resolves with:
  a CS6 dialog capture.
- **Local Adaptation kernel.** The base/detail filter (bilateral? guided?), the
  Detail gain curve, and Shadow/Highlight curves are closed. Resolves with: fitting
  to CS6 on a controlled HDR test chart.
- **Equalize Histogram / Highlight Compression math.** Automatic and undocumented.
  Resolves with: a histogram comparison before/after in CS6.
- **Preset parameter values.** The named presets' exact settings. Resolves with: CS6
  preset files (`.psp`-style or embedded).
- **32-bpc write-back semantics.** What HDR Toning stores when the document stays
  32-bpc (full-float result vs a preview-only change). Resolves with: a CS6 32-bpc
  save/compare.
- **Color-mode restriction.** Confirm RGB + Grayscale only; whether Lab is offered.
  Resolves with: a CS6 mode test.
- **Grayscale Color tab.** Whether Vibrance/Saturation are hidden or no-ops on
  Grayscale. Resolves with: a CS6 Grayscale capture.
- **EV tick calibration.** Whether the Toning Curve EV axis is anchored to the
  document preview white point or a fixed `1.0`. Resolves with: a CS6 curve probe.
- **GPU float tolerance.** Whether GPU and CPU tone-mapping results can meet a tight
  parity tolerance or the CPU path must be normative.
