# Distort Filters

- **Spec ID:** `FILT-040`
- **Status:** `Draft`
- **Parity tier:** `Core` (the geometric warp family) + `Extended-only` for **Lens Correction** (documented separately in `06-filters/lens-correction.md`)
- **New in CS6:** `Yes` — CS6 rebuilds **Lighting Effects** (render family) and adds **Adaptive Wide Angle**, but the Distort submenu itself is unchanged from CS5. **Lens Correction** gains the **Auto Correction** lens-profile workflow (CS5 had manual correction only).
- **Depends on:** `04-image-ops/bit-depth-and-conversion.md`, `01-architecture/color-management.md`, `01-architecture/gpu-rendering-pipeline.md`, `01-architecture/document-model.md`, `05-layers/smart-filters.md`, `05-layers/blend-modes.md`, `06-filters/filters-overview.md`, `06-filters/lens-correction.md`, `06-filters/liquify.md`, `06-filters/adaptive-wide-angle.md`

> All crate, module, widget, and type names below are **design proposals**. No code exists in this repository. Behavior and numeric ranges are taken from the fetched sources unless marked *(inferred)*. Adobe's exact resampling kernels and noise generators are closed; those are marked **behavioral parity only, algorithm TBD**.

## CS6 behavior

`Filter > Distort` geometrically warps the active layer or selection. Help notes these filters "can be very memory-intensive". The submenu contains:

| Filter | Menu path | What it does (CS6 Help) |
|---|---|---|
| Diffuse Glow | `Filter > Distort > Diffuse Glow` | Renders an image as if viewed through a soft diffusion filter; adds see-through white noise, with the glow fading from the center of a selection. Filter-Gallery capable. |
| Displace | `Filter > Distort > Displace` | Uses a second image (a displacement map) to shift pixels; gray 128 = no shift, 0 = maximum negative, 255 = maximum positive. |
| Glass | `Filter > Distort > Glass` | Simulates viewing through glass; choose a built-in surface or load a Photoshop file, then adjust scaling, distortion, and smoothness. Filter-Gallery capable. |
| Ocean Ripple | `Filter > Distort > Ocean Ripple` | Adds randomly spaced ripples so the image appears underwater. Filter-Gallery capable. |
| Pinch | `Filter > Distort > Pinch` | Squeezes toward (positive) or away from (negative) the selection center. |
| Polar Coordinates | `Filter > Distort > Polar Coordinates` | Converts rectangular ⇄ polar coordinates. |
| Ripple | `Filter > Distort > Ripple` | Undulating ripple pattern; choose number and size. For more control use Wave. |
| Shear | `Filter > Distort > Shear` | Distorts along a user-drawn curve; choose how to treat undistorted areas. |
| Spherize | `Filter > Distort > Spherize` | Wraps the selection around a sphere (3D bulge/pincushion look). |
| Twirl | `Filter > Distort > Twirl` | Rotates more sharply at the center than the edges. |
| Wave | `Filter > Distort > Wave` | Like Ripple with control over generators, wavelength, amplitude, and wave type. |
| ZigZag | `Filter > Distort > ZigZag` | Radial distortion by radius; ridges and displacement style. |
| Lens Correction | `Filter > Lens Correction…` | Separate dialog; corrects barrel/pincushion distortion, chromatic aberration, vignetting, perspective and rotation. See its own spec. |

**Undefined areas.** Help groups Displace, Shear, and Wave into one rule: areas the filter leaves undefined are treated by **Wrap Around** (content from the opposite edge) or **Repeat Edge Pixels** (extend the edge color; may band). The Offset filter (Other) adds **Set To Background**. (Sourced — Help "Defining undistorted areas".)

**Glass surface controls.** Glass shares texturizing options with Rough Pastels, Underpainting, Conté Crayon, and Texturizer: choose or load a texture, set **Scaling**, **Relief** (only where available), **Invert**, and **Light Direction** (only where available). (Sourced — Help "Set texture and glass surface controls".)

