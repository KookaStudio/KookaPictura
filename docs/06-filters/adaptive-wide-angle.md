# Adaptive Wide Angle

- **Spec ID:** `FILT-094`
- **Status:** `Draft`
- **Parity tier:** `Core` — `Filter > Adaptive Wide Angle` is a CS6 top-level filter in Standard and Extended. It is **GPU-accelerated and the CS6 GPU FAQ lists it as "requires a compatible video card"** (see `## CS6 behavior`).
- **New in CS6:** `Yes` — Adaptive Wide Angle is new in CS6, alongside the lens-profile database shared with Lens Correction and Camera Raw. It adds constraint-driven local perspective correction on top of profile-based geometry. CS6 also adds **Full Spherical** correction for 360° panoramas made in CS6.
- **Depends on:** `FILT-001` filters-overview, `FILT-101` lens-correction (shared lens-profile/EXIF model), `FILT-100` camera-raw-filter (shared distortion model), `ARCH-006` gpu-rendering-pipeline, `ARCH-007` color-management, `ARCH-009` undo-history, `LAY-021` smart-filters, `LAY-020` smart-objects.

> All module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Adobe's projection math and lens-profile format are
> closed; those parts are **behavioral parity only, algorithm TBD**. Facts come
> from the fetched CS6 Help PDF, the fetched CS6-era book extract, and the fetched
> CS6 GPU FAQ mirror unless marked *(inferred)*.

## CS6 behavior

`Filter > Adaptive Wide Angle` corrects the lens distortion introduced by wide-angle lenses and straightens lines that look curved in panoramas or in fish-eye/wide-angle shots (e.g. buildings that lean inward). The filter:

1. **Detects the camera and lens model** from the image's **EXIF lens metadata** and uses the **lens characteristics** (a lens profile) to straighten the image. If profile data matches, the **Correction** section shows **Auto** and some options are hidden/auto-populated.
2. Uses **constraints** — user-drawn lines that indicate where a straight line should be — to drive the correction. Multiple constraints can be added in different parts of the picture; the filter combines them to remove distortion. Constraints are drawn with the **Constraint** tool (a single dragged line that follows the contour) or the **Polygon Constraint** tool (a polygon drawn along an object).
3. Can also be used on images **without** camera/lens info, with extra manual work.
4. May leave **blank areas**; the user crops them, or fills them with **Content-Aware Fill**.

To keep the settings editable later, the layer is converted to a **Smart Object** (`Layers > Smart Objects > Convert to Smart Object`) and the filter applied as a **Smart Filter**.

### Correction modes (CS6, sourced)

- **Fisheye** — corrects extreme curvature from a fisheye lens.
- **Perspective** — corrects converging lines caused by angle of view and camera tilt.
- **Panorama** — corrects a Photomerge panorama; **only applicable to panoramas created in CS6**.
- **Full Spherical** — corrects 360° panoramas created in CS6; the panorama must have a **2:1 aspect ratio**.
- **Auto** — detects the appropriate correction automatically (shown when a matching lens profile is found).

### Constraint interaction (sourced, CS6 + CS6-era book)

- **Constraint tool (`C`)** — drag across a key object; the filter detects the curvature and traces a line along the contour.
- **Polygon Constraint tool (`Y`)** — click a succession of points; a polygon delineates an area to correct. Clicking the four corners of the preview is similar to applying a lens correction to the whole image; useful where there are no straight reference lines (building facades, tiled floors).
- **Shift while dragging** aligns the constraint **vertically or horizontally**. **Right-click** a constraint to choose an orientation from a pop-up menu.
- A selected constraint shows **two handles**; dragging a handle rotates the constraint (an overlay circle and green line appear; the edited constraint then renders green).
- Per the CS6-era book, holding **`S`** while dragging straightens the edge **and** makes it vertical/horizontal; a minimum of two vertical constraints plus a horizontal horizon constraint is recommended; extending constraints edge-to-edge gives a smoother correction. **`x`** temporarily switches to 100% zoom.
- **Constraint colors (sourced):** unfixed constraints **cyan**, horizontals **yellow**, verticals **magenta**, fixed-orientation constraints **green**, invalid constraints (that cannot be calculated) **red**.

