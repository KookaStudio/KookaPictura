# Noise Filters

- **Spec ID:** `FILT-030`
- **Status:** `Draft`
- **Parity tier:** `Core` — the Noise submenu (Add Noise, Despeckle, Dust & Scratches, Median, Reduce Noise) is CS6 Standard.
- **New in CS6:** `No` — the Noise submenu is unchanged from CS5. `Reduce Noise` remains primarily a **CPU** function (independent GPU testing; `ARCH-006`).
- **Depends on:** `FILT-001` filters-overview, `FILT-010` blur-filters (edge-preserving/smoothing kernels), `ARCH-006` gpu-rendering-pipeline, `ARCH-009` undo-history, `ARCH-007` color-management, `IMG-009` 32-bit-hdr, `LAY-004` layer-masks.

> All module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Adobe's exact denoise kernels are closed; parity is
> **behavioral parity only, algorithm TBD**. The CS6 Help describes each filter's
> controls and qualitative behavior; algorithm families are drawn from the
> standard references cited under `## Sources`. Parameter ranges are *(inferred)*
> where the fetched primary source does not state them.

## CS6 behavior

The Noise filters "add or remove noise, or pixels with randomly distributed color levels," to blend a selection into its surroundings, create texture, or remove defects such as dust and scratches.

- **Add Noise** — applies random pixels to simulate high-speed film grain. It can also reduce banding in feathered/graduated fills and make retouched areas look realistic.
  - **Distribution:** **Uniform** distributes noise values as random numbers between 0 and ± the specified **Amount** (subtle); **Gaussian** distributes them along a bell curve (speckled). *(Both sourced.)*
  - **Monochromatic** applies noise to the **tonal elements only**, without changing colors (i.e. the same delta to every channel), versus the default per-channel application.
- **Despeckle** — detects edges (areas of significant color change) and blurs everything **except** the edges, removing noise while preserving detail. No dialog.
- **Dust & Scratches** — reduces noise by changing dissimilar pixels. The user balances **Radius** (search area for dissimilar pixels; larger blurs more) against **Threshold** (how dissimilar a pixel must be before it is replaced). Procedure: set Threshold to 0 first so all pixels are examined, then raise Radius to the smallest value that removes defects, then raise Threshold to the highest value that still removes them. The Threshold slider "gives greater control for values between 0 and 128."
- **Median** — reduces noise by "blending the brightness of pixels within a selection": searches the **Radius** around each pixel for pixels of similar brightness, discards pixels that differ too much, and replaces the center pixel with the **median** brightness of the searched pixels. Useful for reducing motion effects.
- **Reduce Noise** — reduces noise while preserving edges, based on settings affecting the whole image or individual channels. Controls:
  - **Strength** — amount of luminance noise reduction applied to all channels.
  - **Preserve Details** — preserves edges/detail such as hair or texture; **100** preserves the most detail but reduces luminance noise least.
  - **Reduce Color Noise** — removes random color (chroma) pixels; higher removes more.
  - **Sharpen Details** — sharpens after denoising (denoising reduces sharpness).
  - **Remove JPEG Artifacts** — removes blocky artifacts and halos from low-quality JPEGs.
  - **Advanced / per-channel:** if luminance noise is worse in one or two channels (often blue), click **Advanced** and choose a **Channel**, then set **Strength** and **Preserve Details** for that channel. The Help notes correcting one channel preserves more detail than a global correction.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Filter > Noise > Add Noise` | dialog | — | Amount, Distribution, Monochromatic |
| `Filter > Noise > Despeckle` | menu (immediate) | — | No dialog |
| `Filter > Noise > Dust & Scratches` | dialog | — | Radius, Threshold |
| `Filter > Noise > Median` | dialog | — | Radius |
| `Filter > Noise > Reduce Noise` | dialog | — | Strength, Preserve Details, Reduce Color Noise, Sharpen Details, Remove JPEG Artifacts; Advanced per-channel |
| Reduce Noise per-channel | combo | — | Channel menu shown only in Advanced |
| Filter dialog preview | widget | +/- | Drag to recenter; zoom to see noise |
| `Edit > Fade <Filter>` | dialog | `Shift+Ctrl/Cmd+F` *(inferred)* | Opacity + mode for the apply |

## Parameters & ranges

| Filter | Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|---|
| Add Noise | Amount | float % | 12.5 *(inferred)* | 0.1–400 *(modern; CS6 unverified)* | Magnitude of noise |
| Add Noise | Distribution | enum | Gaussian *(inferred)* | Uniform / Gaussian | Sourced options |
| Add Noise | Monochromatic | bool | off | on/off | Tonal-only vs per-channel (sourced) |
| Despeckle | — | — | — | — | No controls |
| Dust & Scratches | Radius | int px | 1 *(inferred)* | 1–16 (sourced) | Search area; larger blurs more |
| Dust & Scratches | Threshold | int levels | 0 *(inferred)* | 0–255 (sourced scale) | Finer control 0–128 (sourced) |
| Median | Radius | int px | 1 *(inferred)* | 1–100 *(inferred)* | Median window radius |
| Reduce Noise | Strength | int | 0 *(inferred)* | 0–10 *(community; CS6 unverified)* | Luminance denoise amount |
| Reduce Noise | Preserve Details | int % | 0 *(inferred)* | 0–100 *(inferred)* | 100 = most detail, least denoise |
| Reduce Noise | Reduce Color Noise | int % | 0 *(inferred)* | 0–100 *(inferred)* | Chroma strength |
| Reduce Noise | Sharpen Details | int % | 0 *(inferred)* | 0–100 *(inferred)* | Post-denoise sharpening |
| Reduce Noise | Remove JPEG Artifacts | bool | off | on/off | Blocky artifact/halo removal |
| Reduce Noise | Advanced / Channel | combo | Composite | per color channel | Per-channel Strength + Preserve Details |
| Reduce Noise | Preview | bool | on | on/off | |

`(inferred)` values are from modern Photoshop dialogs and community documentation, not the fetched CS6 PDF; see `## Open questions`.

