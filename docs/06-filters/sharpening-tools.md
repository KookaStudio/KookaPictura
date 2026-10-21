# Sharpening Tools (Smart Sharpen Advanced and Shake Reduction)

- **Spec ID:** `FILT-093`
- **Status:** `Draft`
- **Parity tier:** `Core` for **Smart Sharpen Advanced** (CS6 Standard); **`Non-goal (Linux)` / CC-only** for **Shake Reduction** (not a CS6 feature).
- **New in CS6:** `No` — Smart Sharpen and its Basic/Advanced split predate CS6 and are unchanged; **Shake Reduction was introduced in Photoshop CC (2013), after CS6**, and is **not** part of the CS6 parity baseline. This spec is the **deep-dive companion** to `FILT-020` sharpen-filters; shared basics are not duplicated here.
- **Depends on:** `FILT-020` sharpen-filters (canonical Sharpen/Unsharp Mask/Smart Sharpen spec), `FILT-010` blur-filters (Gaussian kernel for `Remove = Gaussian Blur`), `FILT-001` filters-overview, `ARCH-006` gpu-rendering-pipeline, `ARCH-009` undo-history, `LAY-010` blend-modes (Luminosity), `IMG-009` 32-bit-hdr, `LAY-004` layer-masks.

> All module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Adobe's sharpening and deconvolution kernels are
> closed; parity is **behavioral parity only, algorithm TBD**. Shake Reduction
> content is **CC-only** and cannot be CS6 parity.

## CS6 behavior

### Smart Sharpen — Advanced mode (CS6)

`Filter > Sharpen > Smart Sharpen`. The dialog opens in **Basic** mode with **Amount**, **Radius**, **Remove**, **Angle** (only for `Remove = Motion Blur`), and **More Accurate**. Clicking **Advanced** reveals two further tabs, **Shadow** and **Highlight**, for damping the sharpening halos that appear in dark and light areas.

The CS6 Help states that the Shadow/Highlight controls are **available only for 8- and 16-bits-per-channel images** (not 32-bpc):

- **Fade Amount** — amount of sharpening applied in the shadows/highlights.
- **Tonal Width** — the range of tones modified. Smaller values restrict shadow correction to the darkest regions and highlight correction to the lightest regions.
- **Radius** — the size of the neighbourhood used to decide whether a pixel belongs to the shadows or the highlights; smaller = smaller area.

`Remove` selects the blur model to invert: **Gaussian Blur** (the Unsharp Mask method), **Lens Blur** (edge/detail-aware, finer detail with reduced halos), **Motion Blur** (reduces camera/subject-motion blur; uses **Angle**). **More Accurate** processes more slowly for a more accurate removal of blurring. Smart Sharpen and Unsharp Mask apply to **one layer at a time** (merge/flatten to sharpen all layers); a **selection** or an edge mask restricts them, and a Luminosity blend or `Edit > Fade Smart Sharpen` avoids colour shifts.

### Shake Reduction (CC-only — NOT CS6)

**Shake Reduction is post-CS6.** It shipped in **Photoshop CC (version 14.0, June 2013)**; it does not exist in a CS6 install, and the fetched CS6 Help PDF contains no Shake Reduction section. It is documented here because the brief requested it, but it **must not** appear in a CS6 parity baseline. It lives at `Filter > Sharpen > Shake Reduction`.

Documented CC behaviour (from the fetched Adobe Help page): Shake Reduction reduces blur from several kinds of camera motion — linear, arc-shaped, rotational, and zigzag. It:

