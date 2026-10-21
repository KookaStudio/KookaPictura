# 32-bit HDR Workflow

- **Spec ID:** `IMG-009`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `Changed` — CS6 enables **Invert and Threshold adjustments for masks in 32-bit/channel images** and adds **new HDR Pro presets**; the Merge To HDR Pro command and 32-bit working mode themselves date from CS5.
- **Depends on:** `04-image-ops/bit-depth-and-conversion.md`, `04-image-ops/image-modes.md`, `04-image-ops/adjustments/exposure.md`, `04-image-ops/adjustments/hdr-toning.md`, `04-image-ops/adjustments/levels.md`, `04-image-ops/adjustments/hue-saturation.md`, `06-filters/blur-filters.md`, `01-architecture/file-formats.md` (`ARCH-011`), `01-architecture/color-management.md` (`ARCH-007`), `01-architecture/gpu-rendering-pipeline.md`, `09-automation/batch-processing.md`, `10-workflow-io/file-info-and-metadata.md`.

> All module, crate, and widget names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Statements marked *(inferred)* are not taken from a
> fetched source and are candidates for `## Open questions`.

## CS6 behavior

### About HDR

- In Photoshop, the luminance of an HDR image is stored as **32-bit floating
  point per channel**. Values relate directly to scene light, unlike 8/16-bit
  files fixed to black→paper white.
- `File > Automate > Merge To HDR Pro` (also Bridge: `Tools > Photoshop > Merge
  To HDR Pro`) combines multiple exposures of the same scene into one HDR image.
- Because HDR exceeds a standard monitor's dynamic range, Photoshop provides a
  display preview; convert to 16/8-bit when a tool/filter does not support 32-bit.

### Merge To HDR Pro

1. `Browse` / `Add Open Files` / `Use > Folder` to choose source images; `Remove`
   removes one.
2. `Attempt To Automatically Align Source Images` (hand-held shots).
3. `OK`. If images lack exposure metadata, the **Manually Set EV** dialog
   appears.
4. The second dialog shows thumbnails, a merged preview, and a **bit depth**
   choice: `32 Bit` stores the entire dynamic range; 8/16-bit cannot.
5. Adjust tonal range (32-bit or 16/8-bit options, below).
6. `Preset > Save Preset` / `Load Preset`.
7. `Remove Ghosts` handles moving objects, outlining the base image in green.
8. **Response curves**: calculated automatically from the merged images; `Save
   Response Curve` / `Load Response Curve` via the dialog's response-curve menu.

### Options for 32-bit images

- A slider under the histogram adjusts the **white point preview only**; all HDR
  data remains in the file. The preview adjustment is **stored in the HDR file**
  (PSD, PSB, TIFF) and applied whenever the file is opened in Photoshop; readjust
  via `View > 32-Bit Preview Options`.

### Options for 16- or 8-bit tone mapping

Choose one tone-mapping method:

- **Local Adaptation** — adjusts local brightness regions.
  - `Edge Glow Radius` (size of local brightness regions) and `Strength` (tonal
    separation needed to split regions).
  - `Tone and Detail`: `Gamma` (1.0 maximises dynamic range; lower emphasises
    midtones, higher emphasises highlights/shadows), `Exposure` (in f-stops),
    `Detail` (sharpness), `Shadow`, `Highlight`.
  - `Color`: `Vibrance` (subtle colors, minimise clipping) and `Saturation`
    (−100 monochrome … +100 double).
  - `Toning Curve` — an adjustable curve over a histogram of the original 32-bit
    luminance; red ticks are 1 EV apart. By default the curve limits/equalises
    changes point-to-point; the `Corner` option removes the limit for more
    extreme, angular adjustments.
- **Equalize Histogram** — compresses dynamic range while preserving some
  contrast; automatic.
- **Exposure and Gamma** — manual: `Exposure` (gain) and `Gamma` (contrast).
- **Highlight Compression** — compresses highlights into the 8/16-bit
  luminance range; automatic.

### Adjust displayed dynamic range

- `View > 32-Bit Preview Options` → Method `Exposure And Gamma` (sliders) or
  `Highlight Compression`.
- Document **status bar** `32-Bit Exposure` slider sets a per-view white point;
  double-click resets; per-view settings are **not** stored in the file.
- Info panel can show **32-Bit** readouts (Eyedropper icon → 32-Bit).

### HDR Color Picker

- Regular picker plus an **Intensity** slider (brightness), 32-bit RGB floating
  fields, a Preview area of swatches at different exposures, `Preview Stop Size`,
  `Relative to Document`, and `Add to Swatches`. Intensity stops correspond
  inversely to exposure stops.

### Painting/editing support

- Editable with Brush, Pencil, Pen, Shape, Clone Stamp, Pattern Stamp, Eraser,
  Gradient, Blur, Sharpen, Smudge, History Brush, and the Text tool.

