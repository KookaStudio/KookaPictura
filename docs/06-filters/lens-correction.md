# Lens Correction

- **Spec ID:** `FILT-101`
- **Status:** `Draft`
- **Parity tier:** `Core`.
- **New in CS6:** `No` / `Unchanged` — profile-based automatic correction was introduced in **CS5** (the CS5 What's-New section lists "Automated lens correction"); the CS6 What's-New section does not itemize the Lens Correction filter. CS6 documents the same **Auto Correction** and **Custom** tabs. Exact CS5→CS6 deltas are unconfirmed — see `## Open questions`.
- **Depends on:** `06-filters/camera-raw-filter.md` (shared profile/transform model), `01-architecture/color-management.md`, `01-architecture/file-formats.md`, `01-architecture/document-model.md`, `01-architecture/undo-history.md`, `04-image-ops/image-modes.md`, `04-image-ops/bit-depth-and-conversion.md`, `03-tools/crop-tool.md`

> All crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Behavior is taken from the fetched CS6 Help PDF unless
> marked *(inferred)*. The lens-profile math and Adobe's profile format are
> closed; those parts are **behavioral parity only, algorithm TBD**.

## CS6 behavior

`Filter > Lens Correction` (menu path) fixes common lens flaws — barrel/pincushion **distortion**, **vignetting**, and **chromatic aberration** — and can also rotate an image or fix perspective caused by vertical/horizontal camera tilt. **The filter works only with 8- and 16-bit-per-channel images in RGB or Grayscale mode** (PDF). The dialog shows the image with an optional alignment grid and an editable preview.

**Auto Correction (default tab).** Uses **lens profiles** plus **Exif** metadata identifying the camera and lens to correct automatically:
- `Correction` — the set of problems to fix; if corrections would extend or contract the image beyond its original dimensions, `Auto Scale Image` compensates.
- `Edge` — how to handle blank areas caused by pincushion/rotation/perspective correction: **transparency**, a **color**, or **extend the edge pixels**. CS6 captures show the menu as `Edge Extension`, `Transparency`, `Black Color`, `White Color` — black or white, not an arbitrary colour.
- `Search Criteria` — filters the `Lens Profiles` list; default sorts by sensor-size match, and the pop-up offers `Prefer RAW Profiles`.
- `Lens Profiles` — by default only profiles matching the current camera/lens are shown (camera need not match exactly). Photoshop auto-selects a matching **sub-profile** by focal length, f-stop, and focus distance; right-click the current profile to pick a different sub-profile. `Search Online` acquires community profiles; `Save Online Profile Locally` stores them. `Set Lens Default` saves the current settings keyed to camera/lens/focal length/f-stop.

**Custom tab.** Manual correction, usable alone or to refine automatic correction:
- `Settings` menu — `Lens Default` (saved per camera/lens/focal length/f-stop), `Previous Conversion` (last used), and any user-saved settings. CS6 captures show it opening on `Default Correction` and reading `Custom` once a slider moves.
- `Remove Distortion` slider + `Remove Distortion` tool — barrel/pincushion. (Drag toward center = barrel, toward edge = pincushion.)
- `Fix Fringe` — `Fix Red/Cyan Fringe` and `Fix Blue/Yellow Fringe` scale one channel relative to green. Zoom in; hold `Alt`/`Option` while dragging to isolate the channel. CS6 captures also show a third slider, `Fix Green/Magenta Fringe`, between the two.
- `Vignette` — `Amount` (lighten/darken edges) and `Midpoint` (width of the affected area).
- `Transform` — `Vertical Perspective` (corrects up/down camera tilt), `Horizontal Perspective` (left/right tilt), `Angle` (corrects camera tilt; also reachable via the `Straighten` tool by dragging along a line to make horizontal/vertical), and `Scale` (zooms to remove blank areas; pixel dimensions unchanged, so scaling up crops + interpolates back up).
- `Grid` — `Show Grid`, `Size`, `Color`, and a `Move Grid` tool to align the grid to the image.

**Layout (CS6 captures).** A tool column on the left (`Remove Distortion`, `Straighten`, `Move Grid`, then `Hand` and `Zoom`) under a help line that names the active tool ("Drag towards or away from center to correct distortion.") or the control under the pointer; the preview with a zoom box; `Camera Model`, `Lens Model` and `Camera Settings` (`--mm, f/--, --m` when unknown) status lines; `Preview`, `Show Grid`, grid `Color` and `Size`; and on the right `OK` / `Cancel` over the `Auto Correction` and `Custom` tabs. The title reads `Lens Correction (100%)`. With no matching profile the list reads "No matching profiles found" and the `Correction` checkboxes are disabled. The Transform group's `Angle` has a dial that points up at 0° and turns clockwise with the value (0–360°, two decimals).

**Saving:** `Save Settings` / load from the Settings menu. Auto Correction settings and Custom distortion/CA/vignette settings are saved; **perspective settings are not saved** because they vary per image. A `Lens Default` becomes available when EXIF has camera, lens, focal length, and f-stop.

> **Correction:** CS6's Lens Correction has **no separate X/Y offset** control — the transform set is Vertical Perspective, Horizontal Perspective, Angle, and Scale only. Post-CS6 ACR/CC added `Offset X/Y` and other transform controls. Do not treat offset as CS6 parity.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Filter > Lens Correction` | Menu | `Shift+Ctrl+R` | 8-/16-bit RGB or Grayscale only |
| Dialog — `Auto Correction` tab | Tab | — | Default; profile-based |
| Dialog — `Custom` tab | Tab | — | Manual correction |
| Dialog — `Settings` menu | Menu | — | Lens Default, Previous Conversion, saved settings |
| Dialog — `Show Grid` / `Size` / `Color` | Controls | — | Grid display |
| Dialog — `Move Grid` tool | Tool | — | Align grid to image |
| Dialog — `Straighten` tool | Tool | — | Sets `Angle` from a dragged line |
| Dialog — `Remove Distortion` tool | Tool | — | Drags `Remove Distortion` |
| Dialog — Zoom / Hand | Tools | — | Preview navigation |
| Dialog — `Set Lens Default` | Button | — | Requires EXIF |
| Dialog — `Save Settings` | Menu item | — | Persists custom/auto settings |
| Dialog — `Search Online` | Button | — | Downloads community profiles |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Correction — Geometric Distortion | check | on (when profile) | on/off | Auto tab |
| Correction — Chromatic Aberration | check | off (CS6 capture) | on/off | Auto tab |
| Correction — Vignetting | check | off (CS6 capture) | on/off | Auto tab |
| Auto Scale Image | bool | on (CS6 capture) | on/off | Compensates extended/contracted result; captures show a Custom rotation still leaving blank corners with it on |
| Edge | enum | Transparency (CS6 capture) | Edge Extension / Transparency / Black Color / White Color | Fill for blank areas |
| Search Criteria | enum | sensor-size first | sensor-size / Prefer RAW Profiles | Filters profile list |
| Lens Profile | list selection | auto-match | installed profiles | right-click for sub-profile |
| Remove Distortion | slider | 0 | approx −100…+100 *(community)*, two decimals (CS6 capture) | barrel/pincushion |
| Fix Red/Cyan Fringe | slider | 0 | approx −100…+100 *(community)*, two decimals (CS6 capture) | channel scale |
| Fix Green/Magenta Fringe | slider | 0 | approx −100…+100 *(community)*, two decimals (CS6 capture) | channel scale |
| Fix Blue/Yellow Fringe | slider | 0 | approx −100…+100 *(community)*, two decimals (CS6 capture) | channel scale |
| Vignette Amount | slider | 0 | approx −100…+100 *(community)* | + lightens edges |
| Vignette Midpoint | slider | +50 (CS6 capture) | 0…100 | lower = wider |
| Vertical Perspective | slider | 0 | approx −100…+100 *(community)* | parallel verticals |
| Horizontal Perspective | slider | 0 | approx −100…+100 *(community)* | parallel horizontals |
| Angle | dial + field | 0 | 0…360°, two decimals (CS6 capture) | rotate/straighten |
| Scale | slider | 100 | approx 1…200 *(community)* | px dimensions unchanged |
| Grid Size | field | 64 (CS6 capture) | preview units | grid |
| Grid Color | color | grey *(unverified)* | any color | grid |

Numeric ranges absent from the CS6 Help PDF are marked *(community/unverified)* and listed under `## Open questions`.

## Algorithms & pipeline

*(inferred / behavioral parity only, algorithm TBD unless noted.)*

1. **Profile selection** — match EXIF `Make`/`Model`/`LensModel`/focal length/f-stop/focus distance against the profile database; pick the best profile and a sub-profile. Camera match need not be exact (PDF).
2. **Geometric distortion** — a profile defines a radial distortion model (commonly a polynomial in normalized radius, e.g. Brown–Conrady `r_d = r (1 + k1 r² + k2 r⁴ + k3 r⁶)`); correction resamples with the inverse mapping. Adobe's exact coefficients/format are closed. *(inferred)*
3. **Chromatic aberration** — lateral CA is corrected by scaling the red and blue channels relative to green (`Fix Red/Cyan`, `Fix Blue/Yellow`), optionally combined with a profile model. *(inferred)*
4. **Vignetting** — a radial falloff correction with `Amount` and `Midpoint`; profile-driven for auto, manual otherwise. *(inferred)*
5. **Transform** — perspective correction is a projective homography derived from `Vertical`/`Horizontal`/`Angle`, followed by `Scale`. `Straighten` derives `Angle` from a user-drawn line. *(inferred)*
6. **Edge handling** — after transform, blank regions are filled by transparency, a solid color, or edge-pixel extension; `Auto Scale Image` instead zooms so the crop boundary contains valid pixels.
7. **Resampling** — a single high-quality resample (zoom/rotate/project) with appropriate filtering; Adobe's kernel is closed. The operation is applied to the layer (destructive) or as a Smart Filter when on a Smart Object.

For `FILT-100` reuse: the same profile store and transform code power ACR's Lens Corrections tab (Profile/Manual/Color), so this module should be factored once and shared.

## Rust module mapping

- `pictura_lens::LensProfile` — model id, mount/camera match keys, focal/f-stop/focus sub-profile selector, distortion polynomial, CA model, vignette model. `Serialize`.
- `pictura_lens::ProfileStore` — load/index Adobe and community profiles; lookup by EXIF; covers "Search Online"/"Save Online Profile Locally".
- `pictura_lens::ExifMatch` — camera/lens/focal/f-stop/focus matching.
- `pictura_lens::distortion` — apply/invert radial distortion.
- `pictura_lens::ca` — channel scale + fringe correction.
- `pictura_lens::vignette` — amount/midpoint falloff correction.
- `pictura_lens::transform::Homography` — vertical/horizontal/angle/scale → projective matrix; `straighten(line) -> angle`.
- `pictura_lens::edge::{Transparent, SolidColor, Extend}` — blank-area fill.
- `pictura_lens::LensCorrectionParams` — full dialog state; saved-settings and lens-default records.
- `pictura_filter::lens_correction` — the `Filter` implementation + `supported(mode, depth)` gate (8/16-bit RGB/Grayscale).

Crossing types: `Mode`, `BitDepth`, `LensProfileId`, `LensCorrectionParams`, `Homography`, `TileStore`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `LensCorrectionDialog` | `QDialog` | Preview + Auto Correction / Custom tabs + Settings menu |
| `LensProfileListModel` | `QAbstractItemModel` | Profile list, search criteria, sub-profile selection |
| `LensPreview` | `QRhiWidget` | GPU preview with grid and live correction |
| `GridOverlay` | `QWidget`/`QQuickItem` | Show/size/color/move grid; resampling preview |
| `StraightenTool` / `DistortionTool` | interaction controllers | Map drag to `Angle` / `Remove Distortion` |
| `LensTransformPanel` | `QWidget` | Vertical/Horizontal/Angle/Scale sliders |
| `LensCaPanel` | `QWidget` | Fix Red/Cyan, Fix Blue/Yellow |
| `LensVignettePanel` | `QWidget` | Amount, Midpoint |
| `SaveSettingsMenu` | `QMenu` | Save/load, Set Lens Default (EXIF-gated) |

Widgets over QML for the panels (dense, keyboard-navigable); QML/QtQuick for the preview overlay and grid. Shared `LensTransformPanel` with `FILT-100`.

## Data-model impact

- As a plain filter: writes a `LensCorrectionParams` record; destructive on a raster layer, or a Smart Filter entry on a Smart Object (`LAY-021`).
- **Saved settings** live in a filter-settings store; **lens defaults** are keyed by `(camera, lens, focal length, f-stop)` in preferences (`preference-storage.md`). Perspective settings are deliberately not persisted (PDF).
- Undo: one history step for the whole filter application; within the dialog, each parameter change is a cheap re-render, not a history entry.
- `LensProfile` data is external (installation/community download), not embedded in the document; a document referencing a profile that is not installed must degrade to "profile unavailable" and keep the settings.
- No PSD key change for the filter itself; `Filter Effects`/Smart Filter serialization is owned by `LAY-021`.

## Edge cases

- **Mode/depth gate** — CMYK, Lab, Multichannel, Bitmap/Indexed, and 32-bit documents are unsupported (PDF: RGB/Grayscale, 8/16-bit). The Filter menu must gate and/or warn; Smart Filter path shows the warning icon.
- **Missing EXIF** — Auto Correction and `Set Lens Default` unavailable; the Custom tab must still work.
- **No matching profile** — fall back to manual; offer `Search Online`; never silently apply a wrong profile.
- **Online search offline** — must fail gracefully and keep local profiles usable.
- **Blank edges** — the `Edge` fill and `Auto Scale Image` must agree: auto-scale implies no blank area; otherwise use the selected Edge mode.
- **Scale + crop** — scaling up interpolates to original pixel dimensions; a subsequent crop must reflect the scaled result. Must not double-apply.
- **Straighten → Angle** — the derived angle must be stable and reversible.
- **Very wide/chromatic images** — heavy fringing may need channel-specific correction; guard against channel misalignment producing gray fringes.
- **Profile precision** — applying a profile twice must not compound (idempotence); reapplying the same settings must be a no-op.
- **Performance** — a full-resolution homography+vignette on a PSB must be tiled; preview at screen resolution.
- **GPU unavailable** — CPU fallback; cache key must include backend.
- **Undo/redo** — cancelling the dialog discards all changes; OK commits one step.
- **Interop** — a CS6-authored Smart Filter lens-correction record must round-trip; unknown fields preserved byte-for-byte.

## Parity acceptance criteria

- Given an 8-bit RGB image with matching EXIF and an installed profile, `Auto Correction` reduces measured barrel/pincushion distortion versus the uncorrected render, with `Amount` 0/100/200 scaling monotonically.
- Given `Edge = Transparency`, corrected blank areas are transparent; `Edge = Color` fills with the chosen color; `Edge Extension` replicates edge pixels.
- Given `Auto Scale Image`, the result has no blank area and fits the original pixel dimensions.
- Given a `Fix Red/Cyan Fringe` sweep, the red channel scales relative to green; at `Alt`-drag only the relevant fringe is previewed.
- Given a vertical-tilt photo, increasing `Vertical Perspective` makes vertical lines parallel within a defined angular tolerance.
- Given `Straighten` dragged along a known horizontal feature, `Angle` equals the feature's deviation and the feature becomes horizontal/vertical.
- Given an image with camera/lens/focal/f-stop EXIF, `Set Lens Default` makes `Lens Default` available for the next matching image; without EXIF the control is disabled.
- Given a CMYK or 32-bit document, the filter is unavailable/gated and the Smart Filter warning icon appears.
- Given `Save Settings`, the settings reappear in the Settings menu; perspective values are not restored (per the PDF).
- Given a profile that is not installed, the dialog reports "profile unavailable" and retains manual controls.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — CS6 Help corpus (downloaded, `pdftotext -layout`). Established: `Filter > Lens Correction`; barrel/pincushion/vignetting/CA definitions; **8- and 16-bit RGB or Grayscale only**; Auto Correction tab (`Correction`, `Auto Scale Image`, `Edge`, `Search Criteria`/`Prefer RAW Profiles`, `Lens Profiles` + sub-profile by focal length/f-stop/focus distance, `Search Online`, `Save Online Profile Locally`); Custom tab (`Settings`, `Remove Distortion` + tool, `Fix Red/Cyan` / `Fix Blue/Yellow` fringe, `Vignette Amount`/`Midpoint`, `Vertical`/`Horizontal Perspective`, `Angle` + `Straighten` tool, `Scale`); grid controls (`Show Grid`, `Size`, `Color`, `Move Grid` tool); saving settings and `Set Lens Default` (EXIF-gated; perspective not saved); the CS5 What's-New "Automated lens correction" attribution.
- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` (Camera Raw chapter, same fetch) — established the shared ACR Lens Corrections model (Profile/Manual/Color) reused here, and ACR 7.1 chromatic-aberration checkboxes/sliders.

- CS6 captures of the Lens Correction dialog supplied by the project owner (#347) — established the third `Fix Green/Magenta Fringe` slider, the `Edge` menu (`Edge Extension`, `Transparency`, `Black Color`, `White Color`), the opening defaults (`Transparency`, `Auto Scale Image` on, only `Geometric Distortion` checked, Midpoint +50, Grid Size 64, `Default Correction`), two-decimal distortion/fringe/angle fields, the Angle dial, the tool column and help line, the status lines, and "No matching profiles found".

Marked inferred/community (not primary): the distortion polynomial form, CA and vignette models, resampling kernel, resampling parameters, default values flagged *(unverified)*, and numeric slider bounds not in the PDF.

## Open questions

- **CS5 vs CS6 delta.** Did anything in the Lens Correction filter change in CS6 (profile handling, UI, sub-profile matching)? The CS6 What's-New does not itemize it. Resolve with a CS5 vs CS6 Help diff or build screenshots.
- **Profile format.** Which on-disk profile format and distortion/CA/vignette model does Adobe use? Resolve by inspecting installed profiles and the Adobe Lens Profile Creator output; may need a compatible open parser.
- **Exact slider ranges/defaults.** Not tabulated in the PDF; every `(community)` value needs verification against ACR/Lens-Correction UI metadata.
- **Default `Edge` and `Auto Scale Image` states.** Resolved by CS6 captures: `Transparency` and on.
- **Sub-profile selection rule.** The exact interpolation/tie-break across focal length, f-stop, and focus distance is closed. Resolve with test shots.
- **Offset control.** Confirm CS6 truly lacks X/Y offset (post-CS6 addition), so the UI does not expose a non-parity control.
- **Idempotence.** Confirm that re-applying profile correction to an already-corrected image is a no-op (needed for acceptance).
- **Online profile source.** `Search Online` relied on an Adobe-hosted repository; a Linux-native equivalent needs a decision (community mirror, bundled set, or disable).
- **Reuse with `FILT-100`.** Confirm the shared `pictura_lens` design before implementing both.