### Lens profile database

When the filter opens it searches the **lens profiles database** for a profile matching the image's **EXIF** lens metadata. If a profile is present, Correction shows **Auto**; the profile tells the filter how much a constraint must bend, and the projection is **shape-conformal** (preserving shapes proportional to distance) until constraints tell it to add perspective.

If no profile exists, the CS6-era workflow is: open the **Lens Correction** filter on the same image, click **Search Online** for user-created profiles, select one, and use the **Lens Profiles** fly-out menu → **Save Online Profile Locally**; the profile then becomes available to Adaptive Wide Angle on the next open. This profile database, its search, and the local save mechanism are shared with `FILT-101` lens-correction and `FILT-100` camera-raw-filter.

### GPU requirement

The CS6 GPU FAQ lists **"Adaptive wide-angle filter (requires a compatible video card)"** as a CS6 GPU enhancement, in the same "requires" tier as Oil Paint. The Mercury Graphics Engine uses **OpenGL + OpenCL** (not CUDA). Treat the GPU as a hard CS6 requirement for this filter; see `## Open questions` for the fallback policy.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Filter > Adaptive Wide Angle` | menu, dialog | — | Top-level CS6 filter |
| Dialog — Correction | combo | `Ctrl+T` | Auto / Fisheye / Perspective / Panorama / Full Spherical |
| Dialog — Scale | slider/field | `Ctrl+S` | Scales the image to minimise blank areas |
| Dialog — Focal Length | field | `Ctrl+F` | Auto-populated from lens info |
| Dialog — Crop Factor | field | `Ctrl+R` | Combined with Scale to crop blank areas |
| Dialog — As Shot | checkbox | `Ctrl+A` | Use lens-profile values; disabled if no lens info |
| Dialog — Preview | checkbox | `Ctrl+P` | Toggle preview |
| Dialog — Show Constraint | toggle | `Ctrl+W` | Show/hide constraint lines |
| Dialog — Show Mesh | toggle | `Ctrl+E` | Show/hide the warp mesh |
| Dialog — Constraint tool | tool | `C` | Drag a single line |
| Dialog — Polygon Constraint tool | tool | `Y` | Click a polygon |
| Dialog — Move tool | tool | `M` | Move the view/constraints |
| Dialog — Hand tool | tool | `H` | Pan |
| Dialog — Zoom tool | tool | `Z` | Zoom |
| Dialog — transparent matte | hidden toggle | `L` | Toggle transparent matte |
| Dialog — temporary zoom | hidden toggle | `X` | Temporary zoom |
| Dialog — revert polygon corner | hidden | `E` | Revert the last-added polygon corner |
| Constraint orientation | context menu | right-click | Choose orientation for an existing line |
| Constraint align H/V | modifier | `Shift`-drag | Align vertically or horizontally |
| Constraint rotate / straighten | on-canvas | drag handles / `S`-drag | Rotate; `S` also snaps to H/V |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Correction | enum | Auto (when profile found) | Auto / Fisheye / Perspective / Panorama / Full Spherical | Panorama/Full Spherical only for CS6-made panoramas; Full Spherical needs 2:1 |
| Scale | float % | *(unverified)* | *(unverified)* | Reduces blank areas introduced by correction |
| Focal Length | float mm | auto from lens info | *(unverified)* | Auto-populated when lens data is detected |
| Crop Factor | float | *(unverified)* | *(unverified)* | Combined with Scale to crop blank areas |
| As Shot | bool | *(unverified)* | on / off | Disabled when no lens info is found |
| Constraint type | enum | Constraint | Constraint / Polygon Constraint | Single line vs polygon |
| Constraint orientation | enum | unfixed | unfixed / horizontal / vertical / fixed angle | Colors: cyan / yellow / magenta / green |
| Constraint angle | float ° | *(unverified)* | 0–360 | Edited via handle rotation or context menu |

## Algorithms & pipeline

Behavioral parity only; Adobe's projection and constraint solver are closed. The CS6-era book gives a useful conceptual model *(secondary)*:

- With a matching lens profile, the filter computes a **shape-conformal projection** that preserves shapes proportional to distance from the viewer (rather than a perspective-accurate projection). The lens profile knows how every edge should bend.
- Each **constraint** selectively **overrides** the shape-conformal projection locally, telling the filter to apply more **perspective** in that region. As constraints are added, parts of the image compress and others stretch, which is what **Scale** compensates for.
- A constraint is drawn through an image edge; the filter already knows (from the profile) how much to bend it, so the drawn line snaps to the correct corrected geometry.

An implementation model *(inferred)*:

```text
profile = match_lens_profile(exif)            # shared with Lens Correction
if profile: base = shape_conformal_warp(image, profile)
else:       base = identity                  # manual mode

