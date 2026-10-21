# Gradient Presets and the Gradient Editor

- **Spec ID:** `CLR-010`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `Changed` — the Gradient Editor itself is long-standing; CS6 adds a **Dither** option to the Gradient Overlay and Gradient Stroke layer styles (JDI), ships **new Gradient Map presets** for print toning and split-toning, and keeps the gradient fill layer (Style/Angle/Scale/Reverse/Dither/Align With Layer) from CS5. The post-CS6 **interpolation-method** menu (Classic/Smooth/Perceptual) is *not* part of CS6.
- **Depends on:** `03-tools/gradient-and-paint-bucket.md` (`TOOL-012`), `05-layers/fill-layers.md`, `05-layers/layer-styles.md`, `04-image-ops/adjustments/gradient-map.md`, `02-ui-ux/panels/color-panel.md`, `01-architecture/document-model.md`, `10-workflow-io/presets-manager.md`, `07-color-painting/color-sweeps-and-modes.md`.

> Module and widget names below are **design proposals**. The repository
> contains a working implementation; this file stays the behavioural contract
> for the feature, and `docs/dev/doc-code-parity-gaps.md` records what shipped
> and what did not. Facts not confirmed by a fetched CS6/Adobe source are marked
> *(inferred)*.

## CS6 behavior

The **Gradient tool** creates "a gradual blend between multiple colors" from a
preset fill or a user-defined gradient. Source: CS6 reference, "Apply a gradient
fill", "Gradient Editor overview".

- **Invocation.** Select the Gradient tool (grouped behind the Paint Bucket in
  the toolbar, `G`); in the options bar click the triangle next to the wide
  gradient sample for presets, or click **inside** the sample to open the
  **Gradient Editor** (tool tip "Click to edit gradient").
- **Cannot be used with Bitmap or Indexed Color** documents (Help note). It also
  requires a layers-capable document, since it paints onto a layer.
- **Fill scope.** Fills the active selection, or the whole active layer when
  nothing is selected. Drag in the image to set the start and end points;
  `Shift` constrains the line angle to multiples of 45°.
- **Gradient types (options bar):**

  | Type | Behavior |
  |---|---|
  | Linear | Blends in a straight line from the start point to the end point. |
  | Radial | Blends outward from the start point in a circular pattern. |
  | Angle | Blends in a counterclockwise sweep around the start point. |
  | Reflected | Mirrors a linear blend on both sides of the start point. |
  | Diamond | Blends from the center out to the corners in a diamond pattern. |

