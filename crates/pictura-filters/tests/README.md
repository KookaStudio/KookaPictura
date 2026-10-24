# M6-E / M7-C / M8-B / M9-B / M11-B — ImageMagick filter oracle

These tests diff `pictura_filters::apply` against **ImageMagick** for the
Blur / Sharpen / Noise / Other / Stylize / Pixelate / Distort filters that have
a usable equivalent. This is a *sanity* oracle, not a parity oracle: Adobe's
exact integer math and convolution kernels are closed, and the ImageMagick
operators only approximate several Photoshop paths. The mapping table, the IM
flags, the tolerances and the known divergences are below.

- Oracle script: `scripts/filter_oracle.py`
- Tests: `crates/pictura-filters/tests/oracle.rs`
- Verified against: ImageMagick **7.1.2-29 Q16-HDRI** (2026-07-27).

Only filters with a faithful ImageMagick operator are diffed (`GaussianBlur`,
`BoxBlur`, `Blur`, `BlurMore`, `Median`, `UnsharpMask`, `Sharpen`,
`SharpenMore`, `Maximum`, `Minimum`, `Offset` with `wrap = true`, `Custom`,
`Solarize`, `Mosaic`, `Displace` with Repeat Edge Pixels, `ShapeBlur`). The rest are covered by
ImageMagick-independent property/known-value tests (here and in the module unit
tests); the measured divergences that ruled out a differential test are recorded
below. No test is `#[ignore]`d.

## Filter → ImageMagick mapping