for each constraint c (polyline + target orientation):
    # solve a per-region perspective/affine correction that maps c to a straight
    # line at its target orientation, minimizing distortion elsewhere
mesh += solve_constraint(base, c, neighbors)

out = resample(image, mesh) * scale(scale_value)
```

The solver is a constraint-driven mesh warp: each constraint is a hard/soft positional constraint on the warp mesh, and the mesh is interpolated smoothly between constraints. Constraint colors encode each constraint's solved state (unfixed/horizontal/vertical/fixed/invalid). Because the profile and solver are closed, this is **behavioral parity only, algorithm TBD**.

**GPU.** Per the CS6 GPU FAQ, the filter requires a compatible video card and runs on the Mercury Graphics Engine (OpenGL + OpenCL). Bit-depth/mode support is not stated in the fetched CS6 source; the CS6 16-/32-bpc filter lists **exclude** Adaptive Wide Angle (see `## Edge cases`).

## Rust module mapping

Proposals. Adaptive Wide Angle shares the lens-profile and distortion model with Lens Correction and Camera Raw, so it is a new solver/projection module over the same profile data.

- `pictura_filters::adaptive_wide_angle` — `AwaParams { correction: Correction, scale: f32, focal_length: Option<f32>, crop_factor: Option<f32>, as_shot: bool, constraints: Vec<Constraint> }`; `fn apply(&Document, &AwaParams, &mut Backend) -> FilterResult`.
- `pictura_filters::lens_profile` — shared (with `FILT-101`/`FILT-100`): `LensProfileDb`, `fn match_from_exif(&Exif) -> Option<LensProfile>`, online-profile import, local save.
- `pictura_filters::awa::projection` — `enum Correction { Auto, Fisheye, Perspective, Panorama, FullSpherical }`; `fn shape_conformal(profile, size) -> WarpMesh`, `fn perspective(...)`.
- `pictura_filters::awa::constraints` — `struct Constraint { polyline: Vec<Vec2f>, orientation: ConstraintOrientation }`, `enum ConstraintOrientation { Unfixed, Horizontal, Vertical, Fixed(f32) }`, `fn solve(mesh: &mut WarpMesh, constraints, profile)`.
- `pictura_render::warp` — shared mesh-resample compute pass (reused by Liquify); CPU reference path.

Crossing types: `AwaParams`, `Constraint`, `LensProfile`, `WarpMesh`, `FilterResult`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `AdaptiveWideAngleDialog` | `QDialog` | Correction/Scale/Focal Length/Crop Factor/As Shot controls; Preview/Show Constraint/Show Mesh toggles; OK/Cancel |
| `AwaCanvas` | `QRhiWidget` (preview) + `QGraphicsView` (overlay) | Constraint drawing/editing, handle rotation, zoom/hand/move, live warp preview |
| `ConstraintModel` | `QAbstractListModel` | Constraint list with type, orientation, validity (drives the color coding) |
| `ConstraintOrientationMenu` | `QMenu` | Right-click orientation picker |
| `LensProfileBanner` | widget | Shows Auto/detected lens or the "no profile found" affordance (with the Lens Correction Search Online path) |