- **Options bar:** blending mode, opacity, **Reverse**, **Dither**, and
  **Transparency** (use the gradient's opacity stops as a transparency mask).
  The **Neutral Density** preset is called out as a photographic filter.
- **Gradient Editor.** Defines a new gradient by modifying a copy of an
  existing one; supports intermediate colors. Components (Help figure):
  A. panel menu, B. opacity stop, C. color stops, D. value/delete fields,
  E. midpoint.
  - **Color stops** below the bar: click the left stop for the start color, the
    right stop for the end color; choose a color by double-clicking the stop,
    clicking the swatch, choosing from the **Color** pop-up, or sampling with
    the eyedropper (over the bar or anywhere in the image).
  - **Location** entered in the Stops section: `0%` at the far left, `100%` at
    the far right.
  - **Midpoint** diamond between two stops (the even-mix point) is draggable or
    typed.
  - Add an intermediate color by clicking below the bar; delete by **Delete** or
    by dragging the stop downward/off the bar.
  - **Smoothness** text box / pop-up slider controls "how gradual the
    transitions are between color bands".
  - **Name** and **New** save the gradient as a preset; new presets live in a
    Preferences file (lost if presets are reset) unless saved to a library.
- **Opacity (transparency) stops.** Each gradient carries opacity at locations
  along the bar (the checkerboard preview shows transparency). Click the
  left/right opacity stop above the bar, set **Opacity** and **Location**, move
  the midpoint, add intermediate stops by clicking above the bar, delete with
  **Delete** or by dragging the stop up and off the bar. (Source: "Specify the
  gradient transparency".)
- **Noise gradients.** Gradient Type = **Noise** produces "randomly distributed
  colors within the range you specify." Controls: **Roughness**, **Color Model**
  (RGB/HSB/Lab; each component gets a min/max slider range), **Restrict Colors**
  (prevents oversaturated colors), **Add Transparency**, and **Randomize**
  (re-roll); **Name** + **New** saves it. (Source: "Create a noise gradient".)
- **Named gradients and libraries.** Manage presets in the Gradient Picker,
  Preset Manager, or Gradient Editor. Save a library from **Save** (Editor) or
  **Save Gradients** (Picker menu); load with **Load** or **Replace Gradients**;
  a library placed in `Presets/Gradients` appears at the panel-menu bottom after
  restart; **Reset Gradients** restores the default library (replace or append).
  Display options: Text Only / Small or Large Thumbnail / Small or Large List;
  rename by double-clicking in thumbnail/list modes. (Source: "Manage gradient
  presets".)
- **Consumers of gradients.** The same gradient object is used by: the Gradient
  tool; the **Gradient Fill layer** (`Layer > New Fill Layer > Gradient`, with
  Style/Angle/Scale/Reverse/Dither/Align With Layer and image-drag repositioning);
  the **Gradient Map** adjustment (Dither, Reverse); and the **Gradient Overlay**
  / **Gradient Stroke** layer styles (Style, Angle, Scale, Reverse, Align With
  Layer, Center-drag; Dither added in CS6).

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Toolbox (`G` group) | Gradient tool | `G` | Grouped with Paint Bucket |
| Options bar | Gradient sample + preset pop-up | — | Click sample → Gradient Editor |
| Options bar | Gradient type | — | Linear / Radial / Angle / Reflected / Diamond |
| Options bar | Mode / Opacity | — | Paint blending and alpha |
| Options bar | Reverse | — | Flip stop order |
| Options bar | Dither | — | Reduce banding |
| Options bar | Transparency | — | Use opacity stops as mask |
| Gradient Editor | Presets list | — | Select base gradient |
| Gradient Editor | Gradient Type | — | Solid / Noise |
| Gradient Editor | Color stops + swatch + Color menu | — | Start/intermediate/end colors |
| Gradient Editor | Opacity stops | — | Start/intermediate/end opacity |
| Gradient Editor | Location / Opacity / Midpoint / Smoothness | — | Numeric fields + sliders |
| Gradient Editor | Name + New / Delete | — | Save/destroy preset |
| Gradient Editor | Load / Save / panel menu | — | .grd library I/O |
| Noise section | Roughness / Color Model / Restrict / Add Transparency / Randomize | — | Noise gradient controls |
| Gradient Picker menu | Load / Replace / Save / Reset / display / rename | — | Library management |
| `Layer > New Fill Layer > Gradient` | Fill layer dialog | — | Style/Angle/Scale/Reverse/Dither/Align |
| `Layer > Layer Style > Gradient Overlay` | Layer style dialog | — | + Dither (CS6) |
| `Layer > Layer Style > Stroke` (Gradient) | Layer style dialog | — | + Dither (CS6) |
| `Image > Adjustments > Gradient Map` / Properties | Adjustment | — | Dither, Reverse |
| Preset Manager | Dialog | — | Cross-preset management |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Gradient Type | enum | Solid | Solid / Noise | In Editor |
| Gradient geometry | enum | Linear | Linear / Radial / Angle / Reflected / Diamond | Tool options bar |
| Color stop Location | percent | 0% / 100% | 0–100% | Stored 0–4096 in .grd |
| Color stop color | color | per preset | Any picker color + foreground/background stop types | See .grd below |
| Midpoint | percent | 50% *(inferred)* | 0–100% | Between adjacent stops |
| Opacity stop Opacity | percent | 100% | 0–100% | Transparency stops |
| Opacity stop Location | percent | 0% / 100% | 0–100% | |
| Smoothness | percent | 100% *(inferred)* | 0–100% | .grd `Intr` 0–4096 |
| Reverse | bool | Off | On/Off | Tool, fill layer, map, styles |
| Dither | bool | Off | On/Off | Tool, fill layer, map, styles |
| Transparency | bool | On *(inferred)* | On/Off | Tool only |
| Blending mode | enum | Normal | 27 CS6 modes | Tool |
| Opacity | percent | 100% | 0–100% | Tool |
| Fill-layer Angle | degrees | 90° *(inferred)* | −180…+180 | Fill layer / styles |
| Fill-layer Scale | percent | 100% | 10–1000% *(inferred)* | Fill layer / styles |
| Fill-layer Style | enum | Linear | Same 5 geometries | Fill layer / styles |
| Align With Layer | bool | On *(inferred)* | On/Off | Fill layer / styles |
| Noise Roughness | percent | 50% *(inferred)* | 0–100% | Noise editor |
| Noise Color Model | enum | RGB | RGB / HSB / Lab | Noise editor |
| Noise min/max per component | 4 values | full range | 0–100% each + transparency | .grd `Mnm`/`Mxm` |
| Restrict Colors | bool | Off | On/Off | Noise editor |
| Add Transparency | bool | Off | On/Off | Noise editor |
| Random seed | int | random | rerolled by Randomize | .grd `RndS` |

Defaults marked *(inferred)* are not stated in the fetched Help.

## Algorithms & pipeline

Adobe's exact code is closed. The following is a behavioural-parity model
*(inferred)* except where the Help defines the observable behavior.

### Solid gradient evaluation

1. Define `t ∈ [0,1]` along the geometry from the start point to the end point
   (or around the center for Angle/Radial/Diamond/Reflected).
2. Map `t` to the gradient parameter, applying each stop's **midpoint** as a
   piecewise bias between the two adjacent stops (a midpoint of 50% gives a
   symmetric blend; other values warp the transition). The exact bias curve is
   *(inferred)*; a common model is a power/`smoothstep` remap.
3. Interpolate color between the bracketing color stops. CS6 interpolates in the
   document working space; whether it is straight RGB, linear-RGB, or
   premultiplied alpha is unverified. Alpha comes from the opacity stops, which
   are interpolated independently and multiplied in.
4. **Smoothness** controls the falloff near each stop; 100% is the smoothest
   (documented as "how gradual the transitions are"). Its exact mapping to the
   0–4096 `Intr` value in `.grd` is *(inferred)*.
5. Geometry variants: Linear = `t` along segment; Radial = normalized distance
   from start; Angle = `atan2` sweep (counterclockwise); Reflected = `|t|`
   mirrored about the start; Diamond = `(|x|+|y|)`-style Manhattan distance to
   the corners.
6. **Dither** adds random noise to the result to mask banding (the Help describes it as adding noise that smooths the appearance and reduces banding). The
   exact noise distribution and whether it is ordered vs error-diffusion is
   *(inferred)*; behavior TBD.

### Noise gradient

Generate a 1-D color ramp by sampling a stochastic function of the gradient
parameter, seeded by `RndS`, constrained to the component ranges given by
`Mnm`/`Mxm` in the selected Color Model. **Roughness** controls correlation
length (higher roughness = banded/coarser; lower = smoother). **Restrict Colors**
clamps/clamps-saturation so no component exceeds its range. **Add Transparency**
inserts random alpha. The precise noise basis (value noise, perlin, white noise)
and its frequency mapping are *(inferred)*.

### Placement in the pipeline

- Gradient tool / fill layer write pixels (or a non-destructive fill layer) via
  the normal paint pipeline (blend mode, opacity, mask, Preserve Transparency).
- Gradient Map is a per-pixel luminance→gradient lookup in the adjustment
  pipeline.
- Gradient Overlay / Gradient Stroke render in layer-style space (scale/align
  relative to the layer bounding box unless Align With Layer is off).

## Rust module mapping

Proposed:

- `pictura_color::gradient` — `Gradient` enum (`Solid { stops, opacity }` /
  `Noise { params, seed }`), `ColorStop { location: f32, midpoint: f32, color:
  StopColor }`, `OpacityStop { location, midpoint, opacity }`,
  `StopColor::{Rgb, Hsb, Lab, Cmyk, Gray, Foreground, Background}`.
- `pictura_color::gradient::eval` — `sample(gradient, t, space) -> PremulRgba`;
  geometry adapters in `pictura_ops::paint::gradient`.
- `pictura_ops::paint::gradient` — `GradientTool { geometry, settings }`,
  applying the gradient to a tile/selection through a `PaintSink`.
- `pictura_ops::gradient_noise` — seeded generator with component-range clamping.
- `pictura_io::grd` — `.grd` (8BGR v5) reader/writer via the ActionDescriptor
  subset; also reads the `8BPF` preferences gradients file.
- Boundary types: `Rect`, `ColorSpace`, `PremulRgba`, `Gradient`,
  `GradientGeometry`, `GradientSettings`.

## Qt6 component mapping

- `GradientPickerButton` (`QToolButton`) — wide gradient sample, opens a
  `QMenu`-based preset pop-up.
- `GradientEditorDialog` (`QDialog`) — preset list, gradient bar with stop
  handles, Stops/Location/Opacity/Smoothness fields, Solid/Noise page.
- `GradientBarWidget` (custom `QWidget`) — draws the bar and checkerboard and
  handles stop add/move/delete; emits `gradientChanged(Gradient)`.
- `GradientNoiseEditor` (`QWidget`) — Roughness, Color Model, min/max sliders,
  Restrict Colors, Add Transparency, Randomize.
- `GradientFillLayerPage` / `GradientOverlayPage` (`QWidget`) — Style, Angle,
  Scale, Reverse, Dither, Align With Layer, on-canvas center drag.
- `GradientMapEditor` (`QWidget`) — gradient sample + Dither/Reverse, in the
  Properties stack.

Widgets rather than QML: these are dense docked forms matching the widget shell
*(design decision)*.

## Data-model impact

- **Gradient object** is shared (preset, fill layer, adjustment layer, style).
  Serialize as the `.grd` descriptor form so it round-trips with Adobe.
- **Fill layer / adjustment layer / style** payloads reference a gradient by
  value or by document-local preset id; CS6 embeds the definition so a document
  is self-contained.
- **PSD serialization.** Gradient Map uses an `Lr16`/`Lr32`-style additional
  layer block; fill layers and gradient styles use descriptor-based blocks. The
  exact tags are *(inferred)* and belong in `01-architecture/file-formats.md`.
- **Undo.** Editing the gradient in the Editor is a preset operation (not
  document history) until applied; applying a gradient is one history state;
  fill-layer/style edits are individual states.

## Edge cases

- **Bitmap / Indexed**: the Gradient tool is unavailable (Help note); fill
  layers/adjustments are also unavailable in modes without layers.
- **8/16/32-bit**: gradient sampling must run in `f32` and quantize once at the
  document depth; 32-bit must not clip between stops.
- **CMYK / Lab / Grayscale / Duotone**: interpolate in the working space; CMYK
  and Lab stops stored in `.grd` must convert deterministically.
- **Foreground/Background color stops**: stops typed as foreground/background
  reference live toolbox colors and must update when those change.
- **Transparency off**: alpha ignored, gradient painted opaque.
- **Dither**: must not change the mean color (only its distribution); verify it
  reduces visible banding on an 8-bit shallow ramp.
- **1-px / empty / huge (PSB)**: sample per tile; no full-canvas scratch.
- **GPU unavailable**: CPU evaluation is the reference; the GPU path must match
  within tolerance.
- **Noise gradient random seed**: persisted so a saved preset reproduces exactly.

## Parity acceptance criteria

1. Given a default black→white Linear gradient across a 256-px strip, sampled
   values are monotonically non-decreasing and endpoints are exactly the stop
   colors within 1 LSB.
2. Given a mid stop at Location 50% with midpoint 50%, the 50% sample is the
   stop color; moving the midpoint shifts the mix point accordingly.
3. Given `Reverse` on, the gradient is the stop-order reversal of the same
   gradient with `Reverse` off.
4. Given an opacity stop at 50%, the corresponding midpoint pixel has alpha
   equal to that stop opacity within 1 LSB.
5. Given `Dither` on, an 8-bit shallow ramp shows reduced banding and the mean
   per column is unchanged within tolerance versus `Dither` off.
6. Given a Radial/Angle/Reflected/Diamond selection, the iso-value contours
   match the documented geometry.
7. Given a Gradient Fill layer, Style/Angle/Scale/Align With Layer/center-drag
   reproduce the previewed result, and the layer mask confines it.
8. Given a Gradient Map adjustment with a two-stop gradient, image shadows map
   to the first stop and highlights to the last within tolerance.
9. Given a saved `.grd` library, loading it appends/replaces presets and Reset
   Gradients restores the default set; a round-trip save/reload preserves stops,
   midpoints, smoothness, and noise parameters.
10. Given a noise gradient with a fixed seed, Randomize produces a new seed and
    the same seed reproduces the same ramp.

## Sources

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  official CS6 Help reference, downloaded and text-extracted. Establishes:
  "Apply a gradient fill" (tool grouping, preset picker vs Editor, five
  geometry types, Reverse/Dither/Transparency, 45° constraint, Neutral Density
  preset, Bitmap/Indexed limitation); "Manage gradient presets" (Save/Load/
  Replace/Reset, Presets/Gradients folder, display options, rename);
  "Gradient Editor overview" (dialog components, intermediate colors);
  "Create a smooth gradient" (color stops, Location, midpoint, add/delete,
  Smoothness, Name/New, preset-loss warning); "Specify the gradient
  transparency" (opacity stops, checkerboard, Location, midpoint, add/delete);
  "Create a noise gradient" (Roughness, Color Model, Restrict Colors, Add
  Transparency, Randomize); "Create a fill layer" (Gradient fill layer
  Style/Angle/Scale/Reverse/Dither/Align With Layer); "Layer style options"
  (Gradient and Dither semantics, Align With Layer, Scale, center drag);
  "Apply a gradient map to an image" (Dither, Reverse); JDI "Layers" (Added
  dither option to Gradient Overlay and Gradient Stroke layer styles) and
  "Presets" (Added new Gradient Map presets for traditional print toning and
  split-toning).
- `https://raw.githubusercontent.com/Investigamer/json-photoshop-scripting/refs/heads/master/Documentation/Photoshop-Gradients-File-Format/README.md`
  — community analysis (Michel Mariani, 2012) of the `.grd`/`8BPF`
  format: magic `8BGR`/`8BPF`, version 5, descriptor `GrdL` list, custom-stops
  gradient (`GrdF`/`CstS`, `Intr` 0–4096, `Clrs`, `Trns`), color stop (`Clrt`:
  `Lctn`, `Mdpn`, `Type` user/foreground/background, `Clr` color models),
  transparency stop (`TrnS`: `Lctn`, `Mdpn`, `Opct`), color-noise gradient
  (`GrdF`/`ClNs`, `RndS`, `ShTr`, `VctC`, `Smth`, `ClrS`, `Mnm`, `Mxm`), and the
  color objects (RGB/HSB/Lab/CMYK/Grayscale/Book). Used for the Data-model and
  Rust mapping sections; not an Adobe-published spec.

## Open questions

- **CS6 interpolation space**: does `Intr`/Smoothness operate in RGB, linear
  RGB, or premultiplied alpha, and is the midpoint curve power or smoothstep?
  *Resolves with:* a controlled two-stop ramp captured from CS6.
- **Dither algorithm**: noise type, amplitude, and whether it is ordered or
  error-diffusion. *Resolves with:* pixel-diff of a dithered vs undithered ramp.
- **Exact geometry parameterization** for Angle/Radial/Reflected/Diamond.
  *Resolves with:* CS6 captures of each type.
- **Defaults not in Help**: initial Smoothness, fill-layer/style Angle and Scale,
  Transparency toggle, noise Roughness. *Resolves with:* a first-run CS6 UI
  capture.
- **PSD keys** for gradient fill layers, gradient maps, and gradient styles.
  *Resolves with:* `01-architecture/file-formats.md` plus CS6-saved samples.
- **Foreground/background stop representation** in PSD vs `.grd`. *Resolves
  with:* a saved document using such a gradient.
- **Post-CS6 interpolation-method menu** — confirm it is absent in CS6.
  *Resolves with:* a CS6 Gradient Editor screenshot.