**Lens Correction entry points.** The dialog has two tabs, **Auto Correction** (lens profiles, Edge handling, Auto Scale Image) and **Custom** (manual sliders). It works only on **8- and 16-bit-per-channel RGB or Grayscale**. (Sourced.)

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Filter > Distort > Diffuse Glow` | Menu / Filter-Gallery entry | — | Gallery-capable (8-bit) |
| `Filter > Distort > Displace` | Menu / modal dialog | — | Loads a displacement-map PSD after OK |
| `Filter > Distort > Glass` | Menu / Filter-Gallery entry | — | Gallery-capable; texture controls |
| `Filter > Distort > Ocean Ripple` | Menu / Filter-Gallery entry | — | Gallery-capable |
| `Filter > Distort > Pinch` | Menu / modal dialog | — | Preview + slider |
| `Filter > Distort > Polar Coordinates` | Menu / modal dialog | — | Radio options only |
| `Filter > Distort > Ripple` | Menu / modal dialog | — | Amount slider + size combo |
| `Filter > Distort > Shear` | Menu / modal dialog | — | Curve grid with draggable control points; Undefined Areas radios; live preview below |
| `Filter > Distort > Spherize` | Menu / modal dialog | — | Amount slider + mode combo |
| `Filter > Distort > Twirl` | Menu / modal dialog | — | Angle dial/slider |
| `Filter > Distort > Wave` | Menu / modal dialog | — | Numeric fields + Randomize |
| `Filter > Distort > ZigZag` | Menu / modal dialog | — | Amount/Ridges + style combo |
| `Filter > Lens Correction…` | Menu / full workspace dialog | — | Auto + Custom tabs; grid; Straighten tool |
| `Filter > Filter Gallery > Distort > …` | Gallery | — | Diffuse Glow, Glass, Ocean Ripple |
| Smart Filter row | Layers panel | — | Any transformable filter gains filters if gallery/distort is executable there |

## Parameters & ranges

Ranges below marked **[AS]** are taken from the Photoshop CS6 AppleScript Scripting Reference (Adobe, filter-options classes). Ranges marked **[Help]** are stated in the CS6 Help text. Everything else is *(inferred)* from the dialog behavior and marked as such.

| Filter | Control | Type | Default | Range / options | Source |
|---|---|---|---|---|---|
| Diffuse Glow | Graininess | int | 6 *(observed)* | 0–10 | [AS] |
| Diffuse Glow | Glow Amount | int | 10 *(observed)* | 0–20 | [AS] |
| Diffuse Glow | Clear Amount | int | 15 *(observed)* | 0–20 | [AS] |
| Displace | Horizontal Scale | int % | 10 *(photorust)* | −999–999 | [AS] |
| Displace | Vertical Scale | int % | 10 *(photorust)* | −999–999 | [AS] |
| Displace | Displacement Map | enum | Stretch To Fit *(photorust)* | Stretch To Fit / Tile | [AS] |
| Displace | Undefined Areas | enum | Repeat Edge Pixels *(photorust)* | Wrap Around / Repeat Edge Pixels | [AS] |
| Displace | Map file | file | — | Flattened Photoshop PSD; Bitmap unsupported | [Help] |
| Glass | Distortion | int | 5 *(inferred)* | 0–20 | [AS] |
| Glass | Smoothness | int | 3 *(inferred)* | 1–15 | [AS] |
| Glass | Scaling | int % | 100 *(inferred)* | 50–200% | [AS] |
| Glass | Texture | enum | Frosted *(inferred)* | Blocks / Canvas / Frosted / Tiny Lens / Load Texture (texture document) | [AS] |
| Glass | Invert Texture | bool | false | on / off | [AS] |
| Glass | Texture Definition | file | — | Photoshop PSD (mutually exclusive with a built-in Texture) | [AS] |
| Ocean Ripple | Ripple Size | int | 9 *(observed)* | 1–15 | [AS] |
| Ocean Ripple | Ripple Magnitude | int | 9 *(observed)* | 0–20 | [AS] |
| Pinch | Amount | int % | 0 | −100–100 (positive = toward center) | [AS][Help] |
| Polar Coordinates | Kind | enum | Rectangular to Polar | Rectangular to Polar / Polar to Rectangular | [AS] |
| Ripple | Amount | int | 100 *(inferred)* | −999–999 | [AS] |
| Ripple | Ripple Size | enum | Medium *(inferred)* | Small / Medium / Large | [AS] |
| Shear | Curve | list of points (position, offset) | straight | 2–8 control points dragged in the grid; position runs top to bottom | [AS] |
| Shear | Undefined Areas | enum | Wrap Around *(inferred)* | Wrap Around / Repeat Edge Pixels | [AS] |
| Spherize | Amount | int % | 100 *(inferred)* | −100–100 | [AS] |
| Spherize | Mode | enum | Normal | Normal / Horizontal Only / Vertical Only | [AS] |
| Twirl | Angle | int ° | 0 *(inferred)* | −999–999 | [AS] |
| Wave | Number of Generators | int | 5 *(inferred)* | 1–999 | [AS] |
| Wave | Wavelength (min / max) | int | 10 / 120 *(inferred)* | min 1–998; max 2–(min+1) | [AS] |
| Wave | Amplitude (min / max) | int | 5 / 35 *(inferred)* | min 1–998; max 2–(min+1) | [AS] |
| Wave | Scale (Horizontal / Vertical) | int % | 100 / 100 *(inferred)* | 1–100% each | [AS] |
| Wave | Type | enum | Sine | Sine / Triangle / Square | [AS][Help] |
| Wave | Random Seed | int | — | number controlling random wave lengths; Randomize button | [AS] |
| Wave | Undefined Areas | enum | Wrap Around *(inferred)* | Wrap Around / Repeat Edge Pixels | [AS] |
| ZigZag | Amount | int % | 0 *(inferred)* | −100–100 | [AS] |
| ZigZag | Ridges | int | 5 *(inferred)* | 0–20 | [AS] |
| ZigZag | Style | enum | Around Center | Around Center / Out From Center / Pond Ripples | [AS][Help] |
| Lens Correction | Remove Distortion | slider | 0 | barrel ↔ pincushion (sign convention from the tool) | [Help] |
| Lens Correction | Fix Fringe (Red/Cyan, Blue/Yellow) | slider | 0 | per-pair channel scaling | [Help] |
| Lens Correction | Vignette Amount | slider | 0 | lighten ↔ darken edges | [Help] |
| Lens Correction | Vignette Midpoint | slider | 50 *(inferred)* | low = wider affected area; high = edges only | [Help] |
| Lens Correction | Vertical / Horizontal Perspective | slider | 0 | makes vertical/horizontal lines parallel | [Help] |
| Lens Correction | Angle | slider ° | 0 | rotate; Straighten tool available | [Help] |
| Lens Correction | Scale | slider | 100% *(inferred)* | zoom/crop to remove blank edges; pixel dimensions unchanged | [Help] |
| Lens Correction | Edge | enum | — | Transparency / Background Color / Edge Extension | [Help] |
| Lens Correction | Auto Scale Image | bool | off *(inferred)* | keeps original dimensions under correction | [Help] |
| Lens Correction | Show Grid / Size / Color | bool / int / color | off / — / — | grid overlay and Move Grid tool | [Help] |
| Lens Correction | Correction checkboxes | bool | — | Geometric Distortion / Chromatic Aberration / Vignette; Auto Correction tab | [Help] |
| Lens Correction | Lens Profiles / Search Online | list / button | auto-match | EXIF-matched profiles; RAW-profile preference | [Help] |

**Bit-depth gate (Help "Filter basics").** Of the Distort family, only **Lens Correction** is listed as 16-bit capable. None of the geometric Distort filters are listed for 16-bit or 32-bit — they are 8-bit only. `Lens Correction` is 8/16-bit RGB or Grayscale only. (Sourced.)

## Algorithms & pipeline

**Behavioral parity only, algorithm TBD** where noted. The table gives the algorithm *family* each filter belongs to; Adobe's exact resampling filter, edge handling, and RNG are undocumented and must be matched by comparison renders.

| Filter | Algorithm family | Notes |
|---|---|---|
| Displace | Per-pixel inverse lookup into a displacement map | Channel 1 drives horizontal, channel 2 vertical; each channel value `v` maps to shift `(v−128)/128 × scale` pixels; with one channel the shift is along the diagonal defined by the two scales. At 100% the largest shift is 128 px. Map is stretched or tiled to the selection. *[Help]* Each pixel reads the source that far away, so a light map pulls the picture up and to the left, the same direction as ImageMagick's `-compose displace`; a stretched map is sampled bilinearly. *(inferred; matches photorust and the ImageMagick oracle)* |
| Pinch / Spherize | Radial inverse-mapping warp | Normalized radial distance remapped by a monotone curve; Spherize 3D-wraps around a sphere, Pinch squeezes/expands toward center. *(inferred)* |
| Twirl | Angular inverse-mapping warp | Rotation angle falls off from center to edge; angle sign sets direction. *(inferred)* |
| Polar Coordinates | Coordinate transform | `rect→polar` and `polar→rect` with bilinear resampling. *(inferred)* |
| Ripple | Sinusoidal displacement | Periodic offsets; size sets spatial frequency. *(inferred)* |
| Wave | Multi-generator sinusoidal displacement | Sum of `N` wave generators; each has a random phase/period drawn from wavelength range, amplitude range, and type (sine/triangle/square); scale applies axis-wise; random seed makes it repeatable. *[AS]* |
| Shear | Smooth horizontal shift | Curve control points spline-interpolated over the row position (top `-1`, bottom `1`); each row shifts horizontally by `offset × width/2`; undefined columns wrap or repeat edges. *(inferred; matches CS6's row shift and photorust's model)* |
| ZigZag | Radial displacement | Amount scales magnitude, ridges set the number of direction reversals from center to edge; three styles (around center = rotation, out from center = radial, pond ripples = diagonal). *(inferred)* |
| Glass | Texture-driven refraction | Each pixel samples the source displaced along the *slope* of the texture's height field, so each bump acts as a small lens. Distortion scales the reach, Smoothness blurs the height field, Scaling sizes it, and Invert reverses the reach. CS6 renders rule out Displace-style mapping, where one grey value shifts x and y equally: at Tiny Lens, each dome holds its own small piece of the picture. *(approximation tuned against CS6 renders; the built-in surfaces are procedural stand-ins for CS6's bitmaps)* |
| Ocean Ripple | Refraction through random bumps | Pixels are displaced along the slope of a seeded, smoothly interpolated noise surface. Size sets the bump size, which grows only slowly. Magnitude sets the reach, which grows faster than linearly: about 2 px at Magnitude 2, frosted-glass blobs at 9–12, and scattered fragments of about 30 px at 20. *(approximation tuned against CS6 renders)* |
| Diffuse Glow | Luminance-driven mix toward the glow colour | Every channel of a pixel moves toward the glow colour (the Background colour) by one shared fraction. That fraction rises with the pixel's blurred luminance, so highlights bloom and spill past their edges as a halo. Glow Amount lowers the threshold, steepens the ramp, and widens the halo. A low Clear Amount lifts the whole picture with a veil: CS6 lifts the shadows about 58 % at Clear 2, about a tenth at Clear 6, and almost none from Clear 10. Graininess adds see-through noise to the fraction. No centre falloff was visible in the CS6 renders. *(least-squares fit to CS6 renders)* |
| Lens Correction | Radial polynomial distortion + per-channel scaling + vignette | Undistortion is a radial polynomial (Brown–Conrady-style `r²`,`r⁴` terms) inverse-mapped; chromatic aberration scales one channel pair relative to another; vignette applies a radial gain with a midpoint; perspective/angle are homographies. Auto Correction matches EXIF to a lens profile. *(inferred model; profile coefficients are Adobe data)* |

**Pipeline.** All Distort filters are single-image, single-pass on the active layer/selection (Wave and Displace read one auxiliary input). For a Rust core they should share one `Warp` trait that renders output tiles from an input tile + a per-pixel displacement field; the filters differ only in how the field is generated. This is the lazy, GPU-friendly shape and matches how `gpu-rendering-pipeline` wants to run filters.

## Rust module mapping

- `pictura_filter::distort` — submodule per filter; all implement `Filter`.
- `pictura_filter::warp::Warp` — trait: `displacement(&self, ctx) -> DisplacementField`, then a shared inverse-mapping resampler.
- `pictura_filter::warp::DisplacementField` — `{ dx: Tile, dy: Tile, interpolation: Bilinear|Bicubic, edge: EdgeMode }`.
- `pictura_filter::distort::displace` — loads a `pictura_io::psd` document as the map, flattens it, picks channels 1/2.
- `pictura_filter::distort::wave` / `ripple` / `ocean_ripple` — seeded RNG (`u32 seed`) so Randomize is reproducible.
- `pictura_filter::distort::glass` — `TextureSource::{Builtin(GlassSurface), Document(PathBuf)}`; shares the texture pipeline with other texturizing filters.
- `pictura_filter::distort::lens_correction` — `CameraProfile` (distortion polynomial, CA, vignette), `Perspective { v, h, angle, scale }`; the Auto tab reads `pictura_core::metadata` (EXIF) and a profile store.
- `pictura_core::meta::Exif` — camera/lens/focal/f-stop for profile matching.
- `pictura_filter::registry` — maps filter id (and the CS6 four-char event id, e.g. `'Dspl'`, `'Twrl'`, `'Wave'`, `'ZgZg'`, `'Gls '`, `'OcnR'`) to implementation + `supported(mode, depth)`.

Crossing types: `Tile`, `Rect`, `ColorMode`, `BitDepth`, `EdgeMode`, `FilterParams` (typed enum), `DisplacementField`, `PathBuf`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `DistortOptionsDialog` | `QDialog` | Hosts each filter's small parameter form; swap-in `QStackedWidget` per filter |
| `ShearCurveEditor` | `QWidget` | Draws the shear curve grid; drag control points; click the line to add, drag a point out to remove |
| `WaveOptionsPanel` | `QWidget` | Generators, min/max wavelength/amplitude spin boxes, type combo, Randomize |
| `DisplaceOptionsPanel` | `QWidget` | Scale spins, Stretch/Tile radios, Undefined Areas radios, no preview; file chooser opens after OK |
| `GlassOptionsPanel` | `QWidget` | Distortion/Smoothness/Scaling sliders, texture combo + Load Texture, Invert |
| `LensCorrectionWidget` | `QWidget` (workspace) | Auto/Custom tabs, sliders, Straighten tool, grid overlay, lens-profile list |
| `FilterPreviewPane` | `QGraphicsView` | Shared preview with zoom + drag-to-center (per Help) |
| `FilterMenuBuilder` | helper | Greys Distort entries when the document mode/depth is unsupported |

Widgets over QML for the dialogs (they are dense and modal), per `ARCH-003`; the preview pane may be QML if the canvas is QML-based.

## Data-model impact

- Filters record their parameters in the history/undo record as a typed `FilterParams` enum, not raw pixels (`ARCH-009`).
- **Lens Correction Auto** stores the matched profile id + sub-profile and the auto correction flags; perspective settings are **explicitly not saved** per Help, while distortion/CA/vignette and Auto settings are saveable as settings/lens defaults. That save/load shape belongs to this spec's data model.
- Undo granularity is one command per apply. Glass/Displace reference an external PSD path; for lossless round-trip the referenced file is not embedded by Photoshop (Displace loads a separate file), so the record stores the path plus the flattened map digest for cache invalidation.
- **Smart Filters:** each Distort filter that Photoshop enables can sit on a Smart Object stack (`LAY-021`). The filter's parameter descriptor is serialized into the Smart Filter stack; the exact `FXid`/`FEid` parameter packing is unconfirmed (see `LAY-021` open questions).
- **PSD:** the layer's filtered result is plain pixels unless the layer is a Smart Object. No Distort-specific additional-layer key is documented beyond the Smart Filter record.

## Edge cases

- **8/16/32-bit.** Geometric Distort filters are 8-bit only; on 16/32-bit documents they must be greyed/disabled or converted (matching Photoshop's disabled menu). Lens Correction works on 8/16-bit RGB/Grayscale.
- **CMYK/Lab/Multichannel.** Distort filters are documented for 8-bit but some refuse CMYK/Lab; enforce per-filter `supported(mode, depth)` and grey the menu.
- **Filter Gallery boundary.** Diffuse Glow, Glass, and Ocean Ripple are applied through the Filter Gallery; the Gallery is cumulative and 8-bit only, so its ordering/preview rules apply (`06-filters/filters-overview.md`).
- **1-px or empty selection.** Radial filters (Pinch/Twirl/Spherize/ZigZag) have no meaningful center; define a no-op with no error.
- **Displace map size mismatch.** Stretch To Fit vs Tile; a map with one channel displaces diagonally; >1 channel uses channels 1 and 2. Bitmap-mode maps are rejected.
- **Huge/PSB documents.** Help warns Distort filters are memory-intensive. Render tiled, not full-canvas scratch buffers; Wave with 999 generators and large amplitude is the worst case.
- **GPU unavailable.** All warps fall back to CPU bilinear/bicubic; the cache key must include the backend.
- **Seeded randomness.** Wave's Randomize must be reproducible from a stored seed so undo/redo and re-render match.
- **Lens Correction without EXIF.** Auto Correction is unavailable; fall back to the Custom tab.
- **Lens Correction blank edges.** Edge = transparency/color/extension; Scale can crop; pixel dimensions must not change.
- **Twirl/Wave on very large canvases.** Modern Help notes Twirl "does not work on images larger than 11500px by 11500px" (current docs, not necessarily CS6); guard against overflow.
- **Undo/redo.** Store parameters + RNG seed; recompute. No pixel backup required.

## Parity acceptance criteria

- Given a grayscale image and Displace with a uniform 128 map, the output equals the input (no displacement) within tolerance 0.
- Given a Displace map and 100% scales, the maximum displacement is ~128 px and the shift is zero at value 128, negative below, positive above.
- Given Pinch Amount +100, pixels move toward the selection center; Amount −100 moves them outward; 0 is a no-op.
- Given Polar Coordinates Rect→Polar then Polar→Rect with the same parameters, the round-trip approximates the original within resampling tolerance.
- Given Ripple Amount 0, the output equals the input; increasing Amount increases displacement magnitude; Size changes spatial frequency.
- Given Wave with N generators and a fixed random seed, two runs produce identical output; Randomize changes the seed.
- Given Shear's curve set back to a straight line (Default), the output equals the input.
- Given Spherize Mode Horizontal Only, displacement occurs only on the horizontal axis.
- Given ZigZag Ridges = 0, the number of direction reversals from center to edge is 0 (uniform displacement direction).
- Given Twirl Angle +A and −A, the two outputs are mirror rotations in direction.
- Given Glass with Distortion 0, the output equals the input; Distortion > 0 displaces by the chosen/loaded texture.
- Given Diffuse Glow Clear Amount = maximum, the original detail is largely restored; Glow Amount = 0 removes the glow.
- Given a JPEG with an embedded EXIF camera+lens and a matching profile on disk, Lens Correction Auto corrects distortion/CA/vignette; Auto Scale Image keeps original dimensions.
- Given Lens Correction on a 16-bit RGB image, the filter runs; on a 32-bit image it is unavailable.
- Given a Distort filter and a 16-bit or CMYK document, the menu entry is disabled (except Lens Correction where supported).
- Given a PSD with a Smart Object carrying a Distort Smart Filter, opening and re-saving preserves the parameter descriptor round-trip.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — Adobe Photoshop CS6 Help. Established: the Distort submenu list and descriptions; the Filter-Gallery membership of Diffuse Glow/Glass/Ocean Ripple; "Defining undistorted areas" (Wrap Around / Repeat Edge Pixels / Set To Background for Offset); "Set texture and glass surface controls"; "Apply the Displace filter" (map channels, 128 = no shift, 100% = 128 px max, Stretch To Fit/Tile, Bitmap unsupported); "Use the Filter menu / bit-depth support lists" (which filters run at 16/32-bit); the full Lens Correction Auto + Custom workflow, Edge/Auto Scale, profile search, grid, and the statement that perspective settings are not saved; Filter Gallery rules.
- CS6 Filter Gallery comparison renders (issue #286). These are screenshots of the CS6 preview for two photographs. Diffuse Glow was captured at (Graininess, Glow, Clear) = 6/10/15, 2/2/2, 9/18/16 and 8/1/6. Ocean Ripple was captured at (Size, Magnitude) = 9/9, 2/12, 14/2 and 15/20. Glass was captured at Frosted 3/3/66 %, 10/6/66 % and 10/11/162 %, Blocks 17/11/162 %, Canvas 19/15/162 %, and Tiny Lens 19/15/162 %. Each render was registered to its source to establish the displacement and mix behaviour above. Defaults marked *(observed)* are the values CS6 showed in those captures, taken to be its defaults.
- `https://applescriptlibrary.files.wordpress.com/2013/11/photoshop-cs6-applescript-reference.pdf` — Adobe Photoshop CS6 AppleScript Scripting Reference. Established the exact option ranges and enum values for the scriptable Distort filters: Diffuse Glow (0–10, 0–20, 0–20), Displace (scales −999–999; kind stretch to fit/tile; undefined areas), Glass (0–20, 1–15, 50–200%, texture kinds, invert), Ocean Ripple (1–15, 0–20), Pinch (−100–100), Polar Coordinates kinds, Ripple (−999–999; small/medium/large), Shear (curve point list; undefined areas), Spherize (−100–100; normal/horizontal/vertical), Twirl (−999–999), Wave (generators/wavelength/amplitude/scale/type/seed/undefined), ZigZag (−100–100; 0–20; styles); plus the four-char event IDs (`'Dspl'`, `'Pnch'`, `'Plr '`, `'Rple'`, `'Shr '`, `'Sphr'`, `'Twrl'`, `'Wave'`, `'ZgZg'`, `'Gls '`, `'OcnR'`, `'DfsG'`).
- `https://web.archive.org/web/2014id_/https://helpx.adobe.com/photoshop/using/filter-effects-reference.html` and `https://web.archive.org/web/2016id_/https://helpx.adobe.com/photoshop/using/filter-effects-reference.html` — archived Adobe "Filter effects reference" pages; used to confirm the prose descriptions were unchanged from the CS6 Help and to check that numeric ranges are not documented in Help itself.
- `https://bpb-us-w2.wpmucdn.com/wonecks.net/dist/3/535/files/2020/03/Photoshop-filter-effects-reference.pdf` — a PDF mirror of the same Adobe reference; corroborated the prose (no added ranges).