Widgets + a `QGraphicsView` overlay (rather than QML) keeps this consistent with the other modal filter dialogs and reuses the mesh-warp preview component from Liquify (`ARCH-003`).

## Data-model impact

- **Destructive by default.** One history state per OK; `Cancel` is a no-op.
- **Smart filter:** the CS6 Help explicitly recommends converting the layer to a Smart Object so the filter settings can be edited later. Store `AwaParams` (including the constraint list) in the filter record's `params_blob` (`LAY-021`).
- **Lens profiles** are application/session data read from a shared profile database; they are **not** per-document state (except an optional embedded profile reference in EXIF).
- **No new document nodes.**
- **Serialization:** mapping constraints to a PSD/XMP filter descriptor is not documented (see `## Open questions`).

## Edge cases

- **No lens/EXIF profile** — the filter still works manually (constraints only); `As Shot` is disabled and Correction defaults away from Auto. This is a first-class path, not an error.
- **Missing profile but one available online** — surface the Lens Correction `Search Online` → `Save Online Profile Locally` workflow.
- **16-/32-bpc and mode support** — the CS6 16- and 32-bpc filter lists **exclude** Adaptive Wide Angle, implying **8-bpc-only**; confirm. Bitmap/Indexed are excluded for all filters.
- **Panorama corrections** — `Panorama` only for CS6-made Photomerge panoramas; `Full Spherical` requires a **2:1** aspect ratio. Reject/disable otherwise.
- **Blank areas after correction** — expected; provide Scale/Crop Factor and document the Content-Aware Fill follow-up.
- **Invalid constraint** — renders red; the solver must not crash and must exclude it from the warp.
- **Constraint edge-to-edge** — extending constraints to the frame improves the correction; no crash when they overhang.
- **GPU unavailable** — CS6 requires a compatible card; decide the Linux fallback (CPU mesh warp vs disable). The shared `pictura_render::warp` CPU reference makes a fallback feasible.
- **Tilt-shift captures** — the CS6-era book notes such images may still benefit, but only with an accurate profile and **Auto** mode.
- **Huge (PSB) documents** — mesh-based, so memory is bounded by the lattice; tile the resample.
- **Undo/redo** — one atomic state per OK.

## Parity acceptance criteria