### 32-bpc feature availability matrix

| Area | Supported in 32-bpc |
|---|---|
| Adjustments | Levels, Exposure, Hue/Saturation, Channel Mixer, Photo Filter |
| Blend modes | Normal, Dissolve, Darken, Multiply, Lighten, Darker Color, Linear Dodge (Add), Lighter Color, Difference, Subtract, Divide, Hue, Saturation, Color, Luminosity |
| New documents | 32-bit option in the New dialog's bit-depth menu |
| Edit menu commands | All (Fill, Stroke, Free Transform, Transform, …) |
| File formats | Photoshop (PSD, PSB), Radiance (HDR), Portable Bit Map (PBM), OpenEXR, TIFF; can **read** LogLuv TIFF but not save it |
| Filters | Average, Box/Gaussian/Motion/Radial/Shape/Surface Blur, Add Noise, Clouds, Difference Clouds, Lens Flare, Smart Sharpen, Unsharp Mask, Emboss, De-Interlace, NTSC Colors, High Pass, Maximum, Minimum, Offset |
| Image commands | Image Size, Canvas Size, Image Rotation, Crop, Trim, Duplicate, Apply Image, Calculations, Variables |
| View | Pixel Aspect Ratio commands |
| Layers | New/duplicate layers, adjustment layers (Levels, Vibrance, Hue/Saturation, Channel Mixer, Photo Filter, Exposure), fill layers, layer masks, layer styles, supported blending modes, Smart Objects |
| Modes | RGB Color, Grayscale, conversion to 8/16 Bits/Channel |
| Selections | Invert, Modify Border, Transform Selection, Save/Load Selection |
| Tools | All toolbox tools **except**: Magnetic Lasso, Magic Wand, Spot Healing Brush, Healing Brush, Red Eye, Color Replacement, Art History Brush, Magic Eraser, Background Eraser, Paint Bucket, Dodge, Burn, Sponge (some tools work with supported blend modes only) |
| Masks | Invert and Threshold adjustments **newly enabled in CS6** |

### Convert 32 → 16/8

`Image > Mode > 16 Bits/Channel` or `8 Bits/Channel`, then choose a tone-mapping
method and `OK`.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `File > Automate > Merge To HDR Pro` | Menu + dialog | none documented | Merge exposures. |
| Merge To HDR Pro dialog | Dialog | — | Browse/Add/Use Folder, Align, Remove Ghosts, bit depth, tone mapping, response curve, presets. |
| Manually Set EV dialog | Dialog | — | Shown when EXIF exposure is missing. |
| `View > 32-Bit Preview Options` | Dialog | none documented | Method + Exposure/Gamma. |
| Document status bar | Drop-down | — | `32-Bit Exposure` per-view slider (not stored). |
| HDR Color Picker | Dialog | — | Intensity slider, 32-bit RGB fields, preview. |
| `Image > Mode > 16/8 Bits/Channel` | Menu + dialog | — | Tone-map to LDR. |
| Info panel | Panel | — | 32-Bit readout mode. |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Source images | file list | — | Browse / open docs / folder | Merge inputs. |
| Attempt To Automatically Align Source Images | bool | off | on / off | Hand-held shots. |
| Remove Ghosts | bool | off | on / off | Moving-object removal. |
| Bit depth | combo | 32 Bit *(inferred)* | 32 / 16 / 8 | 16/8 triggers tone mapping. |
| White point preview | slider | — | luminance range | Stored in file (32-bit option). |
| Tone method (16/8) | combo | Local Adaptation *(inferred)* | Local Adaptation / Equalize Histogram / Exposure and Gamma / Highlight Compression | Perf mode unavailable for Equalize/Highlight. |
| Edge Glow Radius | slider | — | not documented | Local Adaptation. |
| Strength | slider | — | not documented | Local Adaptation. |
| Gamma | slider | 1.0 (max DR) | not documented | Tone and Detail. |
| Exposure | slider | 0 EV *(inferred)* | f-stops | Tone and Detail. |
| Detail | slider | — | not documented | Tone and Detail. |
| Shadow / Highlight | sliders | — | not documented | Local Adaptation. |
| Vibrance | slider | — | not documented (clipping-minimising) | Color. |
| Saturation | slider | 0 *(inferred)* | −100…+100 | Color. |
| Toning Curve | curve | identity | 13-point-class; `Corner` toggle | With histogram, 1 EV ticks. |
| Preview method | radio | Exposure And Gamma *(inferred)* | Exposure And Gamma / Highlight Compression | View > 32-Bit Preview Options. |
| Preview Exposure / Gamma | sliders | — | not documented | |
| Intensity (HDR picker) | slider | 0 stops *(inferred)* | stops vs exposure | Inverse to exposure. |
| Preview Stop Size | int | 3 *(inferred)* | not documented | |
| Relative to Document | bool | off *(inferred)* | on / off | Preview swatches. |
| 32-Bit Exposure (status bar) | slider | 0 *(inferred)* | per-view | Not stored. |