## Algorithms & pipeline

Adobe kernels are closed; the following are standard families that reproduce the documented behavior. **Behavioral parity only.** Noise filters are the one family where **randomness** is intrinsic; all randomness must be **seeded and stored** so redo/undo are deterministic.

### Add Noise — pseudo-random perturbation
- For each channel and pixel, add a random delta drawn from:
  - **Uniform** on `[−amount, +amount]`; or
  - **Gaussian** `N(0, σ)` with `σ` proportional to amount.
- **Monochromatic:** draw one delta per pixel and apply it to all channels (preserves hue; "applies to tonal elements only"); otherwise draw a delta per channel (chromatic speckle).
- Amount is a percentage of full scale; map to the 8/16-bit range or float range per document depth.
- Reference: Wikipedia *Additive white Gaussian noise* (additive zero-mean Gaussian model).

### Despeckle — edge-gated smoothing
- Detect edges (gradient/color-difference), then apply a smoothing/median operation only to pixels not on an edge. This matches the Help's "blurs all of the selection except those edges" while preserving detail. Algorithm family: edge detection + selective median/Gaussian; the exact detector is closed.

### Dust & Scratches — threshold-gated median/average
- For each pixel, examine the `(2r+1)²` neighborhood (Radius). Compute the local reference (median or robust mean). Replace the center pixel **only if** it differs from the local reference by more than **Threshold** (i.e. it is an outlier: dust, a scratch, or isolated noise). Radius controls the window and thus the smoothing side-effect; Threshold controls specificity. This mirrors "reduces noise by changing dissimilar pixels" and the documented radius/threshold balancing procedure.
- Reference: the median-family rank filter — Wikipedia *Median filter*.

### Median — non-linear rank filter
- Replace each pixel with the **median** of the channel values in its `(2r+1)²` window (per channel, or on luminance for color). Non-linear and **not separable** in general. Preserves edges while removing impulse noise; effective against salt-and-pepper and motion speckle.
- Efficient implementations maintain a **histogram** of the sliding window and select the median directly (Huang et al. 1979, cited by the reference), giving near-constant time per pixel for integer depths.
- Reference: Wikipedia *Median filter* (rank filter, edge preservation, histogram fast median).

