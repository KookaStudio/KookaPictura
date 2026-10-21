# Blur Filters

- **Spec ID:** `FILT-010`
- **Status:** `Draft`
- **Parity tier:** `Core` — the Blur submenu is in CS6 Standard. The CS6 **Blur Gallery** (Field Blur, Iris Blur, Tilt-Shift) is a separate CS6 addition and is specified in `06-filters/blur-gallery.md`.
- **New in CS6:** `Changed` — **Box Blur** and **Shape Blur** are part of the CS6 blur set, and the **Blur Gallery** is new (its on-canvas tools are not this spec). The classic Blur-submenu filters (Average, Blur, Blur More, Gaussian, Lens, Motion, Radial, Smart, Surface) predate CS6 and are unchanged.
- **Depends on:** `FILT-001` filters-overview, `ARCH-006` gpu-rendering-pipeline, `ARCH-009` undo-history, `LAY-004` layer-masks, `IMG-009` 32-bit-hdr, `ARCH-007` color-management.

> All module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Adobe's exact kernels are closed; parity is
> **behavioral parity only, algorithm TBD**. The CS6 Help describes each filter's
> controls and qualitative behavior; algorithm families below are drawn from
> standard image-processing references (fetched), not from Adobe source. Parameter
> ranges are *(inferred)* where the fetched primary source does not state them.

## CS6 behavior

Blur filters soften a selection or a whole image by averaging the pixels adjacent to hard edges in lines and shaded areas. The Blur submenu:

- **Average** — finds the average color of the image or selection and fills it with that color, producing a smooth homogeneous patch.
- **Blur / Blur More** — eliminate noise at significant color transitions; Blur More's effect is **three or four times stronger** than Blur. Neither has a dialog.
- **Box Blur** — blurs using the average color of neighboring pixels; the **radius** sets the size of the area averaged; a larger radius blurs more.
- **Gaussian Blur** — blurs a selection quickly by an adjustable amount; applies a bell-curve weighted average, adding low-frequency detail and producing a hazy effect.
- **Lens Blur** — adds depth-of-field blur controlled by a **depth map** (a selection, an alpha channel, or a layer mask); some objects stay in focus while others blur; the iris shape (number of blades, curvature, rotation) controls the bokeh character; specular highlights and re-injected noise are controllable.
- **Motion Blur** — blurs along a direction from **–360° to +360°** at an intensity from **1 to 999**, analogous to a fixed-exposure shot of a moving subject.
- **Radial Blur** — simulates zoom or rotation blur. **Spin** blurs along concentric circular lines (a degree of rotation); **Zoom** blurs along radial lines (a value **1 to 100**). **Quality** is **Draft / Good / Best** (Draft fast but grainy; Good and Best smooth, indistinguishable except on a large selection). **Blur Center** sets the origin.
- **Shape Blur** — uses a chosen custom-shape **kernel** preset; the **radius** sets kernel size, hence blur amount.
- **Smart Blur** — blurs an image with precision. **Radius** = size of the area searched for dissimilar pixels; **Threshold** = how dissimilar pixels must be before they are affected. **Mode**: **Normal** (whole selection), **Edge Only** (black-and-white edges), **Overlay Edge** (white edge overlay). A **Quality** control exists.
- **Surface Blur** — blurs while preserving edges; **Radius** = area sampled; **Threshold** = how much neighboring pixels' tonal values must diverge from the center before being excluded from the blur. Useful for noise/grain removal.

**Selection-edge caveat (sourced):** Gaussian Blur, Box Blur, Motion Blur, and Shape Blur use image data from **outside the selected area**, so a background selection next to sharp foreground can pick up foreground color and produce a fuzzy, muddy outline. The Help recommends **Smart Blur or Lens Blur** in that situation.