## Algorithms & pipeline

- **Merge.** Camera response curve recovery + radiance merge. The canonical
  formulations are **Debevec & Malik (1997)** (estimate inverse CRF via a
  smoothness-regularised least-squares solve, then a weighted average of
  linearised exposures) and **Robertson (1999)** (iterative CRF estimate).
  Photoshop's response curve is "automatically calculated" and `Save/Load
  Response Curve` exchanges it. **Behavioural parity only, algorithm TBD**;
  the industry-standard methods are the modelling reference.
- **Alignment.** Automatic alignment of hand-held brackets (feature/translation
  based); exact Adobe method closed.
- **Ghost removal.** Choose a base exposure (best tonal balance, green outline)
  and take moving-object pixels from it.
- **Tone mapping.** Global operators: `Exposure and Gamma` (gain/contrast),
  `Highlight Compression`, `Equalize Histogram` (histogram equalisation). Local
  operator: `Local Adaptation` (edge-aware local brightness regions plus the
  Toning Curve). Adobe's Local Adaptation is closed; published analogous local
  operators include Reinhard, Drago, and Mantiuk. **Behavioural parity only,
  algorithm TBD.**
- **Display preview.** A separate exposure/gamma (or highlight-compression)
  transform maps HDR to the monitor; stored (32-bit options) or per-view (status
  bar). Never edits pixel data.
- **Feature gating** follows the matrix above; unsupported commands are disabled
  in 32-bpc documents.

## Rust module mapping

Proposed:

- `pictura-core::hdr` — `HdrImage` (planar/interleaved `f32`), `merge_hdr(inputs: &[Exposure], opts: MergeOptions) -> HdrImage`.
- `MergeOptions { align: bool, remove_ghosts: bool, response: Option<ResponseCurve>, ev: Option<Vec<f32>> }`.
- `ResponseCurve` — per-channel lookup, loadable/savable.
- `ToneMapOperator { LocalAdaptation(LocalAdaptationParams), EqualizeHistogram, ExposureGamma { exposure, gamma }, HighlightCompression }`.
- `PreviewOptions { method, exposure, gamma }`.
- `pictura-core::document::HdrPreview` — white-point preview stored in the
  document (PSD/PSB/TIFF).
- `pictura-core::color::hdr_picker` — 32-bit float color selection.
- Reuse `adjust::hue_saturation`, `adjust::levels`, `05-layers/blend-modes` for
  the supported subset.

## Qt6 component mapping

- `MergeToHdrProDialog` (`QDialog`) — file list + Browse/Add/Folder, Align,
  Remove Ghosts, bit-depth combo, tone-mapping params (stacked pages per method),
  response-curve menu, preset menu.
- `ManuallySetEvDialog` — per-image EV entry.
- `HdrPreviewOptionsDialog`, `HdrColorPicker` (with Preview swatches),
  `HdrStatusBarExposure` widget.
- `ToneMappingPanel` — Local Adaptation sliders + `ToningCurveWidget` (shared
  with Curves).
- All HDR preview/compute runs on the GPU where available (`QRhi`/`wgpu`);
  a CPU fallback must exist (`ARCH-010`).

## Data-model impact

- `DocumentBitDepth = 32` with `f32` pixel buffers; the model must not clamp
  intermediates.
- The 32-bit **preview white point** is stored in PSD/PSB/TIFF. PSD resource
  **1070** (`HDR Toning information`, Photoshop CS2) exists for HDR toning data;
  whether CS6 uses it for the preview is unverified.
- `Image > Mode > 16/8 Bits/Channel` is a destructive command; undo stores a
  pre-image (or the tone-map parameters plus pre-image), per `ARCH-009`.
- Merge To HDR Pro may run as an `09-automation` merge job; it reads exposure
  metadata (EXIF) from each source (`10-workflow-io/file-info-and-metadata.md`).
- `Save/Load Preset` and `Save/Load Response Curve` are separate files, not
  document state.

## Edge cases

- **Missing exposure metadata** → Manually Set EV dialog.
- **Moving objects** → Remove Ghosts; very light/dark movement may need a
  different base thumbnail.
- **Alignment failures** on large motion.
- **Monitor cannot display HDR** → preview only; never modifies data.
- **GPU unavailable** → CPU preview/tone-map fallback; 32-bit features that
  depend on MGE must degrade explicitly.
- **Unsupported tools/filters/adjustments** in 32-bpc are disabled; convert to
  16/8-bit (destructive) to access them.
- **LogLuv TIFF** is read-only.
- **Huge HDR / PSB**: merge and tone-map by tile; avoid multiple full float
  buffers.
- **Saving to formats that cannot hold 32-bit float** (PNG/JPEG) must go through
  an 8/16-bit conversion first — warn on data loss.
- **Color management**: HDR radiance is scene-referred; conversion to output
  spaces must not clamp.
- **Layer edits in 32-bit**: only the supported adjustment/blend set applies.

## Parity acceptance criteria

1. Given three EV-bracketed exposures of the same scene, Merge To HDR Pro at
   32 Bit produces a float image whose radiance is monotonic across the bracket
   and matches a Debevec reference within the tolerance in
   `11-cross-cutting/testing-strategy.md`.
2. Given a saved response curve, reusing it on the same inputs reproduces the
   first merge within tolerance.
3. Given `Remove Ghosts` on with a known moving object, the object is taken from
   the base exposure (green-outlined) and ghosting is absent.
4. Given a 32-bit image, every command in the availability matrix is enabled and
   every excluded command is disabled.
5. Given a 32→16-bit conversion with Exposure and Gamma, output luminance equals
   `clip(exposure * 2^EV)^(1/gamma)` within tolerance; Highlight Compression
   maps values above the LDR white to white without hard clipping of midtones.
6. Given `View > 32-Bit Preview Options`, changing Method/Exposure/Gamma changes
   only the preview; the stored `f32` pixels are byte-identical before and after.
7. Given the status-bar 32-Bit Exposure slider, the adjustment is per-view and
   is not written to the file (reopening restores the stored preview).
8. Given a 32-bit PSD saved and reopened, the preview white point is restored.
9. Given CS6, Invert and Threshold adjustments work on masks in 32-bit/channel
   documents (the CS6 change).
10. Given a 32-bit image saved as TIFF, values round-trip without clamping
    within float tolerance.

## Sources

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  official CS6 Help PDF, "High dynamic range images": "About high dynamic range
  images", "Take photos for HDR images", "Features that support 32-bpc HDR
  images" (full availability matrix), "Merge images to HDR" (steps, ghosting,
  response curves, presets), "Options for 32-bit images" / "Options for 16- or
  8-bit images" (Local Adaptation, Equalize Histogram, Exposure and Gamma,
  Highlight Compression, Toning Curve/Corner), "Convert from 32 bits to 16 or
  8 bpc", "Adjust displayed dynamic range for 32-bit HDR images" (preview
  options, status-bar exposure, 32-bit Info readout), "About the HDR Color
  Picker", "Paint on HDR images"; CS6 "What's new": "Masks — Enable Invert and
  Threshold adjustments for masks in 32-bit/channel images", "Presets — New HDR
  Pro presets"; "Blending modes" note listing 32-bit blend modes.
- `https://web.archive.org/web/2023id_/https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/`
  — PSD spec: depth values `1, 8, 16, 32`; resource **1070** `HDR Toning
  information` (Photoshop CS2); color-mode enum.
