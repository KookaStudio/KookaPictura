# Camera Raw Filter (ACR 7)

- **Spec ID:** `FILT-100`
- **Status:** `Draft`
- **Parity tier:** `Core` for the **ACR 7 raw engine** (opening camera raw files, per-image metadata edits, open-as-Smart-Object). `Non-goal (post-CS6)` for the `Filter > Camera Raw Filter` menu command — that command is **not** in CS6.
- **New in CS6:** `Changed` — CS6 shipped **Camera Raw 7.0** with **Process Version 2012** (PV2012): rewritten Basic tone controls (`Highlights`, `Shadows`, `Whites`, `Blacks`), new local corrections (white balance, highlights, shadows, noise, moiré), camera profiles, and the `Snapshots` tab. **Correction:** the title premise is inaccurate — `Filter > Camera Raw Filter` was introduced in **Photoshop CC (v14, 2013)**, not CS6. CS6 exposes ACR only as the raw-file processing engine / open-as-Smart-Object path. See `## Open questions`.
- **Depends on:** `ARCH-006` gpu-rendering-pipeline, `ARCH-008` document-model, `ARCH-009` undo-history, `01-architecture/color-management.md`, `01-architecture/file-formats.md`, `LAY-020` smart-objects, `LAY-021` smart-filters, `10-workflow-io/camera-raw-workflow.md`, `04-image-ops/32-bit-hdr.md`, `04-image-ops/image-modes.md`, `06-filters/lens-correction.md`

> All crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Behavior is taken from the fetched CS6 Help PDF unless
> marked *(inferred)*. Adobe's ACR implementation is closed; internal algorithm
> claims are marked **behavioral parity only, algorithm TBD**.

## CS6 behavior

Adobe Camera Raw (ACR) is a separate plug-in bundled with Photoshop, Bridge, and After Effects. In CS6 the shipped version is **ACR 7.0** (CS6 is compatible with later ACR updates up to **9.1.1**, per Adobe community reports). It interprets a camera raw file (the raw, unprocessed grayscale sensor data read from a digital camera, together with capture metadata) and builds a color image. ACR can also edit **JPEG and TIFF** files (already-processed pixels).

**How the user reaches it in CS6 (no Filter-menu entry):**
- From Bridge: select raw/JPEG/TIFF files, `File > Open In Camera Raw` (`Ctrl+R` / `Cmd+R`).
- From Photoshop: `File > Open` a raw file (ACR opens), or open a raw file as a **Smart Object** (`Shift` while clicking `Open Image`) so ACR settings stay editable by double-clicking the layer.
- ACR cannot save a camera raw file; raw data is preserved, and adjustments are stored as metadata (Camera Raw database, sidecar `.XMP`, or inside a DNG).

> **Correction, sourced:** Adobe community experts and Adobe's own CC tutorials state that the ACR filter was not part of CS6 but was introduced in Photoshop CC, and that ACR is not a filter in CS6. The CS6 Help PDF likewise contains no `Filter > Camera Raw Filter` item. Any menu-based ACR filter in this project is therefore an intentional **post-CS6 extension**, not parity.

**Dialog overview (CS6):** Filmstrip (multi-image), camera name / file format label, full-screen toggle, **image adjustment tabs**, histogram (with shadow/highlight clipping toggles), Camera Raw Settings menu, zoom controls/tool, Workflow Options link, navigation arrows, adjustment sliders, and the `Save Image` / `Open Image` / `Done` / `Cancel` actions.

