# Blur Gallery (Photographic Blur Gallery)

- **Spec ID:** `FILT-092`
- **Status:** `Partial` — Field Blur, Iris Blur, and the shared workspace are
  implemented (see `## Implementation status`); Tilt-Shift is not.
- **Parity tier:** `Core` — the Blur Gallery shipped in CS6 Standard and Extended. **GPU/OpenCL-gated**: the three CS6 effects are OpenCL-accelerated and the Help's GPU FAQ ties them to an OpenCL-capable card.
- **New in CS6:** `Yes` — CS6 introduces the **Blur Gallery** workspace with **Field Blur, Iris Blur, and Tilt-Shift** and a **Blur Effects** panel (Light Bokeh, Bokeh Color, Light Range). CS6's own Help and launch videos describe a **three-effect** gallery. **Path Blur, Spin Blur, Motion Blur Effects, Noise, and Edge Glow are post-CS6 (CC) additions** — see the clearly-marked subsection under `## CS6 behavior`. Smart-filter support is also CC-only.
- **Depends on:** `FILT-001` filters-overview, `FILT-010` blur-filters (shares the blur-kernel vocabulary), `ARCH-006` gpu-rendering-pipeline, `ARCH-003` qt6-ui-design, `ARCH-007` color-management, `ARCH-009` undo-history, `LAY-021` smart-filters, `LAY-020` smart-objects, `IMG-005` bit-depth-and-conversion.

> All module, widget, and type names below are **design proposals** unless the
> `## Implementation status` section names them as shipped. Adobe's blur
> shaders are closed; the algorithm description is **behavioral parity only,
> algorithm TBD**. Facts come from the fetched CS6 Help PDF and the fetched
> **CS6-era** Adobe Help snapshot unless marked *(inferred)* or explicitly
> labelled **post-CS6**.

## Implementation status

Field Blur, Iris Blur, their Blur Effects, and the shared Blur Gallery
workspace are implemented; Tilt-Shift, Path Blur, and Spin Blur are not. The
Noise tab is a post-CS6 addition ported on the owner's request.

