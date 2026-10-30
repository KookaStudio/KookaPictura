# OpenCV feature mapping

Status: working note, no decision made. Written 2026-09-19.

Where OpenCV could help Kooka Pictura, area by area. This is a map, not a plan.
Nothing here changes a spec, a dependency, or the parity policy in
`docs/00-overview/feasibility-and-non-goals.md`.

## How OpenCV could be used

Three distinct modes, cheapest first:

- **Oracle.** Run OpenCV in the Python test layer as a second implementation and
  diff against our output. No Rust, Cargo, or CMake change. Fits the existing
  oracle design: `scripts/*_oracle.py` subcommands, tests that self-skip when the
  tool is missing, the CI `test` job installs it.
- **Reference.** Read OpenCV's formula or output, then implement the algorithm in
  Rust. No dependency. Use when the CS6 algorithm is closed and OpenCV has the
  standard version.
- **Runtime.** Link OpenCV and call it from Rust. This means a C++ FFI through
  the `opencv` crate, a determinism fight, and a licensing-inventory update. It
  is the last resort, not the first.

## Filters

ImageMagick is already the filter oracle. The filter oracle README marks a long
list as "no faithful operator". Those are the openings.

| Photoshop filter | Spec | Today | OpenCV | Mode | Caveat |
|---|---|---|---|---|---|
| Surface Blur | `06-filters/blur-filters.md` | implemented, no IM oracle | `bilateralFilter` | oracle | The range/space weighting is the whole filter. OpenCV's parameterisation differs from Adobe's, so classify it before trusting a diff. |
| Average | `06-filters/other-filters.md` | implemented, no IM oracle | `boxFilter` | oracle | Global mean; window/border semantics must match. |
| Find Edges | `06-filters/stylize-filters.md` | implemented, no IM oracle | `Sobel`, `Laplacian` | oracle | IM `-edge` is inverted; OpenCV Sobel is the honest check. |
| Sharpen Edges / Sharpen / Sharpen More | `06-filters/sharpen-filters.md` | implemented, no IM oracle | `filter2D` | reference | `filter2D` with our kernel just re-implements the formula. Not independent. |
| Despeckle | `06-filters/noise-filters.md` | implemented, no IM oracle | `medianBlur` | oracle, weak | Different outlier detector; expect a divergence to explain, not a match. |
| Maximum / Minimum | `06-filters/other-filters.md` | implemented, IM faithful | `morphologyEx` | cross-check | IM already covers these; OpenCV only adds a third opinion. |
| Emboss, Motion Blur, Radial Blur | several | implemented, no IM oracle | `filter2D` | reference | Line/directional kernels are ours to define. Not a parity source. |
| Blur, Blur More, High Pass | several | implemented, no IM oracle | `filter2D` | reference | Photoshop's fixed kernels are documented; OpenCV adds nothing independent. |

The real wins are the first four rows: Surface Blur, Average, Find Edges, and a
weak Despeckle check. The rest are already covered or would only restate our own
math.

## Selection and masks

`pictura-select` has one ImageMagick oracle for boolean/mask math. OpenCV covers
a lot of mask geometry that IM does not.

| Feature | Code | OpenCV | Mode | Caveat |
|---|---|---|---|---|
| Selection boundary tracing | `pictura-select/src/contour.rs` | `findContours` | oracle | Our contour is an integer pixel-corner lattice; OpenCV returns pixel centres. Map representations before comparing. |
| Magic Wand | `magic_wand` in `pictura-select/src/lib.rs` | `floodFill`, `connectedComponents` | oracle | Tolerance and connectivity rules must be pinned first. |
| Grow / Similar | `grow`, `similar` | `distanceTransform`, `morphologyEx` | oracle | Distance metric and edge condition are the spec; OpenCV supplies the reference. |
| Color Range | `color_range` | `inRange` | oracle, trivial | Low value; the formula is already explicit. |
| Refine Edge | `08-selection/refine-edge.md` | `grabCut`, `ximgproc.guidedFilter` | reference | CS6 predates the newer matting controls. Use OpenCV to derive, not to ship. |

This is the largest coverage gap in the oracle suite and the best place to
start.

## Image operations

| Feature | Spec | Today | OpenCV | Mode |
|---|---|---|---|---|
| Image Size resize | `04-image-ops/image-size.md` | implemented, IM oracle | `resize` with `INTER_NEAREST/LINEAR/CUBIC/LANCZOS4` | cross-check the interpolation |
| Rotation and flip | `04-image-ops/image-rotation-and-flip.md` | implemented | `rotate`, `flip`, `warpAffine` | cross-check |
| Perspective crop | `03-tools/perspective-crop.md` | not built | `getPerspectiveTransform`, `warpPerspective` | reference |

## Adjustments and histogram

The adjustment oracle already lists Levels, Invert, and Desaturate as faithful.
OpenCV does not beat ImageMagick for most of the rest.

| Adjustment | Code | OpenCV | Mode | Caveat |
|---|---|---|---|---|
| Threshold | `pictura-adjust` | `threshold` | oracle, weak | Photoshop uses Rec.601 luma, OpenCV's grey path differs. |
| Equalize | `pictura-adjust` | `equalizeHist`, `createCLAHE` | reference | CLAHE is not a CS6 control. |
| Channel Mixer | `pictura-adjust` | `transform`, `LUT` | reference | A matrix multiply is deterministic but not more independent than the spec. |
| Auto Tone / Auto Color | `pictura-adjust/src/auto.rs` | Otsu, `createCLAHE` | reference | Adobe's solver is closed; OpenCV gives a starting point, not parity. |
| Curves, Exposure, Hue/Saturation, Vibrance, Photo Filter, Color Balance | `pictura-adjust` | none | none better | No standard equivalent. Leave these to property tests. |