**Image adjustment tabs** (CS6, ACR 7):
- **Basic** — white balance, color saturation, tonality.
- **Tone Curve** — nested `Parametric` (Highlights/Lights/Darks/Shadows region sliders, draggable region dividers; Targeted Adjustment tool) and `Point` (drag points; `Curve` menu preset, default **Medium Contrast**).
- **Detail** — sharpening (`Amount`, `Radius`, `Detail`, `Masking`) and Noise Reduction (luminance: `Luminance`, `Luminance Detail`, `Luminance Contrast`; chroma: `Color`, `Color Detail`).
- **HSL / Grayscale** — nested `Hue` / `Saturation` / `Luminance` tabs over the eight color ranges; `Convert To Grayscale` reveals a single `Grayscale Mix` tab.
- **Split Toning** — `Highlights Hue/Saturation`, `Shadows Hue/Saturation`, `Balance`. (Tones grayscale images; also used for cross-process effects.)
- **Lens Corrections** — nested `Profile` (auto profile correction) and `Manual` (transform, chromatic aberration, vignette) tabs, plus a `Color` tab for chromatic-aberration/fringe work in 7.1+.
- **Effects** — `Grain` (`Amount`, `Size`, `Roughness`) and **Post Crop Vignetting** (`Style` = Highlight Priority / Color Priority / Paint Overlay; `Amount`, `Midpoint`, `Roundness`, `Feather`, `Highlights`).
- **Camera Calibration** — `Camera Profile` (Adobe Standard / Camera Matching / Embedded), calibration sliders, and `Process` version selector.
- **Presets** — save/apply named subsets of settings.
- **Snapshots** — create named renditions of the full edit state; rename, update-with-current, delete; interoperates with Lightroom.

**Process Versions:** **PV2012** (new in ACR 7) offers the new Basic tone controls and tone-mapping for high-contrast images; **PV2010** (ACR 6) and **PV2003** (ACR 5.x and earlier) remain selectable. Older images can be updated via the exclamation-point "Update to Current Process" button or `Camera Calibration > Process > 2012`.

**Local adjustments:** `Adjustment Brush` (`K`) and `Graduated Filter` (`G`). Adjustable effects (PV2012) include `Temp`, `Tint`, `Exposure`, `Highlights`, `Shadows`, `Contrast`, `Saturation`, `Clarity`, `Sharpness`, `Noise Reduction`, `Moiré Reduction`, `Defringe`, `Color`. Brush options: `Size`, `Feather`, `Flow`, `Density`, `Auto Mask`, `Show Mask`. Graduated filter uses a red/green overlay with rotation and range handles. Local presets are managed from the Camera Raw Settings menu and cannot be saved into image presets.

**Other tools:** Rotate 90° CCW (`L`) / CW (`R`); `Straighten` (`A`, then Crop activates); `Crop` (`C`, aspect-ratio menu, move/scale/rotate handles); `Red Eye Removal` (`E`, `Pupil Size`, `Darken`); `Spot Removal` (`Type` = Heal / Clone, `Radius`, draggable source/target circles). Up to nine color samplers; shadow/highlight clipping previews (`U` / `O`); HDR editing for 16-/24-/32-bit float TIFF/DNG in ACR 7.1+ (Basic `Exposure` range expands to +10…−10).

**Workflow Options:** `Space` (target profile), `Depth` (8- or 16-bpc), `Size`, `Resolution`, `Sharpen For` + `Amount`, `Open In Photoshop As Smart Objects`. Save formats from ACR: DNG (compatibility, lossless / linear demosaiced, embedded JPEG preview, embed original raw), JPEG, TIFF, PSD.

**Limits:** ACR supports images up to 65,000 px long/wide and up to 512 MP; CMYK images are converted to RGB on open.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Bridge `File > Open In Camera Raw` | Menu | `Ctrl+R` / `Cmd+R` | Primary CS6 entry for raw/JPEG/TIFF |
| Photoshop `File > Open` (raw file) | Menu | — | Opens the ACR dialog |
| Photoshop `Open Image` + `Shift` | Modifier | `Shift` | Opens the result as a Smart Object |
| `Edit > Preferences > Camera Raw` | Dialog | `Ctrl+K` | JPEG/TIFF handling, cache, defaults, settings storage |
| ACR dialog — image adjustment tabs | Tab strip | — | Basic, Tone Curve, Detail, HSL/Grayscale, Split Toning, Lens Corrections, Effects, Camera Calibration, Presets, Snapshots |
| ACR dialog — Filmstrip | Panel | — | Multi-image select/rate/synchronize/mark-for-deletion |
| ACR dialog — Histogram | Widget | `U`/`O` | Toggles shadow/highlight clipping |
| ACR dialog — White Balance tool | Tool | `I`* | Click neutral area to set Temp/Tint |
| ACR dialog — Targeted Adjustment tool | Tool | `T` (toggles last) | Parametric curve / HSL / Grayscale Mix drag |
| ACR dialog — Crop tool | Tool | `C` | Aspect menu on press-and-hold |
| ACR dialog — Straighten tool | Tool | `A` | Activates Crop after use |
| ACR dialog — Rotate CCW / CW | Button | `L` / `R` | 90° steps |
| ACR dialog — Red Eye Removal | Tool | `E` | Pupil Size, Darken |
| ACR dialog — Spot Removal | Tool | `B`* | Heal / Clone; Radius |
| ACR dialog — Adjustment Brush | Tool | `K` | Local adjustment; paint mask, pins |
| ACR dialog — Graduated Filter | Tool | `G` | Local adjustment; red/green overlay |
| ACR dialog — Workflow Options | Link | — | Space/Depth/Size/Resolution/Sharpen For/Smart Objects |
| ACR dialog — Save Image | Button | `Alt`-suppress | DNG/JPEG/TIFF/PSD |
| `Filter > Camera Raw Filter` | **Absent** | — | **Post-CS6 (CC v14) only — not parity** |