### Reduce Noise — edge-preserving luminance/chroma denoise
- The Help describes distinct luminance and color noise and independent per-channel correction. Standard families that fit: an **edge-preserving smoother** (bilateral / non-local-means / guided filter class) for luminance, and a **chroma-space blur** for color noise. A common production choice is **wavelet-based denoising** (thresholding coefficients per sub-band), which naturally separates luminance from chroma and supports per-channel strength/detail.
- Proposed mapping: **Strength** = denoise threshold/scale; **Preserve Details** = edge-preservation/coefficient-retention term; **Reduce Color Noise** = chroma thresholding strength (typically stronger than luminance); **Sharpen Details** = post-pass USM (`FILT-020`); **Remove JPEG Artifacts** = a block-aware/8×8-DCT deblocking pass or halo suppression.
- Because Adobe does not publish the algorithm, the exact model is TBD; behavioral parity is required (noise down, edges preserved, adjustable per channel).
- References: Wikipedia *Bilateral filter* and Wikipedia *Median filter*'s "denoise methods" context (edge-preserving smoothing; wavelet denoising).

### Shared pipeline
Apply through `FILT-001`: depth/mode gate → **apron** of `radius` for neighborhood filters → backend dispatch (Add Noise, Median, Dust & Scratches, Despeckle are GPU-friendly compute kernels; Reduce Noise is proposed CPU-first, matching CS6) → tile iteration with progress/cancel → selection/mask composite → single undo commit. **Capture and store the RNG seed** for any random filter.

## Rust module mapping

Proposed under `pictura-filters::noise` (`ARCH-002`, `ARCH-006`):

- `pictura-filters::noise::add` — `AddNoise { amount: f32, distribution: Uniform|Gaussian, monochromatic: bool, seed: u64 }`; deterministic RNG (`rand`/`rand_chacha` proposed) seeded per apply.
- `pictura-filters::noise::despeckle` — `Despeckle`; edge detector + selective smoothing.
- `pictura-filters::noise::dust_scratches` — `DustScratches { radius: u8, threshold: u8 }`; threshold-gated median/robust-mean.
- `pictura-filters::noise::median` — `Median { radius: u8 }`; sliding-histogram rank filter for integer depths, selection/sort for float.
- `pictura-filters::noise::reduce` — `ReduceNoise { strength, preserve_details, reduce_color_noise, sharpen_details, remove_jpeg_artifacts, per_channel: Option<ChannelSet> }`; luminance edge-preserving denoise + chroma denoise + optional deblock + post-USM.
- `pictura-filters::noise::rng` — seeded `ChaCha8Rng` shared by random filters; seed stored in the history record.
- Shared helpers: `pictura-filters::robust` (median/order statistics), `pictura-filters::luma`.

Types: `Amount(f32)`, `Radius(u8)`, `Threshold(u8)`, `Distribution`, `Seed(u64)`, `ChannelSet`. Crossing `TileView` ROIs; no Qt types.

## Qt6 component mapping

Widgets, matching the CS6 modal dialogs.

- `AddNoiseDialog` — amount `QDoubleSpinBox`, distribution `QComboBox`, monochromatic `QCheckBox`, preview.
- `DustScratchesDialog` / `MedianDialog` — radius (and threshold) spin boxes, preview at 100% recommended.
- `ReduceNoiseDialog` — Strength/Preserve Details/Reduce Color Noise/Sharpen Details sliders (0–10 and 0–100), `Remove JPEG Artifacts` checkbox, an **Advanced** disclosure that reveals a `Channel` `QComboBox` with per-channel Strength + Preserve Details, and a 100% preview.
- `NoiseFilterPreview` (`QLabel`/`QImage`) — shared preview with +/- zoom and drag-to-pan; noise requires 100% zoom for a faithful view.
- `FilterProgressProxy` — shared cancelable progress (Reduce Noise is the heaviest of this family).

## Data-model impact