## HDR merge and tone mapping

`docs/04-image-ops/32-bit-hdr.md` already cites OpenCV as the reference for
Debevec and Robertson merge and for Drago, Mantiuk, and Reinhard tone mapping.
This is the strongest documented fit.

- Oracle and reference both. `createMergeDebevec`, `createMergeRobertson`,
  `createCalibrateDebevec`, `createTonemapDrago`, `createTonemapMantiuk`,
  `createTonemapReinhard`.
- Our merge and tone-mapping code does not exist yet, so this is greenfield. The
  oracle is cheap and the reference is defensible.

## Content-Aware and healing

The features people usually mean when they ask for OpenCV. Worth being careful
here.

- `03-tools/content-aware-move-and-patch.md` and
  `03-tools/healing-brushes.md` describe the algorithms. CS6 parity is marked
  behavioral only, algorithm TBD. The spec proposes `pictura-retouch` with
  PatchMatch plus a Poisson or biharmonic seam solve.
- OpenCV offers `inpaint` with Telea or Navier-Stokes, `seamlessClone` in three
  modes, `illuminationChange`, `textureFlattening`, and `colorChange`. These are
  plausible but different algorithms from Adobe's. Running them would give a
  fast approximation, not CS6 parity.

So the honest framing is:

- Content-Aware Fill: `inpaint` is a runtime approximation candidate. The spec's
  PatchMatch plus biharmonic is the parity track.
- Healing Brush: `seamlessClone` is second-order Poisson. Adobe's healing is
  fourth-order biharmonic. Close in spirit, visibly different on gradients.
  Use OpenCV only to sanity-check the solver, or as a fallback.

Neither should sit on the deterministic parity path without an oracle that
proves the result.

## Lens correction, alignment, panorama

| Feature | Spec | OpenCV | Mode | Caveat |
|---|---|---|---|---|
| Lens Correction | `06-filters/lens-correction.md` | `calibrateCamera`, `undistort`, `initUndistortRectifyMap` | reference | The profile format is Adobe's and closed. OpenCV supplies the distortion math, not the profiles. |
| Vanishing Point | `06-filters/vanishing-point.md` | `getPerspectiveTransform` | reference | |
| Auto-Align Layers | `05-layers/align-and-distribute.md` | `phaseCorrelate`, `findHomography`, `ORB` | oracle, reference | IM has no equivalent; a genuine gap. |
| Photomerge | `09-automation` | `Stitcher` | runtime | Likely outside CS6 Standard scope. |

## Color models

`docs/07-color-painting/color-models.md` covers RGB to Lab, HSV, and YCbCr.
`cvtColor` gives a fixed, standard conversion to diff against. ICC work stays
with lcms2, which is the correct tool and already a dependency. OpenCV is not a
color-management replacement.

## Where OpenCV adds nothing

- PSD and PSB. OpenCV reads a flattened PSD with no layer records, masks, or
  groups. Useless for `pictura-codec`.
- The UI. HighGUI loses to the existing Qt shell.
- Color management. lcms2 already handles profiles.

## Costs

- Oracle only: `opencv-python-headless` is about 60 MB on Linux x86_64. Add it to
  the CI `test` job next to ImageMagick and psd-tools. Locally the tests
  self-skip when `cv2` is absent, same as today.
- Runtime: the `opencv` crate, a system OpenCV, and a licensing-inventory update.
  OpenCV is Apache-2.0, so the license is compatible. The build cost is the
  issue: about 118 MB installed for
  the system package, about 1 GB peak for a source build, about 2 GB for a CUDA
  build.
- Determinism: OpenCV uses OpenMP, TBB, and IPP-ICV. `setNumThreads(1)` and
  `setUseOptimized(false)` are required before any golden comparison, and some
  paths stay non-deterministic regardless. This alone is a strong reason to keep
  it out of the runtime.

## Recommendation

Start with the oracle. Add `cv2` to the CI `test` job and wire subcommands for
the places ImageMagick cannot reach:

1. Selection mask geometry: `distanceTransform`, `findContours`,
   `connectedComponents`, `floodFill`.
2. The filters with no faithful ImageMagick operator: Surface Blur via
   `bilateralFilter`, Average via `boxFilter`, Find Edges via `Sobel`.
3. HDR merge and tone mapping, which the spec already points at OpenCV for.

Treat content-aware fill, healing, and Refine Edge as reference work. Only
consider a runtime OpenCV link if we decide to drop CS6 parity for synthesis and
accept the determinism and build cost. That decision belongs in the
feasibility and non-goals doc, not here.

## Open decisions

- Which area gets the first oracle: selection masks, filters, or HDR.
- Whether adding `cv2` to CI is acceptable given the licensing inventory.
- Whether content-aware synthesis targets parity at all, or ships as a marked
  approximation.

## Sources

- Existing oracle tables: `crates/pictura-filters/tests/README.md`,
  `crates/pictura-adjust/tests/README.md`, `crates/pictura-render/tests/README.md`.
- `docs/00-overview/feasibility-and-non-goals.md` for the parity policy.
- `docs/04-image-ops/32-bit-hdr.md` for the OpenCV merge and tonemap reference.
- `docs/03-tools/content-aware-move-and-patch.md`,
  `docs/03-tools/healing-brushes.md` for the synthesis algorithms.