*Accepted in CC; CS6 Help does not itemize every tool shortcut.

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| White Balance | enum | As Shot | As Shot, Auto, Daylight, Cloudy, Shade, Tungsten, Fluorescent, Flash, Custom | Custom via Temp/Tint |
| Temperature | slider | from EXIF | raw: **2,000–50,000 K** | JPEG/TIFF: approximate −100…+100 scale (PDF) |
| Tint | slider | from EXIF | approx ±100…150 *(community, unverified)* | exact bounds unconfirmed |
| Exposure | slider | 0 | approx −5…+5; HDR +10…−10 | +1.00 ≈ one f-stop (PDF) |
| Contrast | slider | 0 | approx −100…+100 *(community)* | midtone contrast |
| Highlights / Shadows / Whites / Blacks | slider | 0 | approx −100…+100 *(community)* | PV2012 only |
| Recovery / Fill Light / Brightness | slider | 0 | PV2010/PV2003 only | legacy Basic controls |
| Clarity | slider | 0 | approx −100…+100 *(community)* | large-radius local contrast |
| Vibrance | slider | 0 | approx −100…+100 *(community)* | protects saturated colors |
| Saturation | slider | 0 | −100 (mono) … +100 (double) | PDF states −100/+100 |
| Parametric curve | 4 region sliders | 0 | approx −100…+100 *(community)* | Highlights/Lights/Darks/Shadows |
| Point curve | curve | Medium Contrast | freehand control points | Input/Output shown |
| Sharpen Amount | slider | 25 *(unverified)* | 0…150 *(community)* | 0 = off |
| Sharpen Radius | slider | 1.0 *(unverified)* | 0.5…3.0 *(community)* | px |
| Sharpen Detail | slider | 25 *(unverified)* | 0…100 *(community)* | |
| Sharpen Masking | slider | 0 | 0…100 *(community)* | 0 = all, 100 = edges only |
| Noise Luminance | slider | 0 | 0…100 *(community)* | |
| Noise Luminance Detail | slider | 50 *(unverified)* | 0…100 *(community)* | |
| Noise Luminance Contrast | slider | 0 | 0…100 *(community)* | |
| Noise Color | slider | 25 *(unverified)* | 0…100 *(community)* | |
| Noise Color Detail | slider | 50 *(unverified)* | 0…100 *(community)* | |
| HSL Hue/Sat/Lum (×8 ranges) | slider | 0 | approx −100…+100 *(community)* | Red…Magenta |
| Grayscale Mix (×8 ranges) | slider | 0 | approx −100…+100 *(community)* | shown after Convert To Grayscale |
| Split Toning Hue | slider | 0 | 0…360 | |
| Split Toning Saturation | slider | 0 | 0…100 | |
| Split Toning Balance | slider | 0 | approx −100…+100 | + to Highlights, − to Shadows |
| Lens Profile correction | check + sliders | Off / 100 | Amount over/under 100 | Distortion, Chromatic Aberration, Vignetting |
| Manual Distortion | slider | 0 | approx −100…+100 | barrel/pincushion |
| Manual Vertical / Horizontal | slider | 0 | approx −100…+100 | perspective |
| Manual Rotate | slider | 0 | angle | camera tilt |
| Manual Scale | slider | 100 | approx 1…200 | removes gaps |
| CA Fix Red/Cyan, Blue/Yellow | slider | 0 | approx −100…+100 | channel scaling |
| Defringe | enum/slider | Off | Off, Highlight Edges, All Edges | + local brush in PV2012 |
| Lens Vignette Amount | slider | 0 | approx −100…+100 | + lightens corners |
| Lens Vignette Midpoint | slider | 50 *(unverified)* | 0…100 | width of affected area |
| Grain Amount | slider | 0 | 0…100 *(community)* | 0 disables |
| Grain Size | slider | 25 *(unverified)* | 0…100 *(community)* | ≥25 may blur |
| Grain Roughness | slider | 50 *(unverified)* | 0…100 *(community)* | |
| Post Crop Vignette Style | enum | Highlight Priority | Highlight Priority / Color Priority / Paint Overlay | |
| Post Crop Vignette Amount | slider | 0 | approx −100…+100 | + lightens, − darkens |
| Post Crop Midpoint / Roundness / Feather / Highlights | slider | — | 0…100 / ±100 / 0…100 / 0…100 *(community)* | Highlights only for negative Amount |
| Camera Profile | enum | Embedded / Adobe Standard | Adobe Standard, Camera Matching, Embedded | profiles for raw only |
| Process | enum | 2012 (Current) | 2012 / 2010 / 2003 | |
| Workflow Space | enum | working RGB | built-in profiles (e.g. ProPhoto RGB) | |
| Workflow Depth | enum | 8 bpc | 8 / 16 bpc | |
| Workflow Size | enum | native | with Crop Size resample | best-quality marked `*` |
| Workflow Sharpen For | enum | — | Screen / Matte Paper / Glossy Paper, Amount Low/Standard/High | |
| Workflow Open As Smart Object | bool | preferences | on/off | `Shift` overrides per open |