> **Ported engine (#221).** Most filters below now run through the photorust
> engine (`src/photorust`). The faithful rows still hold: Solarize stays on its
> own exact implementation and Mosaic rounds to the exact block mean. In the
> no-equivalent rows, the "observed max delta" figures and the algorithm notes
> for the ported filters (Emboss, Fragment, Mezzotint, Crystallize, Pointillize,
> Facet, ColorHalftone, the Distort warps, Add Noise, Dust & Scratches) describe
> the replaced implementations. The behaviour those filters now have is in
> `openspec/changes/port-photorust-filters`.

Test image: deterministic 16×16 RGB8, two ramps (R = 16x, G = 16y) plus a 4×4
block checker in B, so every filter sees both gradients and hard edges. All
"measured max delta" figures are absolute 8-bit sample deltas against this image.

| `Filter` | ImageMagick | Faithful? | Tolerance | Notes |
|---|---|---|---|---|
| `GaussianBlur` | `-gaussian-blur 0x{sigma}` (`sigma = radius/3`) | yes | 0 | Same 3σ separable Gaussian, clamp-to-edge; measured max delta 0 (radius 3.0 → σ 1.0). |
| `BoxBlur` | `-statistic mean NxN` (`N = 2·radius+1`) | yes | 0 | Same separable moving average, clamp-to-edge; measured max delta 0 (radius 3). |
| `Median` | `-median {radius}` | yes | 0 | Same `(2r+1)²` per-channel rank filter, clamp-to-edge; measured max delta 0 (radius 1). |
| `UnsharpMask` | `-unsharp 0x{sigma}+{amount/100}+{threshold/255}` | yes | 6 | Same blur-difference gain; IM `amount` is a **fraction** (`1.0` = 100 %). IM's internal blur differs slightly from its standalone `-gaussian-blur`; measured max delta 5 at radius 3.0 / amount 150 / threshold 0. |
| `MotionBlur` | — | **no** | — | IM `-motion-blur` builds a **one-sided Gaussian line** kernel; Pictura averages symmetric uniform taps. Observed max delta 86 (`-motion-blur 0x5+0`). |
| `RadialBlur` | — | **no** | — | ImageMagick 7 has **no** `-radial-blur` (IM 6 only); `-rotational-blur` weights a different angle profile. Observed max delta 58 (Spin 20 vs `-rotational-blur 20`). |
| `Average` | — | **no** | — | Trivial global region mean; no IM operator shares the window/border semantics. Observed max delta 92 (`-statistic mean 16x16`). |
| `Blur` | `-gaussian-blur 0x0.7` | yes | 0 | A Gaussian at the fixed σ 0.7 (radius 2.1); measured max delta 0. |
| `BlurMore` | `-gaussian-blur 0x2` | yes | 1 | A Gaussian at the fixed σ 2.0 (radius 6); IM's kernel at this σ differs by rounding; measured max delta 1. |
| `SurfaceBlur` | — | **no** | — | Per-channel mean of the neighbours within the threshold; IM has no thresholded-mean operator. Observed max delta 71 (radius 3 / threshold 20 vs `-gaussian-blur 0x1`). |
| `LensBlur` | — | **no** | — | A depth-map-driven aperture convolution: ImageMagick has no depth-aware iris blur, no circle-of-confusion ramp, no specular bloom and no seeded grain. Guarded by the focus, ramp, iris, specular and noise unit tests. |
| `Sharpen` | `-unsharp 0x1+0.5+0` | yes | 2 | Unsharp Mask at the fixed 50 % / σ 1; IM's internal blur differs slightly; measured max delta 2. |
| `SharpenMore` | `-unsharp 0x1+1+0` | yes | 3 | Unsharp Mask at the fixed 100 % / σ 1; IM's internal blur differs slightly; measured max delta 3. |
| `SharpenEdges` | — | **no** | — | Unsharp gated by a smoothstep of the Sobel edge strength; IM has no edge-gated sharpen. Observed max delta 44 (`-sharpen 0x1`). |
| `AddNoise` | — | **no** | — | RNG streams differ; the contract is same-seed determinism. Observed max delta 79 (`-attenuate 0.1 +noise Gaussian`). |
| `Despeckle` | — | **no** | — | IM `-despeckle` uses a different rank detector. Observed max delta 13 (`-despeckle`). |
| `Maximum` | `-morphology Dilate Square:{radius}` | yes | 0 | Same separable `(2r+1)²` square grayscale dilate, clamp-to-edge; measured max delta 0 (radius 2). **`Square:N` takes a radius** (kernel diameter `2N+1`), so pass `Square:{radius}`; the M7 plan's `N = 2·radius+1` would dilate over a `(4r+3)²` footprint. |
| `Minimum` | `-morphology Erode Square:{radius}` | yes | 0 | Same separable `(2r+1)²` square grayscale erode, clamp-to-edge; measured max delta 0 (radius 2). Same `Square:N` radius note as `Maximum`. |
| `Offset` | `-roll {+h}{+v}` (`wrap = true`) | yes | 0 | `wrap = true` is an exact integer roll (positive = right/down); measured max delta 0 at `(3,2)` and `(2,3)`. `repeat = true` clamps to the nearest edge, and `wrap = false` fills the exposed area with `background`; neither is a plain `-roll` case (background fill measured max delta 240 vs `-roll +3+2`), so both are no-equivalent and guarded by known-value tests. |
| `HighPass` | — | **no** | — | No single IM operator. A hand-built `\( +clone -gaussian-blur 0x{σ} \) -compose Mathematics -define compose:args=0,-1,1,0.5 -composite` re-implements the formula and is within measured max delta 1 (radius 3.0, σ 1.0), but it is not an independent operator; guarded by the flat-field mid-gray test. |
| `Custom` | `-convolve {kernel}` with `convolve:scale` + `-evaluate add` | yes | 0 | Same f64 5×5 convolution, clamp-to-edge. IM normalizes by the kernel sum, so the matching scale is `sum(kernel)/scale` and the offset is `-evaluate add ({offset}/255)`. Measured max delta 0 (edge kernel: scale 4/offset 8 and scale 9/offset −10). |
| `Emboss` | — | **no** | — | IM `-emboss` is per-channel with a fixed diagonal kernel and no angle/height/amount; Pictura is an angle-directed second difference on **luma** with an achromatic output. No angle/radius/σ matches: observed max delta 210 (angle 135 vs `-emboss 0x1`; best case 186 on the axis-aligned angles). |
| `FindEdges` | — | **no** | — | IM `-edge` is a different detector and renders edges **bright on dark**; Pictura's Sobel magnitude is inverted (dark on light). Observed max delta 255 vs `-edge 1`. |
| `Solarize` | `-solarize 50%` | yes | 0 | Same fixed 50 % inversion curve (`v ≥ 128 → 255 − v`); measured max delta 0. |
| `Mosaic` | `-filter box -resize {W/n}x{H/n}! -filter point -resize WxH!` (`n = cell_size`) | yes | 0 | Exact top-left block mean when the cell divides both dimensions; measured max delta 0 (cell 4 on 16×16). For non-divisor cells IM's resize window is offset from Pictura's top-left blocks (observed max delta 80–136 at cell 3/5/6/7), so the differential uses cell 4. |
| `Crystallize` | — | **no** | — | Seeded Voronoi cells; no IM operator. Closest approximations `-kuwahara 4` (observed max delta 118) and `-paint 4` (157). Guarded by seed determinism + flat-field identity. |
| `Facet` | — | **no** | — | Similar-neighbor banded 3×3 mean; IM `-statistic mean 3x3` is closest (observed max delta 76; `-kuwahara 1` gives 144). Guarded by flat-field identity and gradient flattening. |
| `Fragment` | — | **no** | — | Clamp-anchored sliding 2×2 mean; `-statistic mean 2x2` uses a different window/edge rule (observed max delta 85; `-kuwahara 1` gives 59). Guarded by the 4-tap known-value test. |
| `Mezzotint` | — | **no** | — | Seeded procedural dot/line pattern; IM `-threshold 50%` and `-ordered-dither` are different screens (observed max delta 255). Guarded by seed determinism, binarity and achromatic output. |
| `Pointillize` | — | **no** | — | Seeded local-color dots over background; `-spread 2` displaces pixels instead of drawing dots (observed max delta 240; `-kuwahara 2` gives 232). Guarded by seed determinism and the source/background color set. |
| `ColorHalftone` | — | **no** | — | Per-channel rotated screen; `-ordered-dither o8x8` / `h4x4a` are fixed orthogonal screens (observed max delta 255). Guarded by determinism and binarity. |
| `Twirl` | — | **no** | — | IM `-swirl {angle}` uses a **smooth falloff** and radius `min(w,h)/2`; Pictura twirls with a **linear falloff** over `max(cx,cy)`. Best measured max delta 124 (Twirl +45 vs `-swirl 45`, same sign); −90 vs `-swirl 90` gives 170. Guarded by the zero no-op and rotation tests. |
| `Pinch` | — | **no** | — | IM `-implode` amount is a **fraction** (`0.5` = 50 %, not `50`); positive implodes, negative explodes. Pictura's linear radial remap is not IM's implode falloff. Best measured max delta 61 (Pinch 50 vs `-implode 0.5`); Pinch −50 vs `-implode -0.5` gives 84. |
| `Spherize` | — | **no** | — | Closest is `-implode {amount/100}`; Pictura's arc-length sphere map (Normal / HorizontalOnly / VerticalOnly) is not IM's implode falloff. Best measured max delta 159 (Spherize 50 Normal vs `-implode 0.5`); −50 gives 128. |
| `Ripple` | — | **no** | — | IM `-wave {amp}x{period}` displaces **one axis** with a sine in x and pads the canvas (background fill); Pictura Ripple displaces **both axes** (`dx ∝ sin y`, `dy ∝ sin x`) with clamp-to-edge and a fixed period per size. Measured max delta 255 / mean 104 (Ripple 100 Medium vs `-wave 10x16`, cropped back to 16×16). |
| `Wave` | — | **no** | — | IM `-wave {amp}x{wavelength}` is a single **unseeded** sine along one axis; Pictura sums seeded generators with random phase/period/amplitude and an axis-wise scale. Measured max delta 255 / mean 101 (1 generator sine, amp 20, wavelength 10, seed 42 vs `-wave 20x10`, cropped). |
| `PolarCoordinates` | — | **no** | — | Closest `-distort Polar 0` (RectangularToPolar) / `-distort DePolar 0` (PolarToRectangular); IM uses a different angle origin (≈180° off) and pixel-center/radius anchor plus its own resampling. Best measured max delta 189 (mean 58.3) and 194 (58.9); the crossed directions give max 240. Guarded by the remap/no-op property tests. |
| `Shear` | — | **no** | — | IM `-shear {angle}x0` is a whole-canvas horizontal shear that **background-fills** the expanded canvas; Pictura shifts each row by a smooth spline through the control points with clamp/wrap and expands nothing. Curve `[(-1,-0.5),(1,0.5)]` = 26.565° (IM's sign runs the other way); best measured `-shear -26.565x0 -crop 16x16+4+0 +repage` max 255 / mean 18.4 (WrapAround 23.4). Guarded by the zero-curve no-op and fill-mode tests. |
| `Displace` | `MAP -compose displace -define compose:args={1.275·h}x{1.275·v} -composite` (Repeat Edge Pixels) | yes | 8 | Same lookup: each pixel reads the source `(v−128)/128 × 1.28·scale` px away, with the map's red plane across and green down, bilinear, edges repeated. IM reaches `X` px at full white and centres on 127.5, so `X = 1.275·scale`; that half-level offset, across the 170-level checker edges, is the delta: measured max delta 5 / mean 0.5 at 4×4 and 6×−5 on a ramp map, 9 at 8×−6. Wrap Around (`-virtual-pixel tile`) is not diffed: Pictura wraps the position but blends the edge pixel with itself, where IM blends across the seam (57 of 768 samples over 9, max 125). |
| `ZigZag` | — | **no** | — | IM `-swirl` uses a smooth falloff about `min(w,h)/2`; Pictura's cosine radial profile is pinned to zero at the edge with `ridges` reversals. Best measured `-swirl 50` (amount 80 / ridges 5 / AroundCenter) max 170 / mean 14.3. Guarded by the zero no-op and style tests. |
| `OceanRipple` | — | **no** | — | IM `-wave` is an unseeded single-axis sine that pads the canvas; Pictura refracts through the slope of seeded 2-D value noise with clamp-to-edge. The old sinusoid model measured `-wave 2x8 -crop 16x16+0+2 +repage` max 227 / mean 53.7; the current model was not remeasured. Guarded by seed determinism, the zero-magnitude no-op and the reach test. |
| `DustAndScratches` | — | **no** | — | IM `-statistic median NxN` is an **ungated** rank filter; Pictura replaces a pixel only when it differs from the local median by more than `threshold`, and no IM operator exposes that gate. Guarded by the speck-removal and determinism unit tests. |
| `Extrude` | — | **no** | — | No ImageMagick extrusion renderer; the block/face geometry plus the solid-front, level-based and mask-incomplete options have no operator. Guarded by the changed-image and determinism unit test. |
| `Tiles` | — | **no** | — | No IM tiled-offset-with-fill operator; `-roll` and `-spread` neither offset a fixed grid nor fill the gaps with the foreground/background choice. Guarded by the determinism unit test. |
| `TraceContour` | — | **no** | — | No IM per-channel level-crossing contour operator; `-edge`, `-morphology` and `-threshold` are different detectors. Guarded by the contour unit tests. |
| `Wind` | — | **no** | — | No IM horizontal-streak operator; `-motion-blur`, `-spread` and `-wave` displace pixels differently. Guarded by the determinism unit test. |
| `SmartSharpen` | — | **no** | — | A deconvolution of a sharp-core + halo blur model; IM's `-unsharp` is a plain blur difference and has no deconvolution operator, so the variant is no-equivalent. Guarded by the core-inverse, halo-convergence, remove-path and determinism unit tests. |

### Why `MotionBlur` is not diffed

The M6 plan suggested `-motion-blur 0x{distance}+{angle}`, but the two operators
are not comparable. An impulse test (a single white pixel) shows ImageMagick's
`-motion-blur` response is a **one-sided** Gaussian-weighted line:

```
magick -motion-blur 2x1.0+0   impulse centre row: [2, 20, 88, 145]  (left of centre)
magick -motion-blur 0x5+0     impulse centre row: [10,14,18,23,27,31,35,37,38, …]
```

Pictura's `MotionBlur` averages `distance` taps placed **symmetrically** about
the pixel (`half = (distance−1)/2`, uniform weights). No `-motion-blur`
invocation reproduces that, and `-convolve` with the same kernel would just
re-implement it, so no faithful operator exists. The observed delta against the
suggested invocation is recorded above; the tap geometry is guarded by
`motion_blur_known_values`.

### `Maximum` / `Minimum`: `Square:N` is a radius, not a diameter

The M7 plan suggested `N = 2·radius + 1` for `-morphology Dilate/Erode Square:N`.
That is wrong: ImageMagick's `Square:N` parameter is a **radius**, so the kernel
is `(2N+1)×(2N+1)`. `-morphology Dilate Square:3` prints a `7x7` kernel. The
faithful flag is therefore `Square:{radius}`, and a 3×3 footprint is
`Square:1`. Measured against `pictura_filters` radius 2 (5×5 square,
clamp-to-edge): max delta 0 with `Square:2`, non-zero with `Square:5` (the plan's
suggested value). The separable dilate/erode commutes, so the integer result is
bit-exact.

### `Custom`: mapping the divisor and the bias onto `-convolve`

Pictura computes `Σ kernel·neighbor / scale + offset` with f64 accumulation and
clamp-to-edge. ImageMagick's `-convolve` first normalizes by the kernel sum
(`ΣK`), then multiplies by `-define convolve:scale=...`. The equivalent scale is
therefore `ΣK / scale`, and the additive bias is applied afterwards as
`-evaluate add ({offset}/255)%` (IM `-evaluate add` takes a fraction of the
quantum range). With the shared 5×5 edge kernel (`ΣK = 29`), scale 4 / offset 8
and scale 9 / offset −10 both measured max delta 0.

### `Offset`: `wrap` decides the equivalent

`wrap = true` is a whole-pixel cyclic shift, exactly `-roll +h+v` (positive =
right/down); measured max delta 0 at `(3,2)` and `(2,3)`. `repeat = true`
clamps to the nearest edge sample, and `wrap = false` fills the exposed strip
with `background`; neither is a plain `-roll` case, so both are no-equivalent
(the background fill measured max delta 240). Both are guarded by known-value
tests instead.

## Exact ImageMagick flags

```bash
magick -size 16x16 -depth 8 rgb:IN.rgb \
    <OPERATOR ARGS> \
    -depth 8 rgb:OUT.rgb
```

The script handles a raw planar (`--planar`, channel planes like
`PixelBuffer::data`) or interleaved `rgb`/`rgba` 8-bit image and converts to the
interleaved form ImageMagick reads. `--channels 4` uses `rgba:`, but most IM
operators also touch alpha, so the tests use 3 channels and alpha is never in
play.

| Filter | Flags |
|---|---|
| Gaussian Blur | `-gaussian-blur 0x{sigma}` |
| Box Blur | `-statistic mean {N}x{N}`, `N = 2·radius + 1` |
| Shape Blur | `-define convolve:scale=! -morphology Convolve {S}x{S}:{weights}`, Pictura's coverage kernel, `S = 2·radius + 1` |
| Motion Blur | `-motion-blur 0x{distance}+{angle}` (documented, **not** used by a differential test) |
| Median | `-median {radius}` |
| Unsharp Mask | `-unsharp 0x{sigma}+{amount/100}+{threshold/255}` |
| Maximum | `-morphology Dilate Square:{radius}` (IM `Square:N` is a radius) |
| Minimum | `-morphology Erode Square:{radius}` |
| Offset (`wrap = true`) | `-roll {+h}{+v}` |
| Custom | `-define convolve:scale={ΣK/scale} -convolve {kernel} [-evaluate add {offset/255}%]` |
| Solarize | `-solarize 50%` |
| Mosaic | `-filter box -resize {W/n}x{H/n}! -filter point -resize {W}x{H}!`, `n = cell_size` (cell must divide both dimensions) |
| Emboss | `-emboss {radius}x{sigma}` (measured, **not** used by a differential test) |
| Twirl | `-swirl {angle}` (measured, **not** used by a differential test) |
| Pinch | `-implode {amount/100}` (measured, **not** used; negative = explode) |
| Spherize | `-implode {amount/100}` (measured, **not** used) |
| Ripple | `-wave {amount/10}x{period} -crop WxH+0+{amount/10} +repage` (measured, **not** used) |
| Wave | `-wave {amp}x{wavelength} -crop WxH+0+{amp} +repage` (measured, **not** used) |
| Polar Coordinates | `-distort DePolar 0` / `-distort Polar 0` (measured, **not** used) |
| Shear | `-shear -{angle}x0 -crop WxH+{round(H·tan(angle)/2)}+0 +repage` (measured, **not** used) |
| ZigZag | `-swirl {angle}` (measured, **not** used) |
| OceanRipple | `-wave {amp}x{wavelength} -crop WxH+0+{amp} +repage` (measured, **not** used) |

### Unsharp Mask scaling

Empirically (16×16 test image, `pictura_filters` with σ = radius/3 against
`-gaussian-blur 0xσ`):

- IM `amount` is a **fraction**: `amount/100` (so Pictura 150 → IM `1.5`).
  Passing the percentage literally (`+150`) diverges by 43.
- IM `threshold` is a **fraction of the quantum range**: `threshold/255` (so
  Pictura level 8 → IM `0.0314`), matching `-threshold`'s percentage convention
  used by the M4 oracle.
- The residual (≤5) is IM's `-unsharp` applying a slightly different internal
  blur than its standalone `-gaussian-blur`; it shrinks to 1 at larger radii
  (Pictura radius 6.0 → σ 2.0). The differential uses radius 3.0 / amount 150 /
  threshold 0 with tolerance 6.

### Mosaic: an exact block mean via two resizes

Pictura's `Mosaic` averages each top-left `n×n` block and writes the rounded mean
back to the block. ImageMagick's box filter computes the same block mean when the
cell divides both dimensions, and a point resize replicates it:
`-filter box -resize {W/n}x{H/n}!` then `-filter point -resize {W}x{H}!`. On the
16×16 test image with cell 4 the result is bit-identical (measured max delta 0).
When the cell does not divide the image, IM's resize window is centred on the
destination samples rather than anchored top-left, so the blocks drift: measured
max delta 80 (cell 6/7), 113 (cell 3) and 136 (cell 5). The differential test
therefore uses cell 4; the module unit test covers the top-left rounding contract
directly.

`Mosaic` is the only M8 filter with a faithful operator. The rest are measured
against the closest plausible ImageMagick operator and classified no-equivalent:

| `Filter` | Tried | Observed max delta |
|---|---|---|
| `Crystallize` | `-kuwahara 4` / `-paint 4` | 118 / 157 |
| `Facet` | `-statistic mean 3x3` / `-kuwahara 1` | 76 / 144 |
| `Fragment` | `-statistic mean 2x2` / `-kuwahara 1` | 85 / 59 |
| `Mezzotint` | `-threshold 50%` / `-ordered-dither o8x8` | 255 / 255 |
| `Pointillize` | `-spread 2` / `-kuwahara 2` | 240 / 232 |
| `ColorHalftone` | `-ordered-dither o8x8` / `h4x4a` | 255 / 255 |

The reasons they cannot match: Crystallize's Voronoi seeds are seeded and
jittered; Facet's banded 3×3 mean is a closed heuristic; Fragment is an
anchored (not centred) sliding 2×2 mean; Mezzotint's screen is a seeded
procedural threshold, not a fixed dither matrix; Pointillize draws a seeded dot
per cell over a background instead of displacing pixels; Color Halftone rotates
a separate screen per channel. Each is guarded by property tests in
`oracle.rs` (`m8_no_equivalent_filters_properties`): seed determinism, flat-field
identity or no-op, binarity/achromatic output, and the source/background color
set.

### M9 Distort: measured, all no-equivalent

The Distort filters are inverse-mapping warps that no ImageMagick operator
reproduces. Each was measured against the closest plausible operator on the
16×16 test image; the deltas are the evidence for the no-equivalent rows:

| `Filter` | Tried | Observed max delta (mean) |
|---|---|---|
| `Twirl` | `-swirl {angle}` (same sign) | 124 (8.0) at +45; 170 (15.5) at +90 |
| `Pinch` | `-implode {amount/100}` | 61 (3.2) at +50; 84 (5.2) at −50 |
| `Spherize` | `-implode {amount/100}` (Normal) | 159 (19.7) at +50; 128 (16.8) at −50 |
| `Ripple` | `-wave {amount/10}x{period}` + crop | 255 (104.4) (100 Medium) |
| `Wave` | `-wave {amp}x{wavelength}` + crop | 255 (100.8) (1 sine, seed 42) |

The mismatches are structural, not just numeric:

- **Twirl** (`-swirl`). Pictura uses a **linear** angular falloff
  `angle·(1 − r/rmax)` over `rmax = max(cx, cy)`; IM `-swirl` uses a smooth
  (radius-squared-ish) falloff over `min(w,h)/2`. The rotation direction
  matches at the same sign (`+angle` ↔ `-swirl +angle`); the best case is
  124 at 45°.
- **Pinch / Spherize** (`-implode`). IM's `-implode amount` is a **fraction**
  (so Pictura 50 → `0.5`, not `50`), and positive implodes while negative
  explodes. Pictura Pinch is a linear radial remap and Spherize an arc-length
  sphere map; neither is IM's implode falloff. Spherize's `HorizontalOnly` /
  `VerticalOnly` modes have no IM analogue at all.
- **Ripple / Wave** (`-wave`). IM `-wave amplitude x wavelength` grows the
  canvas by `2·amplitude` (background-filled) and displaces **one axis** with a
  sine in x; the named `wave` op crops it back to the input size. Pictura
  Ripple displaces **both** axes (`dx ∝ sin y`, `dy ∝ sin x`) with
  clamp-to-edge, and Pictura Wave sums `generators` seeded sine/triangle/square
  generators with random phase/period/amplitude and an axis-wise `scale`.

Contracts are guarded at the `Filter::apply` level by
`m9_no_equivalent_filters_properties` (zero is a bit-exact no-op; non-zero
warps move pixels; Wave is seed-deterministic and seed-sensitive) and in more
detail by the module unit tests in `src/distort/radial.rs` and
`src/distort/undulate.rs`.

### M11 Distort: measured, all no-equivalent

The second Distort batch (`PolarCoordinates`, `Shear`, `ZigZag`, `OceanRipple`)
was measured the same way: each was run against its closest plausible
ImageMagick operator on the 16×16 test image and the observed max/mean deltas
are the evidence for the no-equivalent rows. Verified against ImageMagick
**7.1.2-29 Q16-HDRI**.

| `Filter` | Parameter | Tried | Observed max delta (mean) |
|---|---|---|---|
| `PolarCoordinates` | RectangularToPolar | `-distort Polar 0` | 189 (58.3) |
| `PolarCoordinates` | PolarToRectangular | `-distort DePolar 0` | 194 (58.9) |
| `PolarCoordinates` | crossed directions | `-distort DePolar 0` / `Polar 0` | 240–241 (82.7 / 90.1) |
| `Shear` | `[(-1,-0.5),(1,0.5)]` = 26.565°, RepeatEdgePixels | `-shear -26.565x0 -crop 16x16+4+0 +repage` | 255 (18.4) |
| `Shear` | same, WrapAround | `-shear -26.565x0 -crop 16x16+4+0 +repage` | 255 (23.4) |
| `ZigZag` | amount 80, ridges 5, AroundCenter | `-swirl 50` | 170 (14.3) |
| `ZigZag` | same | `-swirl -80` / `-implode 0.8` | 170 (18.8 / 18.6) |
| `OceanRipple` | size 9, magnitude 20, seed 42 (old sinusoid model) | `-wave 2x8 -crop 16x16+0+2 +repage` | 227 (53.7) |

The mismatches are structural:

- **Polar Coordinates** (`-distort Polar` / `-distort DePolar`). Both IM
  operators are coordinate transforms, but IM's angle origin is about 180°
  from Pictura's `atan2` and its pixel-center/radius anchor and resampling
  differ. `RectangularToPolar` is closest to `-distort Polar` and
  `PolarToRectangular` to `-distort DePolar` (the natural same-name pairing),
  but neither is within a usable tolerance.
- **Shear** (`-shear`). IM `-shear {angle}x0` shears the whole canvas along x
  and grows it by `H·tan(angle)`, filling the exposed strip with the background
  colour (black by default); Pictura shifts each row by a smooth spline through
  the control points, clamps or wraps undefined columns and never changes the canvas. The
  straight curve `[(-1,-0.5),(1,0.5)]` is `atan(0.5) = 26.565°`; even after
  cropping the grown canvas back (`+4+0`, centred on the middle row), the
  background-fill boundary alone pins the max delta at 255.
- **ZigZag** (`-swirl` / `-implode`). IM `-swirl` uses a smooth falloff over
  `min(w,h)/2`; Pictura's cosine radial profile is pinned to zero at the edge
  and reverses `ridges` times. `-implode` is a radial scale, not a ridge
  displacement. No `-swirl` angle matches the profile (best mean 14.3).
- **Ocean Ripple** (`-wave`). IM `-wave` displaces one axis with an unseeded
  sine and pads the canvas; Pictura displaces along the slope of seeded 2-D
  value noise with clamp-to-edge, so it cannot be reproduced by any single
  `-wave` invocation.

Contracts are guarded at the `Filter::apply` level by
`m11_no_equivalent_filters_properties` (polar directions differ; a zero shear
curve is a bit-exact no-op and the fill modes differ; ZigZag amount 0 is a
no-op and the three styles differ; Ocean Ripple magnitude 0 is a no-op and it
is seed-deterministic and seed-sensitive) and in more detail by the module unit
tests in `src/distort/coord.rs` and `src/distort/ripples.rs`.

## Known divergences (IM vs Photoshop / Pictura)

- **Motion blur** is directional in IM and symmetric-uniform in Pictura; see
  above. Observed max delta 86.
- **Radial blur.** IM 7 dropped `-radial-blur`; `-rotational-blur` is a
  convolution with an angle falloff, not Pictura's polar resample + midpoint
  smear. Observed max delta 58.
- **Average** is the exact global mean; `-statistic mean WxH` is a per-pixel
  windowed mean with clamp-to-edge borders, so it is not the same operator
  (observed max delta 92). Guarded by the constant-mean property test.
- **Sharpen family** uses fixed 3×3 high-pass kernels; IM `-sharpen` is a
  Gaussian unsharp (observed max delta 17). Guarded by flat-field identity and
  edge-contrast property tests.
- **Blur / Blur More** use the fixed `[1 2 1]` separable kernel (Blur More = 3
  passes); IM `-blur` is a Gaussian (observed max delta 23 / 14). Guarded by the
  solid-field near-identity property test.
- **Surface Blur** is bilateral; no IM operator (observed max delta 32 vs a
  plain Gaussian). Guarded by the zero-radius identity test.
- **Despeckle** uses fixed spike/band thresholds; IM `-despeckle` is a different
  rank detector (observed max delta 13).
- **Add Noise** is randomized; IM's RNG stream is different and not reproducible
  (observed max delta 79). Guarded by the same-seed determinism test at the
  `Filter::apply` level.
- **High Pass** has no single IM operator. A hand-built `-compose Mathematics`
  difference (source = blur, dest = orig, `compose:args=0,-1,1,0.5`) reproduces
  Pictura's formula and is within measured max delta 1 (radius 3.0, σ 1.0), but
  it re-implements the pipeline rather than being an independent operator, so it
  is classified no-equivalent and guarded by the flat-field mid-gray property
  test.
- **Emboss** is per-channel in IM with a fixed diagonal kernel; Pictura is an
  angle-directed second difference on luma with an achromatic output. Observed
  max delta 210 (angle 135 vs `-emboss 0x1`; best case 186 on the axis-aligned
  angles). Guarded by the flat-field neutral-gray and achromatic-output tests.
- **Find Edges.** IM `-edge` is a different detector with opposite polarity
  (bright edges on dark); Pictura inverts the Sobel magnitude (dark on light).
  Observed max delta 255 vs `-edge 1`. Guarded by the flat-field-light and
  dark-edge property tests.
- **Offset** with `repeat = true` clamps to the nearest edge; `wrap = false`
  fills with `background`. IM `-roll` wraps. Observed max delta 240 for the
  background fill. Both cases are guarded by known-value tests.
- **M8 Pixelate.** `Mosaic` is exact only for cells that divide the image (see
  above). Crystallize, Facet, Fragment, Mezzotint, Pointillize and Color Halftone
  have no faithful IM operator; the closest candidates and measured deltas are in
  the M8 table above.
- **16/32-bit, CMYK/Lab, selection/mask/gpu wiring.** Out of M6/M7 scope.

Tolerances apply only to the differential rows. They are absolute per-8-bit-sample
allowances for `pictura_testkit::compare`: 0 where the formula is identical and
integer arithmetic agrees exactly, 6 for Unsharp Mask where IM's internal blur
can differ. Rows marked **no** keep tolerance 0 — they are not compared to IM.

## Running

All tests run with a plain `cargo test`; the differential ones skip (with a
message) when `magick` is not on `PATH`:

```bash
cargo test -p pictura-filters
```

Nothing is `#[ignore]`d.

Manual inspection:

```bash
python3 scripts/filter_oracle.py version
python3 scripts/filter_oracle.py apply --size 16x16 --planar \
    --op gaussian --sigma 1.0 IN.rgb OUT.rgb
python3 scripts/filter_oracle.py apply --size 16x16 --op box --radius 3 \
    IN.rgb OUT.rgb
python3 scripts/filter_oracle.py apply --size 16x16 --op maximum --radius 2 \
    IN.rgb OUT.rgb
python3 scripts/filter_oracle.py apply --size 16x16 --op roll \
    --horizontal 3 --vertical 2 IN.rgb OUT.rgb
python3 scripts/filter_oracle.py apply --size 16x16 --op convolve \
    --kernel "0,0,-1,0,0, 0,-1,4,-1,0, -1,4,20,4,-1, 0,-1,4,-1,0, 0,0,-1,0,0" \
    --kernel-scale 4 --kernel-offset 8 IN.rgb OUT.rgb
python3 scripts/filter_oracle.py apply --size 16x16 --op solarize \
    --threshold-percent 50 IN.rgb OUT.rgb
python3 scripts/filter_oracle.py apply --size 16x16 --planar \
    --op mosaic --cell 4 IN.rgb OUT.rgb
python3 scripts/filter_oracle.py apply --size 16x16 --planar \
    --im-args="-motion-blur 0x5+45" IN.rgb OUT.rgb
python3 scripts/filter_oracle.py apply --size 16x16 --planar \
    --op swirl --angle 45 IN.rgb OUT.rgb
python3 scripts/filter_oracle.py apply --size 16x16 --planar \
    --op implode --implode-amount 0.5 IN.rgb OUT.rgb
python3 scripts/filter_oracle.py apply --size 16x16 --planar \
    --op wave --wave-amplitude 10 --wave-wavelength 16 IN.rgb OUT.rgb
python3 scripts/filter_oracle.py apply --size 16x16 --planar \
    --op depolar IN.rgb OUT.rgb
python3 scripts/filter_oracle.py apply --size 16x16 --planar \
    --op polar IN.rgb OUT.rgb
python3 scripts/filter_oracle.py apply --size 16x16 --planar \
    --op shear --shear-angle 26.565 IN.rgb OUT.rgb
```

## Regeneration

There are no committed fixtures: the oracle runs ImageMagick at test time and
skips (with a message) when `magick` is not on `PATH`. To reproduce the
reference bytes for a filter by hand:

```bash
# planar 16x16 RGB8 input -> IM result, then compare against apply's output
python3 scripts/filter_oracle.py apply --size 16x16 --planar --op unsharp \
    --sigma 1.0 --amount 150 --threshold 0 in.rgb reference.rgb
```

`MAGICK=/path/to/magick` overrides the binary; `--im-args=...` must be attached
with `=` so argparse does not read the leading `-` as another option.

## Not expressed by this oracle

- Motion Blur, Radial Blur, Average, Blur / Blur More, Surface Blur, Despeckle,
  Add Noise, the Sharpen family, High Pass, Emboss, Find Edges, and `Offset`
  with `wrap = false`: no faithful ImageMagick equivalent (see the table);
  covered by property/known-value tests instead.
- Crystallize, Facet, Fragment, Mezzotint, Pointillize and Color Halftone: no
  faithful ImageMagick equivalent (M8; see the M8 table); covered by
  property/known-value tests instead.
- Twirl, Pinch, Spherize, Ripple and Wave: no faithful ImageMagick equivalent
  (M9; the closest `-swirl` / `-implode` / `-wave` operators were measured — see
  the M9 section); covered by `m9_no_equivalent_filters_properties` and the
  module unit tests instead.
- Polar Coordinates, Shear, ZigZag and Ocean Ripple: no faithful ImageMagick
  equivalent (M11; the closest `-distort Polar`/`DePolar`, `-shear`, `-swirl`
  and `-wave` operators were measured — see the M11 section); covered by
  `m11_no_equivalent_filters_properties` and the module unit tests instead.
- Dust and Scratches, Extrude, Tiles, Trace Contour, Wind and Smart Sharpen: no
  faithful ImageMagick equivalent (see the table); covered by property and
  known-value tests in the module unit tests instead.
- Lens Blur: no depth-aware aperture blur in ImageMagick (see the table);
  covered by the focus, ramp, iris, specular and noise module unit tests
  instead.
- Alpha-channel behaviour: the tests use 3-channel buffers, and `apply` is
  specified never to modify channel 4 (guarded by the module unit tests).
- 16/32-bit filter math (M6–M9 are 8-bit only).