- Automatically analyzes the region best suited to shake reduction, estimates the blur, and extrapolates corrections to the whole image.
- Shows a **Detail loupe** for close inspection.
- Supports **multiple blur traces** — a *blur trace* describes the shape and extent of the blur affecting a selected region. Traces are created by the **Add Suggested Blur Trace** icon, the **Blur Estimation Tool** (draw a rectangle), or the **Blur Direction Tool**, and can be adjusted via **Blur Trace Length** and **Blur Trace Direction** and the Detail loupe.
- Supports side-by-side preview of two traces (`Ctrl`/`Cmd`-select), duplicate traces, and **Save/Load Blur Trace** in **KNL** and **PNG** formats.
- **Advanced blur trace settings:** **Blur Trace Bounds**; **Source Noise** (`Auto`/`Low`/`Medium`/`High`, auto-estimated); **Smoothing** (reduces high-frequency sharpening noise; default **30%**, low recommended); **Artifact Suppression** (checkbox + slider; 100% yields the original image, 0% suppresses nothing; best for medium-frequency noise).
- Works best on decently lit, low-noise still camera images and can help sharpen motion-blurred text.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Filter > Sharpen > Smart Sharpen` | dialog | — | Basic by default; Advanced adds Shadow/Highlight |
| Smart Sharpen — Basic controls | controls | — | Amount, Radius, Remove, Angle, More Accurate |
| Smart Sharpen — Advanced button | toggle | — | Shows Shadow + Highlight tabs (8/16-bpc only) |
| Smart Sharpen — Shadow tab | tab | — | Fade Amount, Tonal Width, Radius |
| Smart Sharpen — Highlight tab | tab | — | Fade Amount, Tonal Width, Radius |
| `Edit > Fade Smart Sharpen` | menu, dialog | `Shift+Ctrl/Cmd+F` *(inferred)* | Opacity + mode; Luminosity recommended |
| Smart Sharpen — Preview | checkbox / canvas | — | Click-and-hold in preview compares |
| `Filter > Sharpen > Shake Reduction` | dialog | — | **CC only — absent in CS6** |
| Shake Reduction — Advanced panel | panel | — | Blur trace list, add/delete/duplicate |
| Shake Reduction — Blur Estimation Tool | tool | — | Draw a rectangle to create a trace |
| Shake Reduction — Blur Direction Tool | tool | — | Draw direction; adjust length/direction |
| Shake Reduction — Detail loupe | pane | — | Enhance-At-Loupe-Location icon |
| Shake Reduction — Advanced panel flyout | menu | — | Save/Load Blur Trace (KNL/PNG) |

## Parameters & ranges

Shared Basic Smart Sharpen parameters are tabulated in `FILT-020`; the Advanced-only parameters are below. Defaults marked *(inferred)* or *(unverified)* are not stated by the fetched CS6 source.

| Filter | Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|---|
| Smart Sharpen → Shadow/Highlight | Fade Amount | int % | *(inferred)* 0 | 0–100 *(inferred)* | Damping strength; 8/16-bpc only (sourced restriction) |
| Smart Sharpen → Shadow/Highlight | Tonal Width | int | *(inferred)* 50 | 0–100 *(inferred)* | Lower = restrict to darker (shadow) / lighter (highlight) tones |
| Smart Sharpen → Shadow/Highlight | Radius | int px | *(inferred)* 1 | 1–100 *(inferred)* | Neighbourhood deciding shadow/highlight membership |
| Smart Sharpen | Amount | int % | *(unverified)* | 1–500 *(modern; CS6 unverified)* | See `FILT-020` |
| Smart Sharpen | Radius | float px | *(unverified)* | 0.1–64 *(modern; CS6 unverified)* | See `FILT-020` |
| Smart Sharpen | Remove | enum | *(inferred)* Gaussian Blur | Gaussian Blur / Lens Blur / Motion Blur | `Angle` only for Motion Blur |
| Smart Sharpen | Angle | int ° | *(inferred)* 0 | *(unverified)* | Motion-Blur direction |
| Smart Sharpen | More Accurate | bool | off | on / off | Slower, more accurate blur removal |
| Shake Reduction | Blur Trace Bounds | int | *(unverified)* | *(unverified)* | Bound size of the trace |
| Shake Reduction | Source Noise | enum | Auto | Auto / Low / Medium / High | Auto-estimated |
| Shake Reduction | Smoothing | int % | **30** (sourced) | 0–100 *(inferred)* | Reduces high-frequency sharpening noise |
| Shake Reduction | Artifact Suppression | bool + int % | off? *(unverified)* | 0–100 | 100% = original; 0% = no suppression |
| Shake Reduction | Blur Trace Length / Direction | float / ° | *(unverified)* | *(unverified)* | Blur Direction Tool |
| Shake Reduction | Trace save format | enum | — | KNL / PNG | Advanced flyout |

## Algorithms & pipeline

Behavioral parity only; Adobe's deconvolution is closed.

**Smart Sharpen (Basic).** An unsharp-mask-style operation using the chosen `Remove` model:

```text
blur = deconvolve_model(src, radius, remove, angle)   # Gaussian / lens / motion
detail = src - blur
dst = src + amount * detail * more_accurate_gain
```

`Remove = Gaussian Blur` reduces to `src + amount*(src - gaussian(src,r))`, matching USM. `Lens Blur` uses an edge-aware/detail-preserving inverse model so halos are smaller. `Motion Blur` inverts a directional PSF at `Angle`. **More Accurate** performs a slower, closer approximation of the same inverse.

**Smart Sharpen (Advanced).** The Shadow/Highlight tabs compute a per-pixel tonal membership weight from a local **Radius** and **Tonal Width**, then scale the sharpening detail by `1 - fade_amount * weight` in shadows/highlights, suppressing halos without clipping the midtone sharpening. Evaluate in a luminance-preserving space and keep 32-bit float unclamped (the tabs themselves are disabled at 32-bpc).

**Shake Reduction (CC-only).** A **blind-deconvolution** estimator: derive a point-spread function (the "blur trace") from edge ghosting in selected regions, then deconvolve. `Source Noise` conditions a noise model; `Smoothing` suppresses high-frequency deconvolution ringing; `Artifact Suppression` trades sharpness for noise robustness (100% returns the original). Multiple traces combine several PSF estimates. This is the publicly documented family *(inferred)*; Adobe's exact estimator is unpublished. It is **not** CS6 parity.

## Rust module mapping

Proposals. Reuses the shared sharpen/deconvolution module in `FILT-020`; adds the advanced gating and (as an optional extension) the blind-deconvolution path.

- `pictura_filters::sharpen` — `SharpenParams { amount, radius, remove, angle, more_accurate, shadow: TonalGate, highlight: TonalGate }`, `struct TonalGate { fade_amount, tonal_width, radius }`. `fn apply_advanced(...)` applies the tonal gates after the base inverse. See `FILT-020` for the Basic path.
- `pictura_filters::sharpen::inverse` — `enum RemoveModel { Gaussian, Lens, Motion }` and `fn deconvolve(src, radius, model, angle) -> Buffer`.
- `pictura_render::sharpen` — optional GPU path (Smart Sharpen is on the OpenCL-accelerated filter family in Photoshop); CPU `rayon` reference.
- `pictura_filters::shake_reduction` *(non-parity extension)* — `ShakeReductionSession`, `BlurTrace { bounds, length, direction, weights }`, `TraceStore` (KNL/PNG), `fn estimate_psf(region) -> Psf`, `fn deconvolve_blind(src, psf, source_noise, smoothing, artifact_suppression)`.

Crossing types: `SharpenParams`, `TonalGate`, `Psf`, `BlurTrace`, `FilterResult`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `SmartSharpenDialog` | `QDialog` | Basic controls; `Advanced` button reveals `QTabWidget` with Shadow/Highlight pages; disable the tabs on 32-bpc |
| `SmartSharpenController` | `QObject` | Ties controls to the shared sharpen backend and the `Fade` record |
| `TonalGateWidget` | `QWidget` | Fade Amount / Tonal Width / Radius for one tab (shared by Shadow and Highlight) |
| `ShakeReductionDialog` *(extension)* | `QDialog` | Trace list, loupe, blur-estimation/direction tools, advanced sliders, save/load — **clearly labelled non-CS6** |

## Data-model impact

- **Destructive** by default (one history state per commit). On a **Smart Object** the same parameters can be a **Smart Filter** (`LAY-021`): store `SharpenParams` (including both `TonalGate`s) in the filter record's `params_blob`.
- **Luminosity handling:** record the optional Luminosity `Fade` state for the last commit (`Edit > Fade Smart Sharpen`).
- **Shake Reduction (extension)** would need the estimated PSF/trace set stored for redo and an optional `.knl`/`.png` trace asset; excluded from the CS6 parity model.
- **No new document nodes.**

## Edge cases

- **32-bpc** — Smart Sharpen runs, but the **Advanced Shadow/Highlight tabs must be disabled** (sourced). USM/Smart Sharpen must not clamp HDR values at 1.0.
- **8/16-bpc** — Advanced tabs available.
- **CMYK/Lab** — sharpen in a luminance-preserving space; avoid per-channel colour fringing; Layer > Luminosity is the documented workaround.
- **Bitmap/Indexed** — filters unavailable.
- **No visible layer / empty layer** — no-op; do not create history.
- **Amount 0** — no-op (no history state).
- **Selection / edge mask** — confine processing; use the shared filter apron.
- **Huge (PSB) documents** — tile the deconvolution; bound memory.
- **Shake Reduction** — CC-only; on a non-CS6 build, label it; on very noisy images the estimation is unreliable (feature itself recommends low-noise input).

## Parity acceptance criteria

1. Given Smart Sharpen `Remove = Gaussian Blur`, output matches Unsharp Mask at equal Amount/Radius within tolerance.
2. Given `Remove = Lens Blur`, halo magnitude at a high-contrast edge is measurably lower than `Gaussian Blur` at equal Amount/Radius.
3. Given `Remove = Motion Blur` at Angle θ, edges perpendicular to θ are emphasized relative to parallel edges.
4. Given **Advanced** mode on an 8- or 16-bpc image, raising **Fade Amount** in the Highlight tab monotonically reduces highlight sharpening/halo while midtone sharpening is unchanged; **Tonal Width** changes the affected tonal band.
5. Given a **32-bpc** document, the Shadow/Highlight tabs are disabled and Basic sharpening does not clamp values above 1.0.
6. Given a commit, exactly one history state appears and `Ctrl+Z` restores pre-filter pixels bit-exactly.
7. **CS6-boundary:** `Filter > Sharpen > Shake Reduction` is **absent** in CS6-parity mode; a CC-extension build labels it non-parity and its parameter set is not asserted as CS6 behaviour.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — official CS6 Help reference (downloaded to `/tmp`, `pdftotext`-extracted). Established: Smart Sharpen controls (Amount, Radius, Remove = Gaussian Blur/Lens Blur/Motion Blur, Angle, More Accurate); **Advanced** reveals Shadow/Highlight tabs with Fade Amount, Tonal Width, Radius, **available only for 8-/16-bpc**; one-layer-at-a-time and Luminosity-fade guidance; the 32-bpc list includes Smart Sharpen. **No Shake Reduction section exists** in the CS6 PDF.
- `https://web.archive.org/web/20130104015614id_/http://helpx.adobe.com/photoshop/using/adjusting-image-sharpness-blur.html` — CS6-era Adobe Help snapshot; corroborates the Smart Sharpen text in the PDF.
- `https://web.archive.org/web/20150905162441id_/https://helpx.adobe.com/photoshop/using/reduce-camera-shake-induced-blurring.html` — Adobe Help snapshot for **Shake Reduction** (CC-era): motion types (linear/arc/rotational/zigzag); blur traces and creation/modification tools; multiple traces, side-by-side preview, duplicate; Save/Load Blur Trace in KNL/PNG; Blur Trace Bounds; Source Noise Auto/Low/Medium/High; Smoothing default **30%**; Artifact Suppression 0–100% (100% = original, best for medium-frequency noise); suitability (low-noise, decently lit stills).
- `https://www.greaterthangatsby.com/history-of-photoshop` and `https://www.extremetech.com/computing/160816-photoshop-cc-hands-on-with-camera-shake-reduction-and-how-it-works` — community/tech sources placing **Shake Reduction at CC (v14.0, June 2013)**, after CS6. Secondary.
- `https://community.adobe.com/...` search result — a user confirms CS6's `Filter > Sharpen` does **not** contain Shake Reduction. Secondary.
- SearXNG meta-search (queries: "Photoshop Shake Reduction filter introduced CS6 or CC version", "Photoshop CS6 Smart Sharpen default Amount 100 Radius 1 Remove Gaussian Blur advanced") — used to locate the above; no facts asserted from snippets alone.

Not parsed in this pass: `helpx.adobe.com` live pages (HTTP 403).

## Open questions

- **Smart Sharpen Advanced defaults.** Fade Amount 0, Tonal Width 50, Radius 1 are *(inferred)* from common usage, not the fetched CS6 source. Resolves with: a CS6 dialog capture.
- **Basic Amount/Radius ranges in CS6.** Modern ranges (1–500%, 0.1–64 px) are used in `FILT-020` but are not confirmed for CS6. Resolves with: a CS6 capture.
- **`More Accurate` implementation.** Whether it changes sampling quality, the inverse operator, or both is undocumented. Resolves with: reference-image fitting.
- **`Lens Blur` remove model.** The exact edge-aware inverse is closed. Resolves with: a tolerance-based acceptance experiment.
- **Shake Reduction parity decision.** Confirmed **CC 2013, not CS6**. Decide whether to implement it as a labelled extension; if so, KNL/PNG trace interoperability and PSF estimation accuracy are open. Resolves with: a product decision + an Adobe trace-format sample.
- **Deduplication with `FILT-020`.** This file and `FILT-020` sharpen-filters both cover Smart Sharpen; `FILT-020` is canonical for the Basic path and parameters. Resolves with: a docs consolidation pass to keep one source of truth.