Numeric ranges not stated in the CS6 Help PDF are marked *(community/unverified)* and are listed in `## Open questions`.

## Algorithms & pipeline

*(inferred / behavioral parity only, algorithm TBD.)* Adobe's raw pipeline is closed. The following staging is the observable order and the standard model for a raw converter; it is a design target, not a publicly documented spec.

1. **Decode / demosaic** — parse the camera container, unpack the Bayer/CFA (or X-Trans etc.) sensor data, apply black-level and gain, and reconstruct full RGB by demosaicing. Standard algorithms (bilinear, AHD, AMaZE, etc.) are candidates; **demosaic parity is TBD** and may be delegated to an existing decoder library.
2. **White balance** — apply scene illuminant as multipliers; `Temperature` and `Tint` map to a chromatic adaptation (e.g. Bradford) against the camera-native white point. `White Balance` tool solves for the multipliers that neutralize a clicked pixel. *(inferred)*
3. **Camera profile / color matrix** — transform camera-native RGB into a working color space using an Adobe Standard / Camera Matching profile (not an ICC profile per the PDF). Profiles map to the reference colorimetry; matrix/interpolation model is closed.
4. **Tone mapping (PV2012)** — global tone controls operate on a scene-referred/working-linear signal: `Exposure` (gain, f-stop units), `Highlights`/`Shadows` (range-selective), `Whites`/`Blacks` (endpoint/clipping), `Contrast` (mid-tone S-curve). The exact tone curves and region definitions are **behavioral parity only**.
5. **Local adjustments** — Adjustment Brush and Graduated Filter produce per-pixel masks; each mask applies a specified effect delta. Mask math (feather/flow/density/auto-mask) is locally defined. *(inferred)*
6. **Detail** — sharpening is "a variation of Unsharp Mask" (PDF): computes a detail signal, applies `Amount`, with `Radius`, `Detail` (high-frequency weighting) and `Masking` (edge mask). Noise reduction splits luminance vs chroma; the PDF describes thresholds but not the filter core.
7. **Lens corrections** — profile-based or manual (see `FILT-101`); applied in the raw domain.
8. **Effects** — film-grain synthesis (`Amount`, `Size`, `Roughness`) and post-crop vignette with three blend styles.
9. **Output transform** — crop/straighten/rotate, resize, optional output sharpening, and conversion to the chosen `Space`/`Depth`, then either opened in Photoshop (optionally as a Smart Object wrapping the raw source) or saved to DNG/JPEG/TIFF/PSD.