1. Given an image with a matching lens profile, opening the filter selects `Auto`, populates Focal Length, and applies an initial correction that visibly straightens profile-correctable curvature.
2. Given a fisheye test grid, `Correction = Fisheye` reduces line curvature relative to no correction; `Perspective` corrects converging verticals; adding two vertical constraints plus a horizontal constraint straightens the expected lines (per the CS6-era book's recommended workflow).
3. Given `Shift`-dragging a constraint, it aligns exactly horizontally or vertically; its color changes to the documented horizontal/vertical color; right-click orientation changes it.
4. Given a polygon constraint around four corners, the result approximates a whole-image lens correction.
5. Given `Full Spherical` on a non-2:1 panorama, the mode is rejected/disabled; on a 2:1 CS6-made 360° panorama it applies.
6. Given `As Shot` enabled with no lens info, the control is disabled.
7. Given `Scale`/`Crop Factor`, blank areas introduced by the correction shrink accordingly.
8. Given an invalid constraint (red), it does not affect the output and no error occurs.
9. Given a Smart Object, the filter is re-editable from the Smart Filters entry with its constraint list restored.
10. Given a commit, exactly one history state appears and `Ctrl+Z` restores the pre-filter pixels bit-exactly.
11. Given no compatible GPU, the chosen fallback (CPU mesh warp or disabled filter) is deterministic and documented.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — official CS6 Help reference (downloaded to `/tmp`, `pdftotext`-extracted). Established: the Adaptive Wide Angle overview (camera/lens detection, lens characteristics, multiple constraints, use without lens data, Smart Object conversion); Correction modes (Fisheye, Perspective, Panorama restricted to panoramas created in CS6, Full Spherical for 360-degree panoramas created in CS6 at a 2:1 aspect ratio, Auto); Scale, Focal Length, Crop Factor, As Shot; the Constraint and Polygon Constraint tools; the constraint tool tracing a line along the detected contour; `Shift` to align vertically/horizontally and the right-click orientation pop-up; the blank-area/Content-Aware Fill note; the CS6 new-feature shortcut lists (Constraint `C`, Polygon `Y`, Move `M`, Hand `H`, Zoom `Z`; Control `P/W/E/T/S/F/R/A`; hidden `L/X/E`); the 16-/32-bpc lists exclude it.
- `http://www.photoshopforphotographers.com/pscs6/downloads/Adaptivewideangle.pdf` — Martin Evening, *Adobe Photoshop CS6 for Photographers* free chapter extract (CS6-era book). Established: the lens-profile-database/EXIF search and the Auto display; the **shape-conformal** vs perspective-accurate projection model; constraints override the shape-conformal projection to add perspective; Scale compensates; constraint rotation with handles and the green fixed state; the recommended two-vertical + one-horizontal workflow and `S`-drag (straighten + snap) / `x` (100% zoom); the constraint **color** legend (cyan unfixed, yellow horizontal, magenta vertical, green fixed, red invalid); the Polygon tool's four-corner ≈ whole-image correction; tilt-shift guidance; the missing-profile → Lens Correction Search Online → Save Online Profile Locally workflow. Secondary (book).
- `https://topic.alibabacloud.com/a/photoshop-cs6-gpu-faq_8_8_10184243.html` — mirror of the **Photoshop CS6 GPU FAQ**. Established: Adaptive Wide Angle **requires a compatible video card**; Mercury Graphics Engine uses OpenGL + OpenCL (not CUDA).
- `https://helpx.adobe.com/photoshop/using/adaptive-wide-angle-filter.html` — modern Adobe Help page (live fetch 403; surfaced via search). Corroborates the CS6 parameter set and the lens-profile detection; not used for CS6-specific claims.
- SearXNG meta-search (queries: "Photoshop CS6 Adaptive Wide Angle lens profile database camera lens detection", "Photoshop CS6 GPU FAQ") — used to locate the above; no facts asserted from snippets alone.

Not parsed in this pass: `helpx.adobe.com` live pages (HTTP 403); the proprietary lens-profile format and solver.

## Open questions

- **Scale / Focal Length / Crop Factor ranges and defaults.** Not published in the fetched CS6 source. Resolves with: a CS6 dialog capture.
- **Exact 8-bpc-only confirmation and mode support** (RGB-only?). Inferred from the depth lists; resolves with: a CS6 test.
- **Constraint solver math.** The projection and constraint solve are closed. A constraint-driven smooth mesh warp is *(inferred)*. Resolves with: reference-image fitting or an acceptance tolerance.
- **Blank-area default handling** — CS6 leaves blanks; whether `Auto Scale`/`Scale` defaults already minimise them is unverified. Resolves with: a CS6 capture.
- **GPU fallback policy.** CS6 "requires a compatible video card"; a Linux build needs a CPU path or must disable the filter. Resolve with `ARCH-006`.
- **Smart-filter serialization.** Adobe's PSD filter-descriptor keys for the correction/constraint set are unknown. Resolves with: a PSD containing an Adaptive Wide Angle smart filter, parsed.
- **Lens-profile database licensing/format.** Sharing the profile DB with Lens Correction and Camera Raw has format and licensing implications (`OVR-004` independent-creation). Resolves with: a profile-format decision.
