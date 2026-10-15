# Blur Gallery (Photographic Blur Gallery)

- **Spec ID:** `FILT-092`
- **Status:** `Partial` — Field Blur, Iris Blur, Tilt-Shift, the post-CS6
  Path Blur (by owner decision, non-parity), combining them, and the shared
  workspace are implemented (see `## Implementation status`); the `M` mask
  view and Spin Blur are not.
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

Field Blur, Iris Blur, Tilt-Shift, their Blur Effects, and the shared Blur
Gallery workspace are implemented. The Noise tab and Path Blur are post-CS6
additions ported on the owner's request (issue #462 for Path Blur, from the
owner's CC screenshots) and are **not CS6 parity**; Spin Blur is not
implemented.

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
- **Tilt-Shift engine.** `pictura_filters::blur::tilt` (`TiltPin { x, y,
  angle, focus_above, focus_below, fade_above, fade_below, distortion,
  symmetric, blur }`, `tilt_shift`, `MAX_TILT_EXTENT = 10`): each pin is a
  band through its centre at `angle`; the four extents are fractions of the
  buffer height measured across the band (centre line → solid line → dashed
  line, each side separately). The falloff is 0 within the focus extent, a
  smoothstep across the fade, and 1 beyond — the Help's A/B/C sharp, fade,
  and blur areas. Pins multiply falloffs and blend amounts as Iris does.
  **Distortion** (−100..100 %) smears the blurred zone below the band (both
  zones with **Symmetric Distortion**) along the radius from the pin when
  positive and around it when negative, over up to twice the local radius
  (at most 33 bilinear taps), between the pyramid and the effects passes
  (`variable_blur` is now `pyramid_blur` + `apply_effects`, byte-identical
  for Field and Iris). The falloff, the combine, and the smear model are
  inferred.
- **App.** `Filter ▸ Blur Gallery ▸ Field Blur…` opens `BlurGalleryDialog`
  (with `BlurGalleryCanvas`): a document proxy with draggable circle pins
  (click adds, drag moves, the selected pin's ring changes its amount, each
  ring's white arc shows the blur amount — full at 100 px, Delete removes,
  `H` hides the UI), the Blur Tools panel with the selected pin's
  Blur (`0..=500 px`), the other effects' rows (see **Combined effects**),
  a disabled Spin Blur row, an Effects / Motion Effects / Noise tabbed panel (Bokeh
  with Light Bokeh, Bokeh Color, and the two-handle Light Range; Noise with
  the Gaussian/Uniform/Grain picker, Amount, Size, Roughness, Color,
  Highlights), Preview / Cancel / OK, and one history state on commit. The
  `Blur Gallery`
  submenu also lists a disabled `Spin Blur…` stub; its placement follows the later Photoshop menu (the
  owner's screenshot) rather than CS6's `Filter ▸ Blur` listing. Last Filter /
  Last Filter Settings reuse the pin list. The `field-blur` kind takes
  `[count, x, y, blur, …, bokeh …, noise …]`.
  `Filter ▸ Blur Gallery ▸ Iris Blur…` opens the same workspace in Iris
  mode: the Iris Blur row is on and selected (Blur `0..=500 px`, default
  15); the selected pin draws its outline with four radius
  handles (drag to resize that semi-axis and turn the outline), four
  feather dots (drag to set the sharp core), and the roundness diamond (drag
  to square the outline). The default iris is centred, 0.35 of the width
  across, proportioned like the picture, feather 0.65, squareness 50; a new
  pin copies the selected one's outline and amount. The `iris-blur` kind
  takes `[count, x, y, radius x, radius y, angle, squareness, feather, blur,
  …, bokeh …, noise …]`.
  `Filter ▸ Blur Gallery ▸ Tilt-Shift…` opens the workspace in Tilt-Shift
  mode: the Tilt-Shift row is active with Blur (`0..=500 px`, default 15),
  Distortion (`−100..=100 %`, default 0), and Symmetric Distortion; the
  selected pin draws its two solid focus lines and two dashed fade lines
  across the picture and a rotation dot on each solid line (drag a solid
  line to set that side's sharp area — its dashed line stays put until
  pushed; drag a dashed line to set the fade; drag a dot to turn the band).
  Hovering a line shows a resize arrow across the band and a dot the rotate
  arrow. While a pin, handle, line, or amount slider is dragged the preview
  renders a ≤ 360 px draft (~26 ms vs ~213 ms for the 1100 px proxy at Blur
  176) and the full proxy on release, in every gallery mode. The preview
  filters only the active layer and draws it over the document with that
  layer hidden, so lower layers never blur with it; the blur (and the
  distortion smear) runs over the layer's alpha too, so its edges feather
  into transparency as in Photoshop, while bokeh and noise stay
  colour-only.
  The default band is horizontal through the centre, focus 0.1 and fade
  0.15 of the picture's height each way (read off the owner's screenshots;
  unverified). The `tilt-shift` kind takes `[count, x, y, angle, focus
  above, focus below, fade above, fade below, distortion, symmetric, blur,
  …, bokeh …, noise …]`.