Settings are metadata only; the raw data is never overwritten. For a Photoshop integration, the same settings record drives both the interactive preview and the final render, so the renderer must be deterministic and cacheable.

## Rust module mapping

- `pictura_raw::AcrParams` — the full serialized settings tree (per-tab structs); the single source of truth for rendering. `Copy + Serialize`.
- `pictura_raw::decode::RawFile` — container parse + sensor data + metadata (EXIF, WB, camera/lens IDs). Candidate backing: a permissively licensed raw decoder crate or an in-house decoder; **decision deferred**.
- `pictura_raw::pipeline::Acr7` — `render(RawFile, &AcrParams, ProcessVersion) -> LinearImage`; stage graph for WB → profile → tone → local → detail → lens → effects → output.
- `pictura_raw::wb` — white-balance multipliers and illuminant solving (`Temp`/`Tint` ↔ chromatic adaptation).
- `pictura_raw::profile::CameraProfile` — Adobe Standard / Camera Matching lookup; `Embedded` fallback.
- `pictura_raw::tone::Pv2012` — region-selective tone controls and target curve; `Pv2010`, `Pv2003` variants.
- `pictura_raw::local::{BrushMask, GraduatedMask}` — sparse mask representation and effect application.
- `pictura_raw::detail::{Sharpen, NoiseReduction}`.
- `pictura_raw::lens` — reused by `FILT-101`.
- `pictura_raw::effects::{Grain, PostCropVignette}`.
- `pictura_io::xmp::crs` — read/write the `crs:` XMP namespace; lossless-preserve unknown keys.
- `pictura_io::dng` — DNG open/save; embed-or-sidecar policy.
- `pictura_io::psd::smart` — Smart Object wrapping a raw source; editable ACR settings on the embedded object.

Crossing types: `RawFileHandle`, `AcrParams`, `ProcessVersion`, `CameraProfileId`, `LinearImage` / `TileStore`, `XmpPacket`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `AcrDialog` | `QDialog` | Dedicated ACR workspace: filmstrip, preview, tab strip, toolbar, workflow link, Save/Open/Done |
| `AcrTabs` | `QTabWidget` | Hosts Basic, Tone Curve, Detail, HSL/Grayscale, Split Toning, Lens Corrections, Effects, Camera Calibration, Presets, Snapshots |
| `FilmstripModel` / `FilmstripView` | `QAbstractItemModel` / `QListView` | Multi-image selection, rating, sync, mark-for-deletion |
| `HistogramView` | custom `QWidget` | RGB histogram + clipping toggles + RGB readout |
| `ToneCurveWidget` | custom `QWidget` | Parametric region handles + Point curve editing, Targeted Adjustment integration |
| `HslColorRangeModel` | `QAbstractTableModel` | 8 ranges × Hue/Sat/Lum; Grayscale Mix variant |
| `LensCorrectionPanel` | `QWidget` | Profile list, amount sliders, manual transform, CA, vignette, grid (shared with `FILT-101`) |
| `LocalAdjustmentOverlay` | `QQuickWidget` / `QGraphicsView` | On-image brush pins, graduated red/green overlay and handles |
| `AcrPreview` | `QRhiWidget` / `QOpenGLWidget` | GPU preview surface; wgpu shared surface via `ARCH-006` |
| `AcrParamsModel` | `QObject` properties | Two-way binding between controls and `AcrParams`; drives undo |
| `SnapshotListModel` | `QAbstractListModel` | Snapshot create/rename/update/delete |
| `AcrSettingsMenu` | `QMenu` | Save/Load/Reset settings, defaults, local presets |
| `WorkflowOptionsDialog` | `QDialog` | Space/Depth/Size/Resolution/Sharpen For/Smart Objects |

Rationale: panel/tab controls are Widgets (dense, model-driven, keyboard-navigable); the on-canvas overlays and preview are QML/QtQuick for smooth GPU composition. The panel set is shared with the `FILT-101` Lens Correction dialog.

## Data-model impact

