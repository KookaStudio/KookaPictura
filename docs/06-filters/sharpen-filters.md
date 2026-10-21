# Sharpen Filters

- **Spec ID:** `FILT-020`
- **Status:** `Draft`
- **Parity tier:** `Core` — Sharpen, Sharpen Edges, Sharpen More, Unsharp Mask, and Smart Sharpen are CS6 Standard.
- **New in CS6:** `No` — the Sharpen submenu is unchanged from CS5. **Shake Reduction is *not* a CS6 feature**: it was introduced in **Photoshop CC (2013)** and is documented here as an explicit **non-parity / CC-only** entry because the brief requested it. See `## CS6 behavior`.
- **Depends on:** `FILT-001` filters-overview, `FILT-010` blur-filters (Smart Sharpen's `Remove = Gaussian Blur` and USM share the Gaussian kernel), `LAY-010` blend-modes (Luminosity fade), `ARCH-006` gpu-rendering-pipeline, `ARCH-009` undo-history, `IMG-009` 32-bit-hdr, `LAY-004` layer-masks.

> All module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Adobe's exact sharpening kernels are closed; parity is
> **behavioral parity only, algorithm TBD**. The CS6 Help describes the controls
> and qualitative results; algorithm families are drawn from the standard
> references cited under `## Sources`. Any **Shake Reduction** content is
> **CC-only** and cannot be CS6-parity.

## CS6 behavior

The Sharpen filters focus blurred images by raising the contrast of adjacent pixels. The Help's guidance: reduce noise before sharpening; sharpen in small passes; judge on the output medium; use Unsharp Mask or Smart Sharpen for control, because the other three are **automatic with no options**.

- **Sharpen / Sharpen More** — focus a selection by increasing edge contrast; **Sharpen More** is the stronger of the two. Both are immediate commands with **no dialog**.
- **Sharpen Edges** — finds where significant color changes occur and sharpens **only the edges**, preserving overall smoothness; no options.
- **Unsharp Mask (USM)** — locates pixels that differ in value from their surroundings by more than a **Threshold**, then increases their contrast by an **Amount**, over a **Radius**. It is *not* an edge detector: per the Help, Unsharp Mask instead locates pixels whose values differ from those around them by more than the threshold. Oversharpening creates **halos**. On-screen effect is more pronounced than in high-resolution print. Advice: radius 1–2 for high-res; amount 150–200% for high-res print; threshold 2–20 to avoid noise/posterization; default threshold 0 sharpens every pixel.
- **Smart Sharpen** — the recommended general sharpener; adds controls USM lacks. It has **Basic** and **Advanced** modes:
  - **Basic:** Amount, Radius, Remove (Gaussian Blur / Lens Blur / Motion Blur), Angle (for Motion Blur), More Accurate.
  - **Advanced:** the Basic controls plus **Shadow** and **Highlight** tabs to damp halos in dark/light areas: **Fade Amount**, **Tonal Width**, **Radius**. The Help states these tabs are **available only for 8- and 16-bit-per-channel images** (not 32-bit).
  - **Remove** choices: **Gaussian Blur** = the USM method; **Lens Blur** = detects edges/detail and gives finer detail with reduced halos; **Motion Blur** = attempts to reduce camera/subject-motion blur, using **Angle**.
  - **More Accurate** processes more slowly for a more accurate removal of blurring.
- **Shake Reduction (CC-only)** — automatically reduces blur caused by camera shake. It was introduced in Photoshop CC (2013), after CS6, and does not exist in a CS6 install. Kooka Pictura targets CS6 parity, so Shake Reduction is **out of parity scope**; if implemented at all it would be a CC-behavior extension (blind deconvolution). It is **not asserted as CS6 behavior**.

**Applying to one layer at a time:** USM and Smart Sharpen apply to a single layer even when layers are linked or grouped; merging/flattening is required to sharpen all image layers. Both can be applied to a **selection** or restricted by an **edge mask**. To avoid color shifts on a separate sharpening layer, the Help recommends setting the layer's blending mode to **Luminosity**, and using `Edit > Fade Unsharp Mask` with mode **Luminosity** when bright colors become oversaturated.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Filter > Sharpen > Sharpen` | menu (immediate) | — | No dialog |
| `Filter > Sharpen > Sharpen Edges` | menu (immediate) | — | No dialog |
| `Filter > Sharpen > Sharpen More` | menu (immediate) | — | No dialog; stronger than Sharpen |
| `Filter > Sharpen > Unsharp Mask` | dialog | — | Amount, Radius, Threshold; preview |
| `Filter > Sharpen > Smart Sharpen` | dialog | — | Basic/Advanced; Sharpen/Shadow/Highlight tabs |
| `Filter > Sharpen > Shake Reduction` | dialog | — | **CC only — absent in CS6** |
| `Edit > Fade Unsharp Mask` / `Edit > Fade Smart Sharpen` | dialog | `Shift+Ctrl/Cmd+F` *(inferred)* | Opacity + mode; Luminosity recommended |
| Sharpen dialog preview | widget | +/- | Drag to recenter; click-hold to compare |
| Layers panel blending mode | combo | — | Set sharpening layer to Luminosity |

## Parameters & ranges

| Filter | Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|---|
| Sharpen / Sharpen More / Sharpen Edges | — | — | — | — | No controls |
| Unsharp Mask | Amount | int % | *(unverified; likely 100)* | 1–500 *(modern PS; CS6 unverified)* | Contrast added at edges |
| Unsharp Mask | Radius | float px | 1.0 *(inferred)* | 0.1–250 *(modern PS; CS6 unverified)* | Width of edge effect; 1–2 for high-res (sourced) |
| Unsharp Mask | Threshold | int levels | 0 (sourced) | 0–255 (sourced scale) | Pixels differing by less are untouched; 2–20 recommended |
| Unsharp Mask | Preview | bool | on | on/off | |
| Smart Sharpen | Mode | enum | Basic | Basic / Advanced | Advanced exposes Shadow/Highlight |
| Smart Sharpen | Amount | int % | *(unverified)* | 1–500 *(modern; CS6 unverified)* | |
| Smart Sharpen | Radius | float px | *(unverified)* | 0.1–64 *(modern; CS6 unverified)* | |
| Smart Sharpen | Remove | enum | Gaussian Blur *(inferred)* | Gaussian Blur / Lens Blur / Motion Blur | Sourced options |
| Smart Sharpen | Angle | int ° | 0 *(inferred)* | –360 … +360 *(inferred)* | Only for Remove = Motion Blur |
| Smart Sharpen | More Accurate | bool | off | on/off | Slower, more accurate blur removal |
| Smart Sharpen → Shadow/Highlight | Fade Amount | int % | 0 *(inferred)* | 0–100 *(inferred)* | 8/16-bit only (sourced) |
| Smart Sharpen → Shadow/Highlight | Tonal Width | int | 50 *(inferred)* | 0–100 *(inferred)* | Range of tones affected |
| Smart Sharpen → Shadow/Highlight | Radius | int px | 1 *(inferred)* | 1–100 *(inferred)* | Area deciding shadow/highlight membership |
| Shake Reduction | — | — | — | CC-only | Not in CS6; no parity baseline |

`(inferred)` values are from modern Photoshop dialogs/community documentation, not the fetched CS6 PDF; see `## Open questions`. Values marked *(sourced)* are stated by the CS6 Help PDF.

## Algorithms & pipeline

Adobe kernels are closed; the following are the standard families that reproduce the documented behavior. **Behavioral parity only.**

### Sharpen / Sharpen More — fixed Unsharp Mask
- Unsharp Mask at fixed settings: σ 1 (radius 3 under `FILT-010`), threshold 0, amount **50 %** for Sharpen and **100 %** for **Sharpen More**, per channel. The canonical 3×3 kernel `[0 −1 0; −1 5 −1; 0 −1 0]` (Wikipedia *Unsharp masking* §Implementation) has so much gain at the finest detail that a few repeated passes destroy a photograph, while CS6's Sharpen can be reapplied, each pass adding a little. The amounts are photorust's CS6-tuned approximation (#222); Adobe's kernel is closed.
- Reference: Wikipedia *Unsharp masking* (formula and the 3×3 kernel derivation).

### Sharpen Edges — edge-gated sharpening
- Measure the edge strength as the Sobel gradient magnitude of the brightness of a σ 1 blur (the blur keeps noise from reading as an edge), then add `original − blurred` weighted by a smoothstep of that strength between fixed levels (6 and 36), leaving flat areas alone. The fade avoids the outline a hard gate draws. No controls → fixed internal levels (photorust's model, #222).

### Unsharp Mask — blur-difference sharpening
- Compute a **blurred** copy (Gaussian, `FILT-010`), then `sharpened = original + (original − blurred) × amount`, with a **threshold** gate: only where `|original − blurred|` (or the local difference) exceeds the threshold. This is the canonical digital unsharp-mask formula. Amount controls the overshoot magnitude; radius controls the width of the edge rims/halos; threshold prevents sharpening smooth, low-contrast areas (which would amplify noise).
- Implement via the separable Gaussian kernel; the "difference" can be applied in a single combined convolution (Dirac − Gaussian) for speed.
- Reference: Wikipedia *Unsharp masking* (formula, amount/radius/threshold semantics, halo cause).

### Smart Sharpen — deconvolution of a sharp-core + halo blur
- The blur model is `H = 0.62·core + 0.38·halo`. `core` is a pixel-scale separable 3-tap `[0.17, 0.66, 0.17]`. `halo` follows **Remove** at `0.8·radius`:
  - **Gaussian Blur:** a Gaussian of that σ.
  - **Lens Blur:** a disc of that radius (a defocus PSF), so its inverse peaks near the disc's first Airy minimum instead of drawing an even-width outline.
  - **Motion Blur:** a line of twice that length along **Angle** (a linear motion PSF).
- `core⁻¹` is exact (a causal and an anticausal one-pole pass). The halo factor is solved by relaxed Van Cittert iteration from the original. The detail `deconvolved − original` is scaled by **Amount**, rolled off below a **Reduce Noise** floor, and damped by the tonal fades. Inverting `H` lifts the finest detail hard and the radius-scale structure moderately. The Help says Gaussian Blur is "the method used by Unsharp Mask", but CC's Smart Sharpen does not reduce to an unsharp mask.
- **More Accurate** runs the halo solve to convergence (8 iterations instead of 3).
- *(approximation)* The constants were fitted to two Photoshop screenshots of one JPEG (Gaussian r 30.3 / 378 %, Lens r 9.9 / 405 %). The measured response has a moderate plateau above `1/radius` and a rise toward Nyquist that amplifies the JPEG's 8×8 blocks. That is the signature of this inverse, not of an unsharp mask. It is behavioral parity from those screenshots only; Adobe's algorithm is closed. See OpenSpec `smart-sharpen-deconvolution`.
- **Advanced Shadow/Highlight fade** is a **tonal-range gate**, closely related to layer "Blend If": for each pixel, Fade Amount scales the sharpening contribution when the pixel lies in the shadow (or highlight) tonal band; Tonal Width sets the band width; Radius sets the neighborhood used to decide the pixel's tonality. This is how the Help describes reducing halos in dark/light areas. 8-/16-bit only in CS6.
- Reference: Wikipedia *Unsharp masking* §Comparison with deconvolution (USM is a linear convolution; deconvolution is a nonlinear inverse problem using a PSF model).

### Shake Reduction (CC-only) — blind deconvolution
- Estimate a **point-spread function** from the camera-shake trajectory and deconvolve the image. This is a blind/parametric deconvolution problem and is **not in CS6**. If built, it is a CC-behavior extension, not parity; document separately.
- Reference: search results identify the feature as introduced in CC (2013), e.g. ExtremeTech's "Photoshop CC: Hands on with camera Shake Reduction".

### Shared pipeline
Apply through `FILT-001`: depth/mode gate → **apron** of `radius` → backend dispatch (GPU compute where ported, else `rayon` CPU) → tile iteration with progress/cancel → selection/mask composite → Luminosity handling → single undo commit. For **Smart Sharpen Shadow/Highlight** and any tonal gate, evaluate in a luminance-preserving space and keep 32-bit float unclamped.

## Rust module mapping

Proposed under `pictura-filters::sharpen` (`ARCH-002`, `ARCH-006`):

- `pictura-filters::sharpen` — `Sharpen`, `SharpenMore`; Unsharp Mask at the fixed `SHARPEN_AMOUNT` / `SHARPEN_MORE_AMOUNT` and `SHARPEN_RADIUS`.
- `pictura-filters::photorust::sharpen` — `SharpenEdges`; smoothstep-gated unsharp on a Sobel edge strength.
- `pictura-filters::sharpen::usm` — `UnsharpMask { amount: f32, radius: f32, threshold: u8 }`; blurred-difference with threshold; reuses `blur::gaussian`.
- `pictura-filters::sharpen::smart` — `SmartSharpen { amount, radius, remove: Remove, angle, more_accurate, shadow: TonalFade, highlight: TonalFade }`; `Remove = Gaussian|Lens|Motion`, each a strategy implementation of a `SharpenModel` trait; `TonalFade { fade_amount, tonal_width, radius }`.
- `pictura-filters::sharpen::shake_reduction` — **CC-only, not parity**; `ShakeReduction { ... }` behind a feature flag, blind-deconvolution research placeholder. Not part of the CS6 registry default.
- Shared helpers: `pictura-filters::luma` (luminance-preserving application), `threshold_gate`.

Types: `Amount(f32)` (%), `Radius(f32)`, `Threshold(u8)`, `Remove`, `TonalFade`. Crosses `TileView` ROIs; no Qt types.

## Qt6 component mapping

Widgets, matching the CS6 modal dialogs.

- `UnsharpMaskDialog` — `QDoubleSpinBox` amount, radius; `QSpinBox` threshold; preview pane.
- `SmartSharpenDialog` — `QTabWidget` (Sharpen, Shadow, Highlight); Basic/Advanced toggle; `Remove` `QComboBox`, `Angle` dial, `More Accurate` checkbox; Shadow/Highlight controls disabled on 32-bit documents with a reason.
- `SharpenPreview` (`QLabel`/`QImage`) — shared preview with +/- zoom and click-hold compare.
- `FilterProgressProxy` — shared cancelable progress.
- `ShakeReductionDialog` — present only in a CC-extension build; must not appear in CS6-parity mode.

## Data-model impact

- **No persistent fields** for destructive sharpen filters. One `HistoryRecord::FilterOp` per apply (`ARCH-009`).
- **USM / Smart Sharpen** store `{ amount, radius, threshold, remove, angle, more_accurate, shadow, highlight }` in the record's `params_blob`; all are re-editable from the Smart Filter entry if applied to a Smart Object (`LAY-021`).
- **Luminosity recommendations** are UI guidance, not stored state; `Edit > Fade` mode is a one-shot composite (`FILT-001`).
- **Shake Reduction** (CC extension) would need an estimated PSF/seed stored for redo; excluded from the parity model.

## Edge cases

- **Threshold above the local contrast.** USM with a high threshold leaves low-contrast areas untouched (by design); ensure the gate is on the same scale (0–255 for 8/16-bit; define for 32-bit float).
- **Amount 0 / Radius 0.** Identity; commit consistently or refuse.
- **Clipping/halos.** Oversharpening clips highlights/shadows; the design must not wrap or overflow; clamp only at the final 8/16-bit store, never in the 32-bit pipeline.
- **Color fringing.** Per-channel sharpening can shift hue; Luminosity mode/fade must be available and correct.
- **8/16 vs 32-bit.** Smart Sharpen Shadow/Highlight tabs must be **disabled** on 32-bit (sourced restriction).
- **CMYK / Lab / Grayscale / Multichannel.** Run per channel in the document space; Luminosity fade respects `FILT-001` Lab restrictions.
- **1-px / tiny images.** Edge kernels need border handling; clamp-to-edge, no panics.
- **Huge PSB.** Tile-local; deconvolution-style methods must bound their working set per tile with an apron.
- **GPU unavailable / device lost.** CPU path; identical results within tolerance.
- **Cancellation / undo.** Atomic; no partial writes; one history state.
- **Randomness.** None of these filters is random; redo is deterministic.

## Parity acceptance criteria

1. Given a blurred edge, `Sharpen`, `Sharpen Edges`, and `Sharpen More` all increase edge contrast, with `Sharpen More` visibly stronger than `Sharpen`, and `Sharpen Edges` leaving flat regions unchanged.
2. Given a uniform image, USM with any Amount/Radius leaves it unchanged within 1 LSB (no-op on zero-contrast areas).
3. Given a step edge, USM amount A and radius R produce an overshoot proportional to A and an edge-rim width proportional to R, within a stated tolerance of a reference blurred-difference implementation.
4. Given adjacent pixels differing by less than Threshold, USM does not change them; differing by Threshold or more, it does. Threshold 0 sharpens every pixel (sourced behavior).
5. Given Smart Sharpen at the same Amount/Radius as USM, the finest detail gains more than under USM (deconvolution, not a blur difference); `Remove = Lens Blur` produces a softer halo than the even-width outline of a disc unsharp mask; `Remove = Motion Blur` with Angle θ acts along the motion direction.
6. Given Advanced mode with a Shadow Fade Amount > 0, dark-region halos are reduced relative to the same settings without fade; Tonal Width changes which tones are damped; these controls are disabled on 32-bit documents.
7. Given a selection, only the selection is sharpened; given an edge mask, only masked areas are sharpened.
8. Given sharpening on a separate Luminosity layer (or `Fade ... Luminosity`), hue shifts along edges are reduced versus Normal mode.
9. Given a 32-bit HDR document, USM/Smart Sharpen outputs are not clamped at 1.0.
10. Given any apply, undo is bit-exact and adds exactly one history state; cancelling adds none.
11. Given CS6-parity mode, `Filter > Sharpen > Shake Reduction` is **absent**; if a CC-extension build adds it, it is clearly labelled non-parity.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — Adobe Photoshop CS6 Help reference (text-extracted). Established: Sharpen/Sharpen Edges/Sharpen More descriptions and no-options nature; Unsharp Mask tutorial (does not detect edges; amount/radius/threshold semantics; default threshold 0; threshold 4 on a 0–255 scale; recommend threshold 2–20; radius 1–2 for high-res; amount 150–200% for high-res print; halo from oversharpening; Luminosity fade advice); Smart Sharpen (Amount, Radius, Remove = Gaussian Blur/Lens Blur/Motion Blur, Angle, More Accurate; Advanced Shadow/Highlight tabs with Fade Amount/Tonal Width/Radius, 8-/16-bit only); `Filter > Sharpen` menu paths; 16-/32-bit availability (Smart Sharpen and Unsharp Mask are the only Sharpen filters in both 16- and 32-bit lists); "reduce noise before sharpening" guidance. Primary source.
- `https://en.wikipedia.org/wiki/Unsharp_masking` — digital USM formula `sharpened = original + (original − blurred) × amount`; amount/radius/threshold semantics; the `[0 −1 0; −1 5 −1; 0 −1 0]` sharpen kernel and its derivation; USM as a linear convolution vs. deconvolution as a nonlinear PSF-based inverse problem. Standard reference (fetched).
- `https://en.wikipedia.org/wiki/Gaussian_blur` — the separable Gaussian used by USM and Smart Sharpen `Remove = Gaussian Blur`. Standard reference (fetched).

Feature-version confirmation (search results, not fetched as documents): `https://www.extremetech.com/computing/160816-photoshop-cc-hands-on-with-camera-shake-reduction-and-how-it-works` and Adobe's Photoshop CC 2013 feature summary establish **Shake Reduction as introduced in Photoshop CC (2013)**, not CS6. Treated as *(secondary/unverified)* but consistent across sources; see `## Open questions`.

Repository sources (fetched locally): `docs/06-filters/filters-overview.md`, `docs/06-filters/blur-filters.md` (Gaussian kernel reuse), `docs/05-layers/smart-filters.md` (Smart Filter entry), `docs/01-architecture/rust-core-design.md`, `docs/01-architecture/gpu-rendering-pipeline.md`.

## Open questions

- **Shake Reduction provenance.** Multiple secondary sources place it in Photoshop CC (2013), and it is absent from the fetched CS6 Help PDF. Confirm there was no separate CS6 update channel that shipped it; if confirmed absent, keep it out of parity (current stance).
- **Exact CS6 ranges/defaults.** The fetched PDF states no numeric ranges for USM Amount/Radius, Smart Sharpen Amount/Radius, or the Shadow/Highlight controls. Ranges above are *(inferred)* from modern Photoshop and community sources. Resolve by scripted CS6 dialog reads.
- **Gaussian `Remove` == USM exactly?** Whether Smart Sharpen with `Remove = Gaussian Blur` is bit-identical to USM at the same settings, or merely equivalent in family, is unverified. Resolve by comparison.
- **Lens Blur / Motion Blur `Remove` algorithms.** Adobe does not document the PSF or deconvolution method. Behavioral parity is the requirement; a design spike must pick a concrete algorithm (e.g. Wiener/Richardson–Lucy) and match halos/detail qualitatively.
- **Shadow/Highlight tonal gate formula.** The relationship between Fade Amount, Tonal Width, and Radius and the actual per-pixel gate is closed. Resolve with tonal-step probes.
- **32-bit behavior of Smart Sharpen.** 32-bit is supported (list), but Shadow/Highlight tabs are disabled; confirm the exact 32-bit default and whether `More Accurate` is available.
- **Threshold scale in 32-bit.** How the 0–255 threshold maps to float HDR values is undocumented. Resolve by experiment.
- **Defaults.** Default Amount/Radius/Remove and the USM default Amount are unverified.