- **No persistent fields** for destructive noise filters. One `HistoryRecord::FilterOp` per apply (`ARCH-009`).
- **Huge impact only via tile diffs:** Add Noise touches every pixel in the target; the undo record must retain prior tiles, so a full-layer Add Noise is a large history state (matching CS6's "full copy" cost for full-image operations).
- **Randomness:** the RNG **seed** is stored in the record so an identical apply is reproducible on redo; a new apply gets a new seed. This is a design choice (Adobe does not document its redo behavior).
- **Smart Object:** any noise filter applied to a Smart Object becomes a Smart Filter entry with `{ filter_id, params, blend, opacity, enabled }` (`LAY-021`); params include the seed so the non-destructive preview is stable.
- **Reduce Noise per-channel** stores the channel set and per-channel values in `params_blob`; no channel is created or destroyed.
- **JPEG artifacts** removal does not modify document metadata; it operates on pixels only.

## Edge cases

- **8/16/32-bit.** All of these are in the 32-bit list for Add Noise only; Dust & Scratches, Despeckle, Median, and Reduce Noise are **16-bit max**, not 32-bit (per the sourced lists) — they must be disabled on 32-bit documents. Add Noise must not clamp 32-bit values.
- **CMYK / Lab / Grayscale / Multichannel.** Add Noise and Median operate per channel; Reduce Noise's luminance/chroma split assumes a luminance-carrying space — for CMYK/Lab define the luma axis (L in Lab) and refuse if undefined (e.g. Indexed, which is refused anyway).
- **Randomness and determinism.** A fresh apply must differ; a redo of the same record must be bit-identical. Store the seed.
- **Monochromatic vs color.** Monochromatic must not shift hue; per-channel must produce colored speckle.
- **Threshold 0 in Dust & Scratches.** Replaces every dissimilar pixel — effectively smooths the whole neighborhood and can blur; the documented workflow starts at 0 then raises it.
- **Radius at image bounds.** Clamp-to-edge; never read outside the tile/ROI beyond the apron; tiny images must not panic.
- **Huge PSB.** Tile-local; the undo cost of a full-canvas Add Noise is bounded by the history/tile-diff budget (`ARCH-009`).
- **Reduce Noise memory/CPU.** Primarily CPU in CS6; the design must keep it responsive (progress + cancel) and must not require a GPU.
- **GPU unavailable / device lost.** CPU path for all noise filters.
- **Cancellation / undo.** Atomic; no partial writes; one history state (or none on cancel).
- **Selection edges.** Neighborhood filters sample the apron; Dust & Scratches/Median may smooth across a selection edge by design. Document and test.

## Parity acceptance criteria

1. Given a uniform mid-gray image, Add Noise adds zero-mean speckle: the mean is unchanged within tolerance and the standard deviation grows with Amount; Gaussian produces a Gaussian histogram, Uniform a flat one.
2. Given Monochromatic on, changing Add Noise does not change hue (R−G, G−B stay within tolerance); with it off, per-channel speckle appears.
3. Given two Add Noise applies with the same seed, output is bit-identical; with different seeds, output differs; a redo of a recorded apply reproduces the first exactly.
4. Given an image with salt-and-pepper noise, Median (radius 1) removes isolated pixels while preserving a step edge within a small band.
5. Given a scratch over a smooth region, Dust & Scratches with radius r and a low threshold removes it; raising the threshold to the point where the scratch differs by less than the threshold leaves it; increasing radius smooths more of the image.
6. Given Despeckle, isolated noise is smoothed while strong edges are preserved (edge profile within tolerance of the input).
7. Given a noisy image, Reduce Noise with increasing Strength reduces luminance-noise variance monotonically; increasing Preserve Details retains more edge contrast at equal Strength; Reduce Color Noise reduces chroma variance while preserving luminance detail.
8. Given Advanced per-channel mode, setting Strength on a single channel reduces that channel's noise more than a global correction, per the sourced guidance.
9. Given `Remove JPEG Artifacts` on a low-quality JPEG, blocky edges and halos are reduced relative to the same settings with it off.
10. Given a 32-bit HDR document, only Add Noise is enabled among the Noise filters; Dust & Scratches, Despeckle, Median, and Reduce Noise are disabled with a reason.
11. Given any apply, undo restores every touched pixel bit-exactly and adds exactly one history state; cancelling adds none.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — Adobe Photoshop CS6 Help reference (text-extracted). Established: per-filter descriptions for Add Noise (Uniform ±amount vs Gaussian bell curve, Monochromatic tonal-only), Despeckle (detect edges, blur all except edges), Dust & Scratches (radius/threshold balancing, radius 1–16, threshold 0–255 with finer control 0–128, procedure), Median (radius search, discard dissimilar, replace with median brightness, motion use), Reduce Noise (Strength, Preserve Details 100 = most detail/least denoise, Reduce Color Noise, Sharpen Details, Remove JPEG Artifacts, Advanced per-channel Channel menu with Strength + Preserve Details, luminance vs color noise); `Filter > Noise` menu paths; 16-/32-bit availability (Add Noise, Despeckle, Dust & Scratches, Median, Reduce Noise at 16-bit; only Add Noise at 32-bit); "filters cannot be applied to Bitmap/Indexed" and RAM-error guidance. Primary source.
- `https://en.wikipedia.org/wiki/Median_filter` — median as a non-linear rank filter, window/boundary handling, edge preservation, sliding-histogram fast median (Huang et al. 1979). Standard reference (fetched).
- `https://en.wikipedia.org/wiki/Additive_white_Gaussian_noise` — additive zero-mean Gaussian noise model. Standard reference (fetched).
- `https://en.wikipedia.org/wiki/Bilateral_filter` — edge-preserving smoothing family for the Reduce Noise luminance path. Standard reference (fetched).
- `https://en.wikipedia.org/wiki/Gaussian_blur` — smoothing/low-pass context for denoise. Standard reference (fetched).

Searches performed (not documents fetched): DuckDuckGo/Brave for modern Photoshop parameter ranges (Add Noise amount, Median radius, Reduce Noise Strength 0–10 with Preserve Details/Reduce Color Noise/Sharpen Details percentages). Marked *(inferred)* where used.

Repository sources (fetched locally): `docs/06-filters/filters-overview.md` (shared pipeline, depth matrix), `docs/06-filters/blur-filters.md` (edge-preserving/smoothing kernels), `docs/01-architecture/gpu-rendering-pipeline.md` (Reduce Noise is CPU), `docs/01-architecture/undo-history.md` (tile diff cost), `docs/05-layers/smart-filters.md`.

## Open questions

- **CS6 ranges/defaults.** The fetched PDF states ranges only for Dust & Scratches (radius 1–16, threshold 0–255). All other ranges and defaults are *(inferred)* from modern Photoshop/community. Resolve by scripted CS6 dialog reads.
- **Add Noise Amount scale.** Whether the modern 0.1–400% range and 12.5% default apply to CS6, and how amount maps to uniform vs Gaussian σ, is unverified. Resolve by histogram measurement.
- **Add Noise redo behavior in CS6.** Whether CS6 reproduces the exact noise on redo is unknown; our design stores a seed to guarantee it.
- **Despeckle algorithm.** The edge detector and smoothing operator are closed. Resolve by probing impulse/edge responses.
- **Dust & Scratches local statistic.** Whether the reference is a median, a minimum/maximum, or a thresholded mean is undocumented. Resolve by impulse-response probes.
- **Reduce Noise algorithm.** Adobe does not publish it. Candidate families (bilateral, non-local means, wavelet) must be chosen and matched qualitatively. Resolve with a denoise comparison harness.
- **Reduce Noise per-channel scope.** Whether "Channel" in Advanced covers only RGB channels or also alpha/spot channels, and how it behaves in CMYK/Lab, is unverified.
- **32-bit gating.** Confirm that CS6 disables Despeckle/Dust & Scratches/Median/Reduce Noise on 32-bit exactly as the list implies, and the exact UI treatment (hidden vs disabled).
- **Defaults.** Add Noise distribution default, Median radius default, and all Reduce Noise defaults are unverified.
- **GPU acceleration scope.** Whether any noise filter beyond Add Noise was OpenCL-accelerated in CS6 is unknown; the design treats Reduce Noise as CPU-first per the available evidence.