- **No new document node in CS6.** ACR edits a *source* (raw/JPEG/TIFF); the result is opened into a layer, or a raw file is opened as a **Smart Object** whose editable settings travel with the object.
- **Settings record:** a versioned `AcrParams` blob stored in one of: sidecar `.XMP` (`crs:` namespace), the ACR database (keyed by file content), or inside a DNG. For the Smart Object path, the settings are the object's editable source settings (`ARCH-008`, `LAY-020`).
- **Undo:** ACR maintains its own in-dialog history/snapshots; Photoshop sees the whole ACR session as a single history step when the object is committed. The integrated model should make each `AcrParams` mutation an undoable command while the dialog is open (`ARCH-009`).
- **Serialization:** round-trip must preserve unknown `crs:` keys and unknown DNG tags byte-for-byte for lossless interop with Adobe and Lightroom (`ARCH-008`, `file-formats.md`).
- **Smart Filters:** the ACR result can be committed into a Smart Object and then masked; ACR itself is not a Smart Filter in CS6.
- **Workflow metadata (Space/Depth/Size/Resolution)** is not stored with the settings (PDF): it is a per-output decision.

## Edge cases

- **CS6 vs CC menu scope** — `Filter > Camera Raw Filter` must not be treated as CS6 parity; if implemented, gate it as an extension.
- **Raw vs JPEG/TIFF** — non-raw files have no camera profile and use the approximate −100…+100 temperature scale; Adobe Standard / Camera Matching profiles are raw-only ("Embedded" appears for TIFF/JPEG).
- **HDR** — ACR 7.1+ opens 16-/24-/32-bit float TIFF/DNG only under PV2012; `Exposure` range expands to ±10; output opens as 8- or 16-bit per Workflow Options.
- **CMYK** — converted to RGB on open.
- **Camera not supported** — decode fails; the UI must report the camera/file as unsupported rather than crash.
- **Lens profile missing / no EXIF** — auto correction unavailable; must fall back to manual, with a clear message (see `FILT-101`).
- **Process-version migration** — opening a PV2010/PV2003 image must not silently change rendering; the user must opt into PV2012 and image appearance should change predictably.
- **Settings storage on read-only media** — ACR cannot write XMP to a read-only volume and falls back to its database; mirror this behavior.
- **Lightroom interop** — ACR reads only primary-image settings, not virtual copies (PDF); preserve this boundary.
- **64-bit/Linux** — ACR's GPU usage must be replaced by wgpu/QRhi; CPU fallback required when no GPU.
- **Very large images / PSB** — 65,000 px / 512 MP ACR ceiling; processing must be tiled/streamed.
- **Memory** — a full-resolution linear float buffer per stage is expensive; use tiled/intermediate caches keyed by params hash.
- **Undo/redo** — committing a multi-hundred-slider session as one Photoshop step must be consistent and reversible.
- **Determinism** — preview and final render must match; the pipeline must not depend on display zoom.

## Parity acceptance criteria