- **Empty target.** Every Blur Gallery command refuses an active layer with
  no non-transparent pixel inside the selection (or at all) before the
  workspace opens, with "Could not complete the `<command>` command because
  the initial bounding rectangle is empty." (the owner's Photoshop
  screenshot) and no history state; the Blur family shares the rule
  (`FILT-010`).
- **Combined effects.** Every implemented Blur Tools row is selectable: its
  name selects the effect (switching it on with its default pin, expanding
  it, bold while selected, the canvas editing its pins), its checkbox
  switches it on or off (pins kept), its arrow only expands. Each effect
  keeps its own pins. Several effects on blur together in one pass —
  `pictura_filters::blur::gallery` (`Filter::BlurGallery { field, iris,
  tilt, bokeh, noise }`) takes each pixel's largest radius of the per-effect
  fields (inferred; Adobe's combine is closed), then Tilt-Shift's smear and
  the effects; one effect alone is byte-identical to its own kernel. OK
  commits the one effect's own kind and label, or the `blur-gallery` kind
  ("Filter: Blur Gallery", `[field count, records…, iris count, records…,
  tilt count, records…, bokeh …, noise …]`) for several, which Last Filter
  and Last Filter Settings reuse; nothing on commits nothing.
- **Path Blur engine (post-CS6, non-parity).** `pictura_filters::blur::path`
  (`BlurPath { points, start_speed, end_speed }`, `PathMotion { speed,
  taper, centered, rear_sync, strobe_strength, strobe_flashes }`,
  `path_blur`, `MAX_PATH_POINTS = 32`, `MAX_PATH_SPEED = 500`,
  `MAX_END_POINT_SPEED = 500`): each path is the Catmull-Rom curve through
  its points; a pixel's motion vector is the inverse-square-distance blend
  of every path's nearest-point tangent times the end-point speeds
  interpolated by arc length, times Speed / 100, read from a grid of at most
  512 cells on the long edge. The pixel averages up to 64 bilinear taps
  along the field's streamline over that length — half behind and half
  ahead with **Centered Blur**, all behind otherwise (so a subject smears
  forward) — weighted by **Taper** (`1 − taper·u` toward the far ends), the
  **Strobe** flashes (evenly spaced, mixed in by Strobe Strength), and
  **Rear Sync Flash** (the pixel's own tap gains the trail's whole weight
  again). The gather covers alpha; Noise follows, weighted by trail length;
  Bokeh does not apply to paths (inferred). In a combined pass the paths
  blur what the radius effects left, Bokeh stays gated by the radius
  weights, and Noise takes the larger of the radius and path weights. All of
  this is inferred; Adobe's model is closed.
- **Path Blur workspace.** `Filter ▸ Blur Gallery ▸ Path Blur…` opens the
  workspace in Path mode: the Path Blur row (Basic Blur / Rear Sync Flash,
  reading Custom while Strobe Strength is above 0; Speed `0..=500 %`,
  default 50; Taper `0..=100 %`; Centered Blur, default on; End Point Speed
  `0..=500 px`, enabled only while an end point is selected; a disabled Edit
  Blur Shapes) and a live Motion Effects tab (Strobe Strength `0..=100 %`,
  default 0; Strobe Flashes `1..=100`, default 4). It opens on one path from
  the centre running right 0.08 of the width with both end speeds 0.04 of
  the document width (the owner's opening screenshot at 100 %). Paths draw as
  the curve through their points with a dot per point and an end arrow. As
  in the owner's CC walkthrough, a click on empty picture pins a new path's
  first stop and the curve then follows the pointer, each click pinning
  another stop; a double-click, Enter, Esc, or a click on the last stop ends
  it (ended on its first stop it runs right the opening length). Hovering
  empty picture or a curve shows a pushpin-with-plus cursor. Dragging a
  point moves it, pressing the curve inserts a point, Ctrl-drag moves the
  whole path, and Delete removes the selected point while its path keeps
  two, else the selected path while another remains (inferred). While Path
  Blur is selected the Effects (Bokeh) tab is disabled and Motion Effects
  enabled, and the reverse for the other effects. The Noise checkbox
  defaults on in every gallery mode, as in the owner's screenshots; Amount
  0 adds nothing. In every mode the workspace cannot be squeezed below what
  its side column needs, so every slider and value box keeps its height.
  One Blur Tools row is open at a time: opening a row (its arrow, or
  selecting its effect) closes the open one, so the floor stays at most
  ~754 px, Path Blur's row being the tallest. The `path-blur` kind takes `[count, (point
  count, (x, y)×point count, start speed, end speed)…, speed, taper,
  centered, rear sync, strobe strength, strobe flashes, bokeh …, noise …]`
  (27 canonical); `blur-gallery` appends the path count, records, and motion
  slots after the Tilt-Shift section (51 canonical).
- **Ceilings.** The canvas shows only the selected effect's pins; the distortion is a line-sampled smear, not
  Adobe's shader; an Iris feather dot moves all four together (no Alt-drag of
  one); the `M` mask view is not shipped; Edit Blur Shapes is not built
  (each end's blur shape follows the path tangent, only its length is
  editable); Path Blur's motion field, streamline gather, flash weights,
  ranges, and gestures are inferred; pins are drawn
  as fractions of the document while the preview and the commit place them
  as fractions of the active layer's bounds, so on a layer that does not
  cover the document the overlay sits off the effect it controls; no GPU path; Spin
  Blur stays a post-CS6 non-goal.

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

Simulates a **tilt-shift lens**: a band of sharpness then a fade to blur at the edges, used for a "miniature" look. Areas are **A. sharp, B. fade, C. blur**. Drag the lines to move them, drag the handles to rotate; multiple pins are allowed. The Blur Tools panel carries **Blur** (px), **Distortion** (%; shapes the blur in the bottom blur zone), and **Symmetric Distortion** (applies the distortion from both directions) *(owner's screenshots of a later Photoshop; unverified against CS6)*.

### Blur Effects panel (CS6 — only these three)

The Blur Effects panel takes a value for each of:

| CS6 Blur Effects control | Meaning (CS6 Help) |
|---|---|
| **Light Bokeh** | Brightens the out-of-focus/blurred areas. |
| **Bokeh Color** | Adds more vivid colour to lightened areas that are **not blown out to white**. |
| **Light Range** | Determines the range of tones affected by the Bokeh settings. |

### Post-CS6 additions — NOT CS6 parity

The task brief names Path Blur, Spin Blur, and "noise/color/edge glow" blur effects. The fetched CS6 sources contain **none** of these; they are **later Creative Cloud** additions. Recorded here so the boundary is explicit. By owner decision, the Noise tab and **Path Blur** are implemented anyway as labelled non-parity extensions (see `## Implementation status`); the rest stay out of scope.

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
| `Filter > Blur Gallery > Path Blur` | workspace | — | **Post-CS6 (CC)**; implemented by owner decision, non-parity |
| `Filter > Blur Gallery > Spin Blur` | workspace | — | **Post-CS6 (CC) only**; disabled stub |

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
| Tilt-Shift | Distortion | int % | 0 *(screenshot)* | −100–100 *(screenshot)* | Yes |
| Tilt-Shift | Symmetric Distortion | bool | off *(screenshot)* | on / off | Yes |
| All | Light Bokeh | int % | *(unverified)* | *(unverified; ~0–100)* | Yes |
| All | Bokeh Color | int % | *(unverified)* | *(unverified; ~0–100)* | Yes |
| All | Light Range | int | *(unverified)* | *(unverified; tonal range ~0–255)* | Yes |
| Path Blur | Speed | int % | 50 *(screenshot)* | 0–500 *(unverified)*; applied to all paths | **No (CC)** |
| Path Blur | Taper | int % | 0 *(screenshot)* | 0–100 *(unverified)*; gradual trail-off | **No (CC)** |
| Path Blur | Centered Blur | bool | on *(screenshot)* | on / off | **No (CC)** |
| Path Blur | Basic Blur / Rear Sync Flash | enum | Basic | Basic / Rear Sync Flash (Custom while strobing) | **No (CC)** |
| Path Blur | End Point Speed | int px | — | 0–500 *(unverified)*; per endpoint | **No (CC)** |
| Path Blur | Strobe Strength / Strobe Flashes | int % / int | 0 / 4 *(screenshot)* | 0–100 / 1–100 *(unverified)* | **No (CC)** |
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
- **Empty target** — an active layer with nothing opaque in the selection (a new empty layer) refuses with "Could not complete the `<command>` command because the initial bounding rectangle is empty." (owner's Photoshop screenshot) before the workspace opens; no history state.
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
9. **CS6-boundary:** `Spin Blur` is **absent**; `Path Blur` is present only as a labelled non-parity extension (owner decision, #462): its row tooltip and workspace hint mark it post-CS6, and nothing claims CS6 parity for it.

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