- `https://docs.opencv.org/4.13.0/d2/df0/tutorial_py_hdr.html` — Debevec and
  Robertson merge, camera-response estimation, and tone mapping (TonemapDrago,
  TonemapMantiuk, TonemapReinhard); used as the standard-algorithm reference.
  Secondary.
- `https://pauldebevec.com/Research/HDR/debevec-siggraph97.pdf` — Debevec &
  Malik, *Recovering High Dynamic Range Radiance Maps from Photographs* (1997);
  the canonical merge/response-curve algorithm. Referenced, not fetched.
- `https://en.wikipedia.org/wiki/Tone_mapping` — tone-mapping overview and
  operator families. Secondary.

## Open questions

- **Adobe's CRF/merge internals**: weighting function, smoothing, and whether the
  implementation matches Debevec or Robertson. *Resolves with:* comparing CS6
  output to reference implementations.
- **Local Adaptation math**: the exact edge-aware local operator and the Toning
  Curve's default limit/equalise behaviour. *Resolves with:* analysis of
  or an Adobe paper.
- **Preview storage mechanism**: how PSD/PSB/TIFF store the 32-bit preview white
  point (resource 1070 or otherwise). *Resolves with:* inspecting a CS6 32-bit
  file.
- **Default tone method and slider ranges/defaults** in the Merge To HDR Pro
  16/8 dialog. *Resolves with:* CS6 captures.
- **Bit-depth default** in a fresh Merge To HDR Pro dialog. *Resolves with:* CS6
  screenshot.
- **Automatic alignment algorithm** and its failure modes. *Resolves with:*
  controlled hand-held bracket tests.
- **Ghost-removal base-image selection** heuristic ("best tonal balance").
  *Resolves with:* CS6 experiments.
- **Which later-version additions are not CS6** (e.g. 32-bit color pickers for
  3D materials, which the CS6 Help labels Creative Cloud only). *Resolves with:*
  CS6 vs CS6-CC side-by-side.