- Given a supported camera raw file opened from Bridge in a CS6-equivalent, the ACR dialog appears with the full tab set and the Filmstrip when multiple files are selected.
- Given a raw image, setting `Temperature` within 2,000–50,000 K warms/cools the rendered image monotonically; a JPEG/TIFF shows the approximate −100…+100 scale instead.
- Given `Highlight Priority` post-crop vignette with negative `Amount`, corners darken while highlight contrast is preserved relative to `Color Priority` and `Paint Overlay` on the same image.
- Given a click with the White Balance tool on a neutral patch, the resulting display of that patch is neutral within a defined ΔE tolerance.
- Given `Open Image` with `Shift`, the result is a Smart Object layer whose double-click reopens ACR with the saved settings.
- Given a raw file with a matching installed lens profile, enabling profile correction reduces measured distortion/CA/vignetting relative to the uncorrected render, and `Amount` 0/100/200 scales the correction monotonically.
- Given a 32-bit float HDR TIFF in ACR 7.1+ under PV2012, `Exposure` accepts +10…−10 and the output opens as 8- or 16-bit per Workflow Options.
- Given a raw file with sidecar XMP edits made in Lightroom, opening it in ACR shows the same primary-image adjustments (virtual copies excluded).
- Given a CS6 scope check, `Filter > Camera Raw Filter` is absent from the CS6 menu model; if present it is flagged as a post-CS6 extension.
- Given a committed ACR session, one undo returns the layer/object to its pre-ACR state.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — the CS6 Help corpus (downloaded and converted with `pdftotext -layout`). Established: `Camera Raw 7 plug-in`; raw-file definition and preservation of raw data; JPEG/TIFF support; no Filter-menu ACR entry; dialog elements; the image-adjustment tab set and each tab's purpose; PV2012/2010/2003; the full Basic control list including PV2012-only Highlights/Shadows/Whites/Blacks and legacy Recovery/Fill Light/Brightness; `Temperature` 2,000–50,000 K for raw and the approximate −100…+100 scale for JPEG/TIFF; Tone Curve parametric/point behavior; Detail sharpening and noise-reduction controls; HSL/Grayscale and Split Toning; local-adjustment tools and effects; rotate/straighten/crop/red-eye/spot; Lens Corrections Profile/Manual/Color; Camera Calibration profiles (Adobe Standard / Camera Matching, not ICC); Effects grain and post-crop vignette styles/sliders; Presets and Snapshots; settings storage (database / sidecar XMP / DNG); workflow options; output formats; 65,000 px / 512 MP limit; CMYK→RGB; HDR support in 7.1+ with ±10 exposure.
- `https://community.adobe.com/t5/photoshop-ecosystem-discussions/unable-to-install-camera-raw-9-1-1-in-cs6-was-no-camera-raw-filter-in-cs6/m-p/8541203` — established that the camera raw filter is not part of CS6 (it was introduced in Photoshop CC) and that ACR is not a filter in CS6; CS6 ACR updates were for new-camera support only.
- `https://digital-photography-school.com/adobe-camera-raw-acr-photoshop-filter` — secondary confirmation that the Camera Raw filter is new to the Creative Cloud version and was not available in earlier Photoshop versions; describes ACR-as-filter + Smart Object behavior.
- `https://www.thegraphicmac.com/photoshop-cs4s-shortcut-changes-and-missing-features` — established the CS4 removal of Extract and Pattern Maker from the default install (used in `FILT-104`).
- `https://planetphotoshop.com/wheres-my-patternmaker.html` — secondary confirmation of the CS4 Pattern Maker/Extract removal (used in `FILT-104`).

Discovery searches (results, not opened as pages): DuckDuckGo HTML and Brave/SearXNG queries for "Camera Raw Filter introduced CS6 or CC" (DuckDuckGo later returned a bot challenge).

Marked inferred/community (not primary): numeric slider ranges absent from the PDF; demosaic/tone/profile internals; the exact ACR 7.1 chromatic-aberration behavior.

## Open questions

- **Filter-menu scope.** Confirm the project treats `Filter > Camera Raw Filter` as a post-CS6 extension. Resolve with a `00-overview/feasibility-and-non-goals.md` decision; the task premise ("introduced CS6") is contradicted by Adobe secondary sources and the CS6 Help PDF.
- **Exact slider ranges/defaults.** The Help PDF does not tabulate numeric ranges. Resolve by reading ACR 7.0's UI metadata or an official ACR 7 reference; every `(community)` value above needs verification.
- **Raw decoding strategy.** Build an in-house decoder or wrap an existing library? This is the single largest scope question. Resolve with a licence/quality/coverage evaluation (and a decision on DNG as the internal raw container).
- **Demosaic parity.** Adobe's demosaic is closed; define whether parity means visual similarity or a specific pixel tolerance, and against which reference demosaicer.
- **PV2012 tone-model parity.** The region definitions, curves, and clipping behavior are closed. Resolve with a reference dataset and measured tolerances, or accept behavioral parity only.
- **Camera profile format.** How are Adobe Standard / Camera Matching profiles stored and evaluated (DCP)? Resolve by parsing an installed profile set; may require a documented third-party implementation.
- **Chromatic-aberration model.** Which CA type(s) ACR 7.1 actually corrects and with what model is not public. Resolve with test images and analysis.
- **Grain synthesis.** The grain generator is closed; define tolerance on grain statistics (spectrum, contrast).
- **HDR float precision.** Whether ACR 7.1 uses `f32` end-to-end or `f16` intermediates affects parity; resolve with a memory/quality study.
- **Third-party plugins.** CS6 ACR also served as a host for camera profiles and the DNG Converter; decide whether the project ships any equivalent.
- **Colour management coupling.** The `Space` menu and the project's ICC/lcms2 pipeline (`ARCH-010`/`color-management.md`) must be reconciled; define the working-space default (PDF suggests ProPhoto RGB as the escape hatch).