**Blur Gallery** (Field/Iris/Tilt-Shift) is CS6-new and lives in its own workspace with on-image pins and Bokeh controls; it is out of scope here.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Filter > Blur > Average` | menu (immediate) | — | No dialog |
| `Filter > Blur > Blur` | menu (immediate) | — | No dialog |
| `Filter > Blur > Blur More` | menu (immediate) | — | No dialog; ~3–4× Blur |
| `Filter > Blur > Box Blur` | dialog | — | Radius |
| `Filter > Blur > Gaussian Blur` | dialog | — | Radius; preview + on-canvas |
| `Filter > Blur > Lens Blur` | dialog | — | Depth map, iris, specular, noise |
| `Filter > Blur > Motion Blur` | dialog | — | Angle, Distance; preview |
| `Filter > Blur > Radial Blur` | dialog | — | Spin/Zoom, Quality, Blur Center |
| `Filter > Blur > Shape Blur` | dialog | — | Shape preset, Radius |
| `Filter > Blur > Smart Blur` | dialog | — | Radius, Threshold, Quality, Mode |
| `Filter > Blur > Surface Blur` | dialog | — | Radius, Threshold |
| `Filter > Blur > Field/Iris/Tilt-Shift` | workspace | — | CS6 Blur Gallery; see `blur-gallery.md` |
| `Filter > Blur > Lens Blur` depth source | combo | — | None / Transparency / Layer Mask / Selection |
| Filter dialog preview | widget | +/- | Drag to recenter |

## Parameters & ranges

| Filter | Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|---|
| Average | — | — | — | — | No controls; region = selection or layer |
| Blur / Blur More | — | — | — | — | No controls |
| Box Blur | Radius | float px | *(unverified)* | *(unverified; likely 1–1000)* | Larger radius = greater blur |
| Gaussian Blur | Radius | float px | *(unverified)* | 0.1–1000 *(modern PS; CS6 unverified)* | Bell-shaped weighted average |
| Gaussian Blur | Preview | bool | on | on/off | |
| Motion Blur | Angle | int ° | 0 *(inferred)* | –360 … +360 | Direction of motion |
| Motion Blur | Distance | int px | 20 *(inferred)* | 1 … 999 | Sourced range |
| Radial Blur | Blur Method | enum | Spin | Spin / Zoom | |
| Radial Blur | Amount | int | 10 *(inferred)* | 1–100 (Zoom) / degrees of rotation (Spin) | Sourced "degree of rotation" for Spin |
| Radial Blur | Quality | enum | Good *(inferred)* | Draft / Good / Best | |
| Radial Blur | Blur Center | point | image center | drag in box | Origin of blur |
| Shape Blur | Shape | preset | *(unverified)* | custom-shape library | |
| Shape Blur | Radius | float px | *(unverified)* | *(unverified)* | Kernel size |
| Smart Blur | Radius | float px | *(unverified)* | 0.1–100 *(modern; CS6 unverified)* | Search area for dissimilar pixels |
| Smart Blur | Threshold | float | *(unverified)* | 0.1–100 *(modern; CS6 unverified)* | Dissimilarity gate |
| Smart Blur | Quality | enum | *(unverified)* | *(Low/Medium/High inferred)* | |
| Smart Blur | Mode | enum | Normal | Normal / Edge Only / Overlay Edge | Sourced |
| Surface Blur | Radius | int px | 5 *(inferred)* | 1–100 *(inferred)* | Area sampled |
| Surface Blur | Threshold | int levels | 15 *(inferred)* | 1–255 *(inferred)* | Tonal divergence gate |
| Lens Blur | Preview | enum | Faster | Faster / More Accurate | Sourced; preview sampling only, never committed |
| Lens Blur | Depth Map Source | combo | None | none / transparency / layer mask / selection | Black = front, white = far |
| Lens Blur | Blur Focal Distance | int | 126 | 0–255 | Depth in focus |
| Lens Blur | Invert | bool | off | on/off | Invert depth map |
| Lens Blur | Shape | preset | Triangle (3) | Triangle (3) … Octagon (8) | Iris/bokeh shape |
| Lens Blur | Blade Curvature | int % | 33 | 0–100 | Rounds the iris |
| Lens Blur | Rotation | int ° | 191 | 0–360 | Rotates the iris |
| Lens Blur | Radius | int px | 50 | 0–100 | Blur amount |
| Lens Blur | Specular Threshold | int | 93 | 0–255 | Brightness cutoff for highlights |
| Lens Blur | Specular Brightness | int | 44 | 0–100 | Highlight boost |
| Lens Blur | Noise Distribution | enum | Gaussian | Uniform / Gaussian | Re-inject grain |
| Lens Blur | Monochromatic | bool | off | on/off | Noise without color change |
| Lens Blur | Noise Amount | int | 47 | 0–100 | Grain amount |

Lens Blur's defaults and ranges are pinned to the CS6 dialog in issue #338.
The remaining `(inferred)` values come from modern Photoshop dialogs and
community documentation, not the fetched CS6 PDF; see `## Open questions`.

## Algorithms & pipeline