- **Engine.** `pictura_filters::blur::field` (`BlurPin { x, y, blur }`,
  `BokehParams`, `BlurNoise`, `field_blur`, `MAX_FIELD_RADIUS = 500`,
  `MAX_FIELD_PINS = 16`): each pixel's radius is the inverse-square-distance
  weighted average of the pin amounts, and the variable blur linearly blends
  the two bracketing levels of a box-blur pyramid. The field weights and the
  pyramid are approximations of the `(inferred)` model above — the doc's
  disc/bokeh gather is not implemented (a per-pixel disc gather cannot drive
  a live pin drag). After the blur, the Blur Effects pass applies Light
  Bokeh (a doubled multiplicative brightening of the in-range blurred
  pixels, so bright highlights bloom brilliantly toward white) and Bokeh
  Color (a chroma push on the lightened, non-clipped pixels), both gated by
  the Light Range window and the local blur weight; the Noise pass adds
  seeded Gaussian/Uniform/Grain grain scaled by the weight and damped in
  highlights. The effect curves and the grain model are inferred
  approximations (Adobe's are closed).
- **Iris engine.** `pictura_filters::blur::iris` (`IrisPin { x, y,
  radius_x, radius_y, angle, squareness, feather, blur }`, `iris_blur`,
  `iris_exponent`, `MAX_IRIS_RADIUS = 10`): each pin is a rotated
  superellipse whose semi-axes are fractions of the buffer width
  (squareness 0..100 maps to the exponent `2 / (1 − 0.009·s)`, 2 → 20).
  A pin's falloff is 0 inside its feather core (`ρ ≤ feather`), a smoothstep
  across the fade band, and 1 outside the outline — the Help's A/B/C sharp,
  fade, and blur areas. Several pins multiply their falloffs, so each keeps
  its own sharp island, and the exterior amount is the
  inverse-square-distance blend of the pin amounts. The radius field drives
  Field Blur's pyramid and effects passes unchanged. The falloff curve, the
  squareness map, and the multi-pin combine are inferred.
- **App.** `Filter ▸ Blur Gallery ▸ Field Blur…` opens `BlurGalleryDialog`
  (with `BlurGalleryCanvas`): a document proxy with draggable circle pins
  (click adds, drag moves, the selected pin's ring changes its amount, each
  ring's white arc shows the blur amount — full at 100 px, Delete removes,
  `H` hides the UI), the Blur Tools panel with the selected pin's
  Blur (`0..=500 px`) and disabled Iris Blur / Tilt-Shift / Path Blur /
  Spin Blur rows, an Effects / Motion Effects / Noise tabbed panel (Bokeh
  with Light Bokeh, Bokeh Color, and the two-handle Light Range; Noise with
  the Gaussian/Uniform/Grain picker, Amount, Size, Roughness, Color,
  Highlights), Preview / Cancel / OK, and one history state on commit. The
  `Blur Gallery`
  submenu also lists disabled `Tilt-Shift…`, `Path Blur…`, and `Spin Blur…`
  stubs; its placement follows the later Photoshop menu (the
  owner's screenshot) rather than CS6's `Filter ▸ Blur` listing. Last Filter /
  Last Filter Settings reuse the pin list. The `field-blur` kind takes
  `[count, x, y, blur, …, bokeh …, noise …]`.
  `Filter ▸ Blur Gallery ▸ Iris Blur…` opens the same workspace in Iris
  mode: the Iris Blur row is active (Blur `0..=500 px`, default 15) and the
  others are disabled; the selected pin draws its outline with four radius
  handles (drag to resize that semi-axis and turn the outline), four
  feather dots (drag to set the sharp core), and the roundness diamond (drag
  to square the outline). The default iris is centred, 0.35 of the width
  across, proportioned like the picture, feather 0.65, squareness 50; a new
  pin copies the selected one's outline and amount. The `iris-blur` kind
  takes `[count, x, y, radius x, radius y, angle, squareness, feather, blur,
  …, bokeh …, noise …]`.
- **Ceilings.** The workspace runs one effect at a time (Field and Iris do
  not combine); an Iris feather dot moves all four together (no Alt-drag of
  one); the `M` mask view is not shipped; the Motion Effects tab only
  states that Path/Spin motion effects are unimplemented; pins are fractions
  of the document proxy while the commit filters the active layer, so a layer
  that does not cover the document previews approximately; no GPU path; Path
  Blur and Spin Blur stay post-CS6 non-goals.

## CS6 behavior

`Filter > Blur` then **Field Blur**, **Iris Blur**, or **Tilt-Shift**. The Help (both the PDF and the CS6-era Help page) is explicit that the gallery offers three distinct photographic blur effects and lists only those three. The workspace gives a **full-size, live preview** with **on-image overlay controls** (pins, handles, lines) instead of a modal slider-only dialog. Each blur can be applied alone or combined with the others.

Common CS6 workspace behaviour:

- **Pins.** A pin marks a blur location. Click the image to add pins; drag a pin to move it; press **Delete** to remove the selected pin. For Field Blur the final result is the **combined effect of all pins**; a pin may be placed **outside the image** to blur corners.
- **Blur handle / Blur Tools panel.** Drag the blur handle to increase/decrease the blur, or type a value in the **Blur Tools** panel.
- **Mask view (`M`).** Temporarily shows the blur mask: **dark = sharp, light = blurred**.
- **Hide UI (`H`).** Temporarily hides the on-canvas UI.

### Field Blur (CS6)

Builds a **gradient of blurs** by placing **multiple pins, each with its own blur amount**; the final result is the combined field of all pins. Useful for a graduated depth-of-field across several regions.

### Iris Blur (CS6)

Simulates a **shallow depth-of-field** independent of camera/lens. The default pin carries an ellipse with **A. sharp area, B. fade area, C. blur area**; drag the handles to redefine the areas and the blur handle to change amount. Multiple focus points (multiple pins) are supported — an effect the Help calls almost impossible with traditional camera technique.

### Tilt-Shift (CS6)

Simulates a **tilt-shift lens**: a band of sharpness then a fade to blur at the edges, used for a "miniature" look. Areas are **A. sharp, B. fade, C. blur**. Drag the lines to move them, drag the handles to rotate; multiple pins are allowed.

### Blur Effects panel (CS6 — only these three)

The Blur Effects panel takes a value for each of:

| CS6 Blur Effects control | Meaning (CS6 Help) |
|---|---|
| **Light Bokeh** | Brightens the out-of-focus/blurred areas. |
| **Bokeh Color** | Adds more vivid colour to lightened areas that are **not blown out to white**. |
| **Light Range** | Determines the range of tones affected by the Bokeh settings. |

### Post-CS6 additions — NOT CS6 parity

The task brief names Path Blur, Spin Blur, and "noise/color/edge glow" blur effects. The fetched CS6 sources contain **none** of these; they are **later Creative Cloud** additions. Recorded here so the boundary is explicit:

- **Path Blur** *(CC)* — motion blur along one or more drawn paths; overlay path with endpoints; controls include **Speed** (applied to all path blurs) and **Taper** (blur trails off gradually), a **Basic Blur / Rear Sync Flash** choice (Rear Sync Flash simulates a flash fired at the end of the exposure), and per-endpoint **End Point Speed**. *(Secondary source: Adobe Help snapshot, CreativePro.)*
- **Spin Blur** *(CC)* — radial blur around one or more points, measured in **Blur Angle 0–360°**, with an ellipse of centre points; supports **Strobe Strength**, **Strobe Flashes**, and **Strobe Flash Duration** (motion effects). *(Secondary source: Adobe Help snapshot, PCWorld.)*
- **Motion Blur Effects** *(CC)* — apply only to Path and Spin blurs (there must be a motion path/radius).
- **Noise effects** *(CC/later)* — add grain/texture back into blurred areas: reported controls include noise amount, size, roughness, colour, and highlights; a **Noise Distribution** (Uniform/Gaussian) and **Monochromatic** option are reported in later builds. Not found in CS6 sources.
- **Edge Glow** *(later; not found in CS6 sources)* — reported as a Blur Effects tab in modern Photoshop. Do **not** implement it as CS6 parity without a CS6 source; see `## Open questions`.
- **Smart filter support** — the photographic blur effects became Smart-Filter-capable **only in CC** ("Applying Blur gallery effects as smart filters | Creative Cloud only"). CS6 applies them **destructively**.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Filter > Blur > Field Blur` | workspace | — | Multiple pins, per-pin blur |
| `Filter > Blur > Iris Blur` | workspace | — | Ellipse: sharp/fade/blur areas |
| `Filter > Blur > Tilt-Shift` | workspace | — | Parallel sharp band → edge blur |
| Blur gallery — hide UI | overlay toggle | `H` | Temporarily hides on-canvas UI |
| Blur gallery — blur mask | overlay toggle | `M` | Dark = sharp, light = blurred |
| Blur gallery — blur handle | on-canvas drag | — | Drag to change blur amount |
| Blur Tools panel | docked panel | — | Numeric blur amount per effect |
| Blur Effects panel | docked panel | — | CS6: Light Bokeh, Bokeh Color, Light Range |
| Pin context | on-canvas | `Delete` | Remove selected pin; click image to add |
| `Edit > Fade Field/Iris/Tilt-Shift` | menu, dialog | `Shift+Ctrl/Cmd+F` *(inferred)* | Opacity + mode after commit |
| `Filter > Blur > Path/Spin Blur` | workspace | — | **Post-CS6 (CC) only** |

## Parameters & ranges

CS6 Help does **not** publish numeric ranges for the CS6 controls; the entries below marked *(unverified)* require a CS6 capture or a calibration experiment. Post-CS6 controls are listed separately and are not CS6 parity.

| Effect | Control | Type | Default | Range / options | CS6? |
|---|---|---|---|---|---|
| All | Pins | interactive | 1 default pin | any number | Yes |
| All | Blur amount | float px | *(unverified)* | *(unverified; ~0–500 px common)* | Yes |
| Field Blur | Per-pin blur amount | float px | *(unverified)* | per pin | Yes |
| Iris Blur | Ellipse dimensions | interactive | default ellipse | sharp / fade / blur regions | Yes |
| Iris Blur | Ellipse rotation | float ° | *(unverified)* | 0–360 | Yes |
| Tilt-Shift | Band position / width | interactive | default band | sharp / fade / blur regions | Yes |
| Tilt-Shift | Band angle | float ° | *(unverified)* | 0–360 | Yes |
| All | Light Bokeh | int % | *(unverified)* | *(unverified; ~0–100)* | Yes |
| All | Bokeh Color | int % | *(unverified)* | *(unverified; ~0–100)* | Yes |
| All | Light Range | int | *(unverified)* | *(unverified; tonal range ~0–255)* | Yes |
| Path Blur | Speed | float | — | applied to all paths | **No (CC)** |
| Path Blur | Taper | float | — | gradual trail-off | **No (CC)** |
| Path Blur | Basic Blur / Rear Sync Flash | enum | Basic | Basic / Rear Sync Flash | **No (CC)** |
| Path Blur | End Point Speed | float | — | per endpoint | **No (CC)** |
| Spin Blur | Blur Angle | float ° | — | 0–360 | **No (CC)** |
| Spin/Path | Strobe Strength / Flashes / Flash Duration | float/%/int/° | — | motion effects | **No (CC)** |
| All | Noise amount / size / roughness / colour / highlights | float | — | reported later builds | **No (CC/later)** |
| All | Noise Distribution | enum | — | Uniform / Gaussian (reported) | **No (CC/later)** |
| All | Monochromatic | bool | — | reported | **No (CC/later)** |
| All | Edge Glow | — | — | **not found in any CS6 source** | **No (unverified)** |

## Algorithms & pipeline

Behavioral parity only; Adobe's shaders are closed. Independent testing reported the gallery filters **require OpenCL 1.1** (see `## Sources`).

**Blur field construction** *(inferred)*. The gallery behaves like a depth-of-field renderer driven by a synthetic depth/weight field built from the placed pins, rather than a single uniform blur:

```text
# weights per pixel from pins (Field: multiple values; Iris/Tilt-Shift: region shapes)
for each pixel p:
    w(p) = combine_pin_weights(p, pins)         # e.g. inverse-distance / radial basis
    r(p) = lerp(r_min, r_max, w(p))             # blur radius
    out(p) = variable_radius_blur(src, p, r(p))
```

- **Field Blur** — `w` is an interpolation (inverse-distance / radial-basis-like) over the pin values, giving a smooth blur *gradient*; pins outside the image anchor the field at the borders.
- **Iris Blur** — the ellipse defines a **sharp** core, an annular **fade**, and a **blur** exterior; multiple pins generate multiple in-focus islands.
- **Tilt-Shift** — a band (two parallel lines) is sharp, with a symmetric fade to blur outside it; handle rotation sets the band angle.
- **Variable-radius blur / bokeh** *(inferred)* — a variable-radius blur, typically a disc/polygonal-kernel convolution for bokeh character rather than a pure Gaussian, so out-of-focus highlights stay "round" and bright.

**Blur Effects** *(inferred)*:

- **Light Bokeh** lifts luminance in blurred regions proportional to their brightness (an additive/highlight boost).
- **Bokeh Color** saturates lightened, non-clipped regions, roughly by increasing chroma in the bright/non-white range.
- **Light Range** gates both above to a tonal window (similar in spirit to a weight by luminance), so only a chosen band of tones is affected.

**Mask view (`M`)** renders the normalized field `w` as a grayscale overlay (dark = sharp, light = blurred) to explain exactly what the live blur is doing.

**GPU/OpenCL.** The GPU FAQ lists the gallery effects (under the labels scene blur, aperture blur, and tilt/offset) as **OpenCL-accelerated**; the CS6 beta notes call them accelerated by an OpenCL-compatible video card, and independent testing found the gallery needs **OpenCL 1.1**. Unlike Oil Paint, the gallery is listed as accelerated rather than "requires a compatible card"; a CPU fallback is the safe Linux default (see `## Open questions`).

## Rust module mapping

Proposals. The gallery is a GPU variable-blur pipeline with a small, serializable parameter set and an interactive overlay model.

- `pictura_filters::blur_gallery` — `BlurGalleryKind { Field, Iris, TiltShift }`, `BlurGalleryParams { pins: Vec<Pin>, bokeh: BokehParams, kind }`, `fn apply(&Document, Rect, &BlurGalleryParams, &mut Backend) -> FilterResult`.
- `pictura_filters::blur_gallery::field` — `fn build_weight_field(pins, kind, image_size) -> MaskBuffer`; interpolation kernel; pin-outside-image handling.
- `pictura_filters::blur_gallery::bokeh` — `struct BokehParams { light_bokeh: f32, bokeh_color: f32, light_range: (f32, f32) }`; highlight-lift and chroma post-pass.
- `pictura_filters::blur_gallery::kernels` — variable-radius disc/polygonal convolution; separable fast path where approximate.
- `pictura_render::blur_gallery` — `wgpu`/compute implementation; capability probe for OpenCL-class compute; CPU reference path.
- `pictura_filters::overlay` — geometry + hit-testing for pins, ellipse handles, and tilt-shift lines, shared with the Qt overlay.

Crossing types: `Pin { pos: Vec2f, blur: f32, kind_specific }`, `BokehParams`, `MaskBuffer`, `FilterResult`.

## Qt6 component mapping

The gallery is **canvas-centric and interactive**, so this is the one filter family where a QML overlay over a `QRhiWidget` preview is the better fit than a plain `QDialog`.

| Proposal | Base | Responsibility |
|---|---|---|
| `BlurGalleryWorkspace` | `QWidget`/`QRhiWidget` host | Full-size live preview + overlay; `H` hide-UI, `M` mask |
| `BlurGalleryOverlay.qml` | QML `Item` over the preview | Pins, iris ellipse + handles, tilt-shift lines; drag/add/delete |
| `BlurToolsPanel` | `QWidget`/QML panel | Numeric blur amount for the selected pin/effect |
| `BlurEffectsPanel` | `QWidget`/QML panel | CS6: Light Bokeh, Bokeh Color, Light Range |
| `PinModel` | `QAbstractListModel` | Pin list (position, blur, selection) shared with the overlay |
| `BlurGalleryDialog` | `QDialog` | Hosts the workspace for modal invocation; OK/Cancel/Fade |

Rationale: on-image editing with many hit-testable handles and a live GPU preview is exactly the interactive-canvas problem QML solves well; the existing app frame already embeds QML for canvas overlays (`ARCH-003`).

## Data-model impact

- **Destructive in CS6.** Applying from the Filter menu commits pixels to the active layer; one history state per commit; `Cancel` is a no-op.
- **Smart filters (CC only).** In CC the gallery is a smart filter; CS6 is not. Design the filter record to hold `BlurGalleryParams` (pin list + bokeh) so a smart-filter extension is possible (`LAY-021`).
- **No new document nodes.** The mask view is transient.
- **Serialization:** pin geometry and bokeh live in the filter `params_blob`; map to an XMP/PSD filter descriptor if smart-filter parity is pursued. The exact Adobe descriptor keys are unknown (see `## Open questions`).

## Edge cases

- **No supported GPU / missing OpenCL 1.1** — decide fallback: CPU path (safe Linux default) or gate the menu item. CS6 is GPU-accelerated; no confirmed hard requirement like Oil Paint.
- **8-bpc only?** — the CS6 16-/32-bpc filter lists exclude Field/Iris/Tilt-Shift, implying 8-bpc-only. Confirm; grey out on other depths if so.
- **Modes** — Bitmap/Indexed excluded; whether the gallery is RGB-only is unstated.
- **Pins outside the image** — Field Blur explicitly allows this to blur corners; the weight field must handle it.
- **Zero pins / deleted all pins** — treat as a no-op or re-seed one pin; do not crash.
- **Multiple pins** — Field combines all; Iris/Tilt-Shift compose multiple focus regions; the combine order must be deterministic.
- **Huge (PSB) documents** — full-size live preview is memory-heavy; use a downscaled preview while editing and render at full resolution on commit.
- **Selection** — confine processing to the selection (plus apron for the blur kernel).
- **Undo/redo** — one atomic state per OK.
- **Color management** — bokeh math (highlight lift, chroma) should be defined in the working space; Adobe's choice is undocumented.

## Parity acceptance criteria

1. Given a supported backend, Field Blur with N pins of equal amount produces a blur that increases from a pin toward the field midpoints and is monotonically related to per-pin amounts.
2. Given an Iris Blur pin, pixels inside the sharp ellipse are (within tolerance) unchanged, the fade annulus transitions smoothly, and the exterior is blurred; dragging handles moves the regions.
3. Given a Tilt-Shift band, pixels inside the sharp band are unchanged and blur increases with distance from the band; rotating the handles rotates the band.
4. Pressing `M` renders a mask consistent with the field (dark where sharp, light where blurred); `H` hides the overlay without changing the result.
5. Increasing **Light Bokeh** brightens blurred areas; increasing **Bokeh Color** raises chroma in lightened, non-clipped regions; **Light Range** restricts the effect to the selected tonal band.
6. Given a commit, exactly one history state appears and `Ctrl+Z` restores the pre-filter pixels bit-exactly.
7. Given a selection, pixels outside it are bit-identical.
8. On the CPU fallback, acceptance results match the GPU path within tolerance.
9. **CS6-boundary:** `Filter > Blur > Path Blur` and `Spin Blur` are **absent** in CS6-parity mode; if a CC-extension build adds them they are clearly labelled non-parity.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — official CS6 Help reference (downloaded to `/tmp`, `pdftotext`-extracted). Established: the page title "Photographic blur gallery | CS6" and its roster of three distinct photographic blur effects (Field Blur, Iris Blur, Tilt-Shift); Field/Iris/Tilt-Shift behaviour, pins, handles, A/B/C areas, `M` mask; the **Blur Effects** panel with exactly **Light Bokeh, Bokeh Color, Light Range**; smart-filter support marked **Creative Cloud only**; the CS6 new-feature shortcut list (`H` hide UI, `M` blur mask); the "Using the new three-part Blur Gallery" launch video title; the 16-/32-bpc lists exclude the gallery.
- `https://web.archive.org/web/20121116114821id_/http://helpx.adobe.com/photoshop/using/blur-gallery.html` — **CS6-era Adobe Help snapshot** (wayback; live `helpx` 403). Confirmed the page title "Photographic blur gallery | CS6", the three-effect roster, the per-effect instructions, and that the CS6 Blur Effects panel has only Light Bokeh / Bokeh Color / Light Range.
- `https://web.archive.org/web/20140701061120id_/https://helpx.adobe.com/photoshop/using/blur-gallery.html` — later (2014) Adobe Help snapshot. Used to source the **post-CS6** `Path Blur` and `Spin Blur` behaviour (Speed, Taper, Basic/Rear Sync Flash, End Point Speed; Blur Angle 0–360°, Strobe Strength/Flashes/Flash Duration).
- `https://topic.alibabacloud.com/a/photoshop-cs6-gpu-faq_8_8_10184243.html` — mirror of the **Photoshop CS6 GPU FAQ**. Established "Scene blur, aperture blur, and tilt/offset" are **OpenCL-accelerated**; Oil Paint/Adaptive Wide Angle require a compatible card.
- `https://photographyuncapped.com/adobe-photoshop-cs6-uses-opencl-opengl-features-of-gpu-to-accelerate-graphics-improved-performance/adobe-cs6` — CS6 beta notes listing Field Blur, Iris Blur, and Tilt/Shift as accelerated by an OpenCL-compatible video card; secondary.
- `https://www.pcworld.com/article/393280/how-to-use-photoshop-blur-gallery-for-bokeh-effects-and-more.html` — modern (2020) overview of the **five-effect** gallery and its Bokeh / Motion / Noise effects tabs; used only for the labelled **post-CS6** controls, secondary.
- `https://creativepro.com/photoshop-blur-gallery-path-blur` and `https://creativepro.com/photoshop-blur-gallery-spin-blur` — community articles on the post-CS6 Path/Spin blurs; secondary.
- SearXNG meta-search (queries: "Photoshop CS6 Blur Gallery Path Blur Spin Blur blur effects noise edge glow", "Photoshop CS6 GPU FAQ") — used to locate the above; no facts asserted from snippets alone.

Not parsed in this pass: `helpx.adobe.com` live pages (HTTP 403); Adobe's gallery shaders (proprietary).

## Open questions

- **CS6 numeric ranges/defaults.** The CS6 Help gives no min/max/default for blur amount, ellipse geometry, or the three bokeh sliders. Resolves with: a CS6 dialog capture and a calibration experiment.
- **Was the CS6 Blur Gallery hard-gated on OpenCL?** Independent testing says OpenCL 1.1 is required; the GPU FAQ says "accelerated." Determine whether CS6 **refuses** to open the gallery without it or merely runs slowly. Resolves with: a CS6 test on a non-OpenCL machine.
- **8-bpc-only confirmation** for CS6 (inferred from the depth lists). Resolves with: a CS6 depth test.
- **`Edge Glow`.** Named in the task brief but **not found in any fetched CS6 source**; likely a later Blur Effects tab. Do not treat as CS6 parity until a CS6 source establishes it. Resolves with: an Adobe CS6-era Blur Effects screenshot or Help page.
- **Noise controls.** "Noise effects" are post-CS6; the exact modern control set (amount/size/roughness/colour/highlights, distribution, monochromatic) is from a secondary overview and varies by version. Resolves with: a version-specific Adobe Help page.
- **Bokeh algorithm.** Whether the blur kernel is a disc, polygon, or a shaped aperture, and how Light Bokeh/Color/Range map to math, is undocumented. Resolves with: reference-image fitting against CS6 output.
- **Smart-filter serialization.** Adobe's PSD filter-descriptor keys for pin geometry/bokeh are unknown. Resolves with: a PSD with a Blur Gallery smart filter, parsed.
- **Performance budget** for full-resolution preview and commit on large documents. Belongs to `01-architecture/performance-targets.md`.