Not used in this pass:

- `helpx.adobe.com` live pages returned HTTP 403 (per the project source policy); archived copies were used.

## Open questions

- **Exact warp kernels.** Adobe's resampling filter (bilinear? bicubic? custom), edge clamping, and sub-pixel phase for each Distort filter are undocumented. Resolve by rendering reference pairs in a real CS6 and fitting. Until then: behavioral parity only.
- **Twirl size limit.** Current Adobe docs mention an 11500×11500 limit for Twirl; whether CS6.0 enforces it is unconfirmed. Resolve with a CS6 build test.
- **Undefined-area defaults.** The option *sets* are sourced; the dialog defaults for Wrap Around vs Repeat Edge Pixels per filter are inferred. Resolve from a CS6 screenshot or defaults file.
- **Wave generator model.** The mapping from Number of Generators + wavelength/amplitude ranges + seed to the actual displacement field is closed. Resolve by fitting rendered waves.
- **Glass/Displace texture interpretation.** Whether the texture is treated as luminance-only (Help says most filters use grayscale information) and how Smoothness interpolates it is inferred. Resolve with controlled textures.
- **Lens Correction profile format.** Adobe's `.lcp` profile coefficient model and the exact polynomial order are not public in the fetched sources. Resolve from the Lens Profile Creator docs / SDK.
- **Lens Correction slider ranges.** Help describes behavior but not numeric min/max for Remove Distortion, Fix Fringe, Vignette, Perspective, Angle, Scale. Resolve from a CS6 UI capture.
- **Smart Filter parameter packing** for each Distort filter in `FXid`/`FEid`. See `LAY-021`.