Adobe kernels are not published; the following are standard algorithm families that reproduce the documented behavior. All are proposed as **behavioral parity only**.

### Gaussian Blur — separable Gaussian convolution
- Convolve with a 2-D Gaussian; because the kernel is **separable**, implement as two 1-D passes (horizontal then vertical) with cost ≈ `O(2·N·r)` per tile.
- Discrete kernel radius ≈ `⌈3σ⌉` (contributions beyond 3σ are negligible); normalize the kernel so the sum is 1. `σ` is derived from the UI **radius** (Adobe's radius↔σ mapping is closed; propose `radius` as the 3σ support, i.e. `σ = radius/3`, and tune).
- Alternatives for large radii: three successive **box blurs** approximate a Gaussian, or an IIR/Deriche-style recursive filter; the FIR kernel is the correctness baseline.
- Reference: Wikipedia *Gaussian blur* (separable FIR, normalization, 3σ cutoff, box-blur/IIR alternatives).

### Box Blur — moving average
- Each output pixel = mean of a `(2r+1)²` neighborhood. Separable; can be computed in **O(1) per pixel** with a running sum (or a summed-area/integral image).
- Reference: Wikipedia *Box blur*.

### Motion Blur — 1-D line convolution
- Convolve along a line through the pixel at the given **Angle**, with length **Distance** (1–999) — a 1-D box (or linear-ramp) kernel. Non-separable in the general-angle case; sample along the line with supersampling to reduce aliasing (odd/even distance).
- Reference: Wikipedia *Motion blur* (direction + distance streaking).

### Radial Blur — polar resampling + angular/radial smearing
- **Spin:** convert to polar `(θ, r)`, take a 1-D moving average along `θ` over the rotation amount, resample back. Cost/quality = number of angular samples: Draft (few, grainy), Good, Best (many).
- **Zoom:** take a 1-D moving average along the radial direction (scale smear), then resample back. Amount 1–100 maps to the smear length.
- Both center on **Blur Center**; sample with interpolation.

### Surface Blur — thresholded window mean (bilateral family)
- Per channel, the mean of the neighbours in the `(2·Radius+1)²` window (clipped to the image) whose value is within **Threshold** of the centre's: a bilateral filter with a box spatial kernel and a hard range gate. The gate stops averaging across edges, and each channel decides independently which neighbours count. Photoshop's Surface Blur is documented as a bilateral filter; this model is photorust's CS6-tuned approximation (#222), not a verified parity.
- Computed with a sliding per-row histogram, `O(r)` per pixel rather than the direct form's `O(r²)`.
- Extensions/limitations: a Gaussian spatial or soft range kernel is the textbook bilateral; a **guided filter** is a known faster alternative for those.
- Reference: Wikipedia *Bilateral filter* (definition, σd/σr, artifact list, explicit "Photoshop surface blur" note).

### Average — region mean
- Compute the mean color over the selection (or layer bounds) and fill; `O(N)`. For 32-bit, average in float and do not clamp to 1.0 (`IMG-009`).

### Lens Blur — depth-map-driven aperture convolution
- **Depth map** `d(x)` (0 = near/front, 255 = far) from selection/alpha/mask; `|d − focal|` selects the blur radius at each pixel (a "circle of confusion" ramp). Adobe documents focal distance semantics: with focal 100, pixels at 1 and 255 are fully blurred and pixels near 100 are blurred least; clicking the image sets focal distance.
- Blur with a **disc/iris-shaped** kernel whose shape is the aperture: number of blades → polygon; **Blade Curvature** rounds the polygon toward a circle; **Rotation** rotates it. This produces the characteristic bokeh (out-of-focus point lights repeat the aperture shape).
- **Specular highlights:** pixels brighter than **Specular Threshold** contribute extra brightness (additive highlight kernel), so out-of-focus highlights bloom rather than dim.
- **Noise:** re-inject Uniform or Gaussian grain (optionally Monochromatic) because blurring removes original grain.
- **Preview Faster/More Accurate** changes sampling density only.
- Reference: Wikipedia *Bokeh* (aperture-shaped out-of-focus blur).

### Smart Blur — edge-aware selective blur
- Detect significant color changes; blur only the non-edge pixels. The **Threshold** gates dissimilarity; **Radius** is the search window; **Mode** either blurs normally, draws black-and-white edge lines (**Edge Only**), or overlays white edges on the blurred result (**Overlay Edge**). Algorithm family: edge detection (gradient/threshold) + threshold-limited averaging; the exact Adobe edge kernel is closed.

### Shape Blur — kernel-shaped convolution
- Convolve with a binary/antialiased **custom shape** kernel (scaled by radius) instead of a disc/Gaussian. Family: arbitrary-kernel convolution, usually via summed-area table across a downsampled rotated/scaled shape.

### Blur / Blur More
- A Gaussian at a fixed σ: **0.7** for Blur and **2.0** for Blur More (radius 2.1 and 6 under `FILT-010`'s 3σ convention), so Blur More's blur reaches about three times as far, in line with the Help's three-to-four-times-stronger description (sourced). The σ values are photorust's CS6-tuned approximation (#222); Adobe's kernel is closed. No controls.

### Shared pipeline
Apply through the `FILT-001` pipeline: target resolution → depth/mode gate → **apron** (blur kernels need `radius` of source outside the selection) → backend dispatch (GPU compute if ported, else `rayon` CPU) → tile iteration with progress/cancel → selection/mask composite → single undo commit. Color-managed docs convolve in the document working space; 32-bit float stays float.

## Rust module mapping

Proposed under `pictura-filters::blur` (`ARCH-002`, `ARCH-006`):

- `pictura-filters::blur::gaussian` — `GaussianBlur { radius }`; separable FIR (baseline), optional box-approx and IIR paths; `fn sigma_from_radius`.
- `pictura-filters::blur::box` — `BoxBlur { radius }`; running-sum separable implementation.
- `pictura-filters::blur::motion` — `MotionBlur { angle, distance }`; line kernel + supersampling.
- `pictura-filters::blur::radial` — `RadialBlur { method: Spin|Zoom, amount, quality, center }`; polar resampler + 1-D smear.
- `pictura-filters::photorust::convolve::surface_blur` — `SurfaceBlur { radius, threshold }`; thresholded window mean on a sliding histogram.
- `pictura-filters::blur::smart` — `SmartBlur { radius, threshold, quality, mode }`; Gaussian mean over the neighbours whose every channel is within Threshold levels, Quality thinning the search disc (≤ 3 / 6 / 12 taps a side for Low / Medium / High), and a 4-neighbour edge map for Edge Only / Overlay Edge *(approximation; #339)*.
- `pictura-filters::blur::average` — `Average`; region mean.
- `pictura-filters::blur::lens` — `Filter::LensBlur { depth: Option<Arc<DepthMap>>, focal, invert, shape, radius, curvature, rotation, specular_brightness, specular_threshold, noise_amount, noise_distribution, monochromatic, faster, seed }`; iris-shaped aperture convolution, specular add, seeded grain injection. The app materializes the chosen source (transparency, layer mask, or selection) into the `DepthMap`; `None` acts as a uniform far depth.
- `pictura-filters::blur::smart` — `SmartBlur { radius, threshold, quality, mode }`; edge-gated averaging.
- `pictura-filters::blur::shape` — `ShapeBlur { kernel: ShapeRef, radius }`.
- `pictura-filters::blur::simple` — Blur / Blur More as Gaussian Blur at the fixed `BLUR_RADIUS` / `BLUR_MORE_RADIUS`.
- Shared: `pictura-filters::kernel::separability`, `Sampler` (bilinear), `apron`.

Types: `Radius(f32)`, `Angle(f32)`, `Iris`, `DepthSource`, `Quality`. Crossing `TileView` ROIs; no Qt types.

## Qt6 component mapping

Widgets, matching the CS6 modal dialogs; pixel work in Rust.

- `GaussianBlurDialog` — `QDoubleSpinBox` radius + preview.
- `MotionBlurDialog` — `QDial`/`QSpinBox` angle + `QSpinBox` distance.
- `RadialBlurDialog` — method `QComboBox`, amount spin, quality `QComboBox`, `BlurCenterWidget` (drag crosshair).
- `BoxBlurDialog` / `SurfaceBlurDialog` — radius (and threshold) controls.
- `SmartBlurDialog` — radius, threshold, quality, mode.
- `LensBlurDialog` — depth-source `QComboBox` (populated from document channels/masks), focal `QSlider`, iris `QComboBox`, curvature/rotation/radius/specular/noise controls, and a **depth-map preview** with click-to-set-focus.
- `BlurFilterPreview` (`QLabel`/`QImage`) — shared preview pane with +/- zoom and drag-to-pan; all dialogs embed it.
- `FilterProgressProxy` — shared cancelable progress.

## Data-model impact

- **No persistent fields** for destructive blur filters. Each apply is one `HistoryRecord::FilterOp { filter_id, roi, params_blob, before_tiles, after_hash }` (`ARCH-009`).
- **Lens Blur** reads an existing depth source (selection, alpha channel, layer mask) but does not create channels; it never mutates the source map.
- **Randomness:** Blur itself is deterministic. Lens Blur's re-injected noise is random; store the RNG **seed** in the record so redo is bit-exact (design choice).
- **Smart Object:** a blur applied to a Smart Object becomes a Smart Filter entry with `{ filter_id, params, blend, opacity, enabled }` (`LAY-021`); not baked.
- **Color/depth:** parameters are stored per filter; the 32-bit float path stores no LUT.

## Edge cases

- **Empty target.** An active layer with no non-transparent pixel inside the selection (or at all, e.g. a new empty layer) refuses every Blur command before its dialog opens, with "Could not complete the `<command>` command because the selected area is empty." and no history state. The Blur Gallery words it "…because the initial bounding rectangle is empty." (`FILT-092`). A layer without transparency is never empty; a layer mask does not count. *Implemented:* `filter_target_empty` + `refuseEmptyBlur` (`frame_menus_filter.cpp`), on the menu, Last Filter, and Last Filter Settings paths.
- **Degenerate radius 0 / distance 1.** Identity or near-identity; still commit consistently (or refuse with no history state).
- **Selection edge.** Gaussian/Box/Motion/Shape sample the apron and may contaminate edges; Smart/Lens must not. Document per-kernel; tests must pin the chosen behavior.
- **Radius larger than the image/tile.** Clamp the kernel support to available source; use reflection/replication at image borders (propose **clamp-to-edge**, matching common Photoshop behavior; unverified).
- **1×1 and 1-px-wide documents.** Separable passes must handle zero-height/width; no panics.
- **Huge PSB documents.** Tile-local; no whole-image buffers; `⌈3σ⌉` support bounded by an apron and possibly multi-pass tiling for very large radii.
- **32-bit HDR.** Lens Blur and Gaussian must not clamp >1.0; Average keeps float precision.
- **CMYK / Lab / Grayscale / Multichannel.** Operate per channel in the document space; Lab radial/blur is fine, but any blend/fade afterward respects `FILT-001` Lab restrictions.
- **Lens Blur missing/invalid depth source.** Refuse with a clear message rather than blurring uniformly.
- **Lens Blur preview "Faster" must never be committed.** Only the final full-quality result is written.
- **GPU unavailable / device lost.** CPU path; identical results within tolerance.
- **Cancellation mid-apply.** Atomic no-op (no history state).
- **Undo/redo.** Bit-exact; randomness via stored seed.

## Parity acceptance criteria

1. Given a solid-color image, Gaussian, Box, and Blur are no-ops (within 1 LSB); given an edge, each produces a smooth transition whose width grows with radius.
2. Given the same radius, Gaussian Blur output matches a separable-Gaussian reference convolution within 2 LSB (8-bit) for `radius ≤ 50`, on both CPU and GPU paths.
3. Given Motion Blur at `angle = 0`, the streak is horizontal; at `angle = 90`, vertical; at `distance = 1`, the result is close to the original.
4. Given Radial Blur Zoom with a centered blur center, the image scales-smears radially; Spin with the same center smears rotationally; Draft is visibly grainier than Best for the same amount on a large image.
5. Given a step edge, Surface Blur with a small Threshold preserves the edge while smoothing flat regions; increasing Threshold smooths across the edge.
6. Given Average on a uniform region, the output is exactly the region mean; on a selection it affects only the selection.
7. Given a depth map that is black at the bottom and white at the top with focal distance 0, Lens Blur keeps the bottom sharp and blurs the top increasingly; clicking the image updates the focal distance and refocuses that depth. A point light well off-focus reproduces the chosen iris shape.
8. Given Smart Blur Mode `Edge Only`, the output is a black-and-white edge map; `Overlay Edge` overlays white edges on the blurred image; `Normal` blurs non-edge areas.
9. Given a selection adjacent to sharp foreground, Smart Blur and Lens Blur do not pick up foreground color at the selection edge, while Gaussian/Box/Motion/Shape may (documented behavior).
10. Given 32-bit HDR input, Gaussian Blur preserves values > 1.0 without clamping.
11. Given a blur apply, undo restores every touched pixel bit-exactly and adds exactly one history state; cancelling adds none.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — Adobe Photoshop CS6 Help reference (text-extracted). Established: per-filter descriptions and qualitative behavior for Average, Blur/Blur More (Blur More ~3–4×), Box Blur, Gaussian Blur, Lens Blur (depth map, focal distance semantics, iris shape/curvature/rotation, specular threshold/brightness, uniform/Gaussian/monochromatic noise), Motion Blur (angle –360…+360, distance 1–999), Radial Blur (Spin/Zoom, amount 1–100, quality Draft/Good/Best, blur center), Shape Blur (custom-shape kernel, radius), Smart Blur (radius, threshold, quality, Normal/Edge Only/Overlay Edge), Surface Blur (radius, threshold, edge-preserving); the selection-edge caveat naming Gaussian/Box/Motion/Shape and recommending Smart/Lens Blur; and the 16-/32-bit availability lists (Box, Gaussian, Motion, Radial, Surface, Shape at 16/32; Lens Blur 16-only; Average at all depths). Primary source.
- `https://en.wikipedia.org/wiki/Gaussian_blur` — separable FIR convolution, 3σ support, normalization, cost, box-blur/IIR alternatives. Standard reference (fetched).
- `https://en.wikipedia.org/wiki/Box_blur` — box blur as spatial moving average. Standard reference (fetched).
- `https://en.wikipedia.org/wiki/Motion_blur` — motion blur as directional intra-exposure streaking. Standard reference (fetched).
- `https://en.wikipedia.org/wiki/Bilateral_filter` — bilateral definition, σd/σr, artifact list, and the explicit statement that Photoshop's Surface Blur is a bilateral filter. Standard reference (fetched).
- `https://en.wikipedia.org/wiki/Bokeh` — out-of-focus blur shaped by the aperture (Lens Blur iris). Standard reference (fetched).

Repository sources (fetched locally): `docs/01-architecture/gpu-rendering-pipeline.md` (GPU/CPU split, tile cache, color), `docs/01-architecture/performance-targets.md` (separable convolution cost model, cancellation), `docs/01-architecture/rust-core-design.md` (`pictura-filters`, `TileView`), `docs/01-architecture/color-management.md` (working space), `docs/06-filters/filters-overview.md` (shared pipeline).

Searches performed (not documents fetched): DuckDuckGo/Brave for modern Photoshop parameter ranges (Gaussian radius 0.1–1000, Smart Blur 0.1–100, Surface threshold); these are marked *(inferred)*.

## Open questions

- **Radius↔σ mapping for Gaussian Blur.** Adobe does not document how the UI radius maps to the Gaussian standard deviation. Resolve by measuring Photoshop's impulse response and fitting σ.
- **CS6 parameter ranges/defaults.** The fetched PDF gives ranges only for Motion Blur distance (1–999), Radial Zoom amount (1–100), and Dust & Scratches (elsewhere). All other ranges/defaults are *(inferred)* from modern Photoshop/community and need confirmation from a CS6 build or scripted dialog read.
- **Box Blur and Shape Blur 16-bit availability.** The CS6 16-bit list includes both; confirm the exact CS6-at-launch dot release. Shape Blur's shape-preset enumeration and default are unknown.
- **Radial Spin amount semantics.** Whether the Spin "amount" is literal degrees or a 1–100 scale is unclear; resolve with measurements.
- **Lens Blur iris definition.** The blade presets and dialog defaults are pinned to the CS6 dialog in #338; the curvature-to-polygon mapping and the specular-highlight compositing formula remain closed. Resolve by matching out-of-focus point-light images.
- **Lens Blur noise distribution defaults.** Distribution (Gaussian) and Monochromatic (off) are pinned to the CS6 dialog in #338; the grain's exact distribution shape remains unverified.
- **Border handling.** Whether Photoshop clamps, reflects, or wraps at image borders for each blur is untested. Resolve with edge-pixel probes.
- **Smart Blur edge kernel.** Exact edge detection and threshold application are closed; `Edge Only`/`Overlay Edge` output convention (black/white assignment) is known but the kernel is not. Behavioral parity.
- **Gaussian precision.** Whether CS6 computes 8-bit blur in 8-bit integer or in higher precision internally affects banding; resolve by gradient tests.
- **GPU port priority.** Which blur kernels must be GPU-accelerated to meet `ARCH-003` (Gaussian is the stated benchmark) is a design choice; the parity requirement is only that CPU results are correct.
