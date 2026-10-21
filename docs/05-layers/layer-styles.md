# Layer Styles (fx)

- **Spec ID:** `LAY-011`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `Changed` — CS6 adds a **dither** option to Gradient Overlay and Gradient Stroke; **reorders** the effects list to the order in which they are applied (Drop Shadow moved below the other effects); adds **Rasterize Layer Style** as a one-step alternative to `Create Layers`; shows a **Blend If / Blending Options badge** on a layer when advanced blending is customized; and lets you change blend mode/lock/color label for multiple selected layers at once. A community CS6 reference also claims CS6 added **layer styles on groups**, but the CS6 Help says groups are excluded — see `## Open questions`. The effect set and formulas are unchanged from CS5.
- **Depends on:** `01-architecture/document-model.md` (`ARCH-008`), `01-architecture/undo-history.md` (`ARCH-009`), `01-architecture/gpu-rendering-pipeline.md`, `01-architecture/color-management.md` (`ARCH-007`), `05-layers/blend-modes.md` (`LAY-010`), `05-layers/vector-masks-and-clipping-masks.md`, `05-layers/layer-masks.md`, `07-color-painting/pattern-presets.md`, `07-color-painting/gradient-presets.md`.

> Module/crate/widget names are **design proposals**. The repository contains a
> working implementation; this file stays the behavioural contract for the
> feature, and `docs/dev/doc-code-parity-gaps.md` records what shipped and what
> did not. The CS6 Help PDF gives effect semantics and control *names* but almost
> never numeric ranges; PSD field units come from the Adobe File Formats
> Specification. Range/default values that are not in a fetched source are marked
> *(inferred)* or `not stated`, and flagged in `## Open questions`.

## CS6 behavior

A **layer effect** is a procedural decoration attached to a layer's transparency;
a **layer style** is one or more effects applied together. Effects are linked to
layer content: moving or editing the layer re-renders the effects. A layer with a
style shows an **fx** badge to the right of its name; clicking the triangle expands
the effect list in the Layers panel, and double-clicking an effect reopens it for
editing. A style can be saved as a **preset** and applied from the **Styles**
panel. Effects re-render live as the layer (pixels, shape geometry, or type)
changes.

The seven effect families and what they do (CS6 Help, "Layer Style dialog box
overview"):

| Effect | Behavior |
|---|---|
| **Drop Shadow** | Adds a shadow that falls behind the layer's content, offset by `Distance`/`Angle`. |
| **Inner Shadow** | Adds a shadow just inside the content edges, giving a recessed look. |
| **Outer Glow** | Adds a glow emanating from the outside edges. |
| **Inner Glow** | Adds a glow from the inside edges or the center (Source). |
| **Bevel & Emboss** | Adds combinations of highlight and shadow, with optional separate edge Contour and Texture; styles are Inner/Outer/Pillow/Stroke Bevel and Emboss. |
| **Satin** | Interior shading that produces a satin finish. |
| **Color / Gradient / Pattern Overlay** | Fills the layer content with a color, gradient, or pattern. |
| **Stroke** | Outlines the content with a color, gradient, or pattern; especially useful on hard-edged shapes and type. |

Bevel & Emboss contains two optional sub-panels: **Contour** (a separate contour
that shapes the bevel edge) and **Texture** (an embossed pattern applied to the
surface). The Contour sub-panel is indented under Bevel & Emboss exactly as its
"Contour" checkbox, and Texture follows.

### Advanced Blending (per-layer, in Blending Options)

- **Blend Interior Effects As Group** — apply the layer's blend mode to interior
  effects (Inner Glow, Satin, Color/Gradient/Pattern Overlay, Stroke Emboss).
  Off by default. See `LAY-010`.
- **Blend Clipped Layers As Group** — apply the base layer's mode to all clipped
  layers (default on).
- **Transparency Shapes Layers** (default on), **Layer Mask Hides Effects**,
  **Vector Mask Hides Effects**, per-channel **Include** checkboxes, and the
  **Blend If** sliders.
- **Fill With (Mode)-Neutral Color** on new layers (see `LAY-010`).

### Global light

`Layer > Layer Style > Global Light` sets one master **Angle** and **Altitude**
used by every effect that has `Use Global Light` selected (Drop Shadow, Inner
Shadow, Bevel & Emboss). Turning `Use Global Light` off makes an effect's angle
local. The global angle is stored as a document-level image resource; default
global angle is `30°` (`ARCH-008`; resource 1037).

### Copying, scaling, converting, presets

- **Copy Layer Style / Paste Layer Style** (`Layer > Layer Style`) copies the whole
  style to the target and *replaces* the target's style. Dragging a single effect
  (or the whole **Effects** bar) between layers in the panel moves it; `Alt`-drag
  copies. Dragging effects onto the canvas applies to the topmost layer with pixels
  at the drop point.
- **Scale Effects** (`Layer > Layer Style > Scale Effects`) scales the effect
  parameters without scaling the layer object, via a percentage with preview.
- **Create Layers** (`Layer > Layer Style > Create Layers`) explodes the style into
  regular image layers for hand-editing; the result may not exactly match the live
  style. `Rasterize Layer Style` (CS6) merges the effects into the layer in one
  step. Some effects (e.g. Inner Glow) become layers inside a clipping mask.
- **Style presets** live in the **Styles** panel and are grouped into loadable
  libraries (web buttons, text effects, etc.). Clicking a style replaces the
  current style; `Shift`-click/drag **appends** instead of replacing. Styles can
  be renamed, deleted, saved to and loaded from `.asl` libraries, reset, and
  managed in the Preset Manager. Saving a library into the default
  `Presets/Styles` folder makes it appear at the bottom of the Styles panel menu
  on restart.
- **Sources conflict on groups.** The CS6 Help still states "You cannot apply
  layer styles to a background, locked layer, or group", but a community CS6
  reference states CS6 added layer styles to groups. The Help is likely carrying
  a CS5-era line. Treat group styles as probable-but-unverified (see
  `## Open questions`). Background and locked layers are always excluded; convert
  the background first.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Layers panel > Add a Layer Style icon | Menu | n/a | Adds one effect or opens Blending Options |
| Layers panel > double-click layer (outside name/thumbnail) | Gesture | n/a | Opens Layer Style dialog / Blending Options |
| Layer > Layer Style | Menu | n/a | All effects, Copy/Paste/Clear Layer Style, Global Light, Scale/Create/Rasterize, Hide/Show All Effects |
| Layer Style dialog | Dialog | n/a | Left list of effects; checkbox to enable without selecting; click name to edit |
| Styles panel | Dock | `Window > Styles` | Preset thumbnails/list; Create New Style, Clear Style buttons; fly-out library menu |
| Options bar (Shape/Pen in shape mode) | Pop-up style picker | n/a | Select a style before drawing |
| Layers panel > effect row | Drag / `Alt`-drag | n/a | Move / copy effects between layers |
| `Alt`-click eye or fx triangle | Gesture | n/a | Show/hide all effects; expand/collapse styles (in groups) |
| Layer > Layer Style > Global Light | Dialog | n/a | Master Angle + Altitude |
| Layer > Layer Style > Scale Effects | Dialog | n/a | Percentage slider + Preview |
| Brush/Shape options | — | — | Styles can be selected for Shape tools before drawing |

Layer-panel shortcuts (CS6 Help, "Keys for layers"): double-click layer = edit
style; double-click an effect = edit that effect; `Alt`-double-click an effect =
hide it; double-click the layer mask thumbnail = mask display options.

## Parameters & ranges

Controls are grouped by effect. `Blend Mode` ranges over the `LAY-010` mode set;
`Opacity` is 0–100 % everywhere. Units are PSD units where documented. Ranges not
stated by the CS6 Help are marked `not stated` or `(inferred)`.

### Drop Shadow

| Control | Type | Default | Range | Notes |
|---|---|---|---|---|
| Blend Mode | enum | Multiply | 27 modes | black shadow by default |
| Color | color | black | RGB/CMYK/Lab-picker | |
| Opacity | percent | 75 | 0–100 | |
| Angle | degrees | 30 (global) | 0–359 | drag in canvas to set |
| Use Global Light | bool | on/off (on) | — | links Angle to the global angle |
| Distance | px | 5 | 0–~30000 `(inferred)` | vertical offset is opposite the angle |
| Spread | percent | 0 | 0–100 | expands the matte before blur; 100 = hard edge |
| Size | px | 5 | 0–250 `(styles format)` | blur radius |
| Contour | contour | Linear | contour list | shapes opacity falloff |
| Anti-alias | bool | on | on/off | blends contour edges |
| Noise | percent | 0 | 0–100 | |
| Layer Knocks Out Drop Shadow | bool | on | on/off | hides shadow through semi-transparent layer pixels |

### Inner Shadow

`Blend Mode` (Multiply), `Color` (black), `Opacity` (75 %), `Angle` (30°,
global), `Distance` (5 px), **`Choke`** (0 %, 0–100; shrinks the matte before
blur), `Size` (5 px), `Contour` (Linear), `Anti-alias`, `Noise` (0 %).

### Outer Glow

| Control | Type | Default | Range | Notes |
|---|---|---|---|---|
| Blend Mode | enum | Screen | 27 modes | |
| Opacity | percent | 75 | 0–100 | |
| Noise | percent | 0 | 0–100 | |
| Color / Gradient | toggle | yellow (`#ffffbe` `(inferred)`) | picker / gradient | gradient mode enables more controls |
| Technique | enum | Softer | Softer, Precise | Softer = blur; Precise = distance measure |
| Spread | percent | 0 | 0–100 | |
| Size | px | 5 | 0–250 `(styles format)` | |
| Contour | contour | Linear | list | creates rings for solid glows |
| Anti-alias | bool | on | on/off | |
| Range | percent | 50 | 0–100 | which portion of the glow the contour targets |
| Jitter | percent | 0 | 0–100 | varies gradient color/opacity (gradient mode) |

### Inner Glow

Same as Outer Glow, plus **Source** (`Edge` or `Center`); uses **`Choke`**
instead of `Spread` (0–100 %). Default Source is Edge.

### Bevel & Emboss

| Control | Type | Default | Range | Notes |
|---|---|---|---|---|
| Style | enum | Inner Bevel | Inner Bevel, Outer Bevel, Emboss, Pillow Emboss, Stroke Emboss | Stroke Emboss invisible without a Stroke |
| Technique | enum | Smooth | Smooth, Chisel Hard, Chisel Soft | smoothing vs distance measurement |
| Depth | percent | 100 `(inferred)` | 0–1000 `(inferred)` | bevel strength |
| Direction | enum | Up | Up, Down | |
| Size | px | 5 | 0–250 `(styles format)` | bevel width |
| Soften | px | 0 | 0–~250 `(inferred)` | blurs shading |
| Angle | degrees | 120 (global) `(inferred)` | 0–359 | |
| Altitude | degrees | 30 `(inferred)` | 0–90 | light height; 0 = ground, 90 = straight above |
| Use Global Light | bool | on | on/off | |
| Gloss Contour | contour | Linear | list | applied after shading; metallic look |
| Anti-alias | bool | on | on/off | |
| Highlight Mode / Color / Opacity | enum / color / percent | Screen / white / 75 % `(inferred)` | 27 modes / picker / 0–100 | |
| Shadow Mode / Color / Opacity | enum / color / percent | Multiply / black / 75 % `(inferred)` | 27 modes / picker / 0–100 | |
| Contour (sub-panel) | contour + Anti-alias + Range | off / Linear / off / 50 % | list | shapes the bevel edge |
| Texture (sub-panel) | Pattern, Scale, Depth (+/-), Invert, Link With Layer, Snap To Origin | off | pattern list / 1–1000 % / 0–1000 % `(inferred)` | embossed surface; Depth controls direction up/down |

### Satin

`Blend Mode` (Multiply `(inferred)`), `Color` (black `(inferred)`), `Opacity`
(50 %), `Angle` (19° `(inferred)`), `Distance` (11 px `(inferred)`), `Size`
(14 px `(inferred)`), `Contour` (list), `Anti-alias`, `Invert` (off).

### Color / Gradient / Pattern Overlay

| Effect | Controls |
|---|---|
| Color Overlay | Blend Mode, Color, Opacity (100 % `(inferred)`) |
| Gradient Overlay | Blend Mode, Opacity, Gradient, Reverse, Style (Linear/Radial/Angled/Reflected/Diamond), Align With Layer, Angle, Scale, **Dither (CS6)** |
| Pattern Overlay | Blend Mode, Opacity, Pattern, Snap To Origin, Link With Layer, Scale (1–1000 % `(inferred)`) |

### Stroke

| Control | Type | Default | Range | Notes |
|---|---|---|---|---|
| Size | px | 3 `(inferred)` | **1–250** (integer) | layer-style strokes are integer-size and cap at 250 px (Bjango) |
| Position | enum | Outside | Outside, Inside, Center | |
| Blend Mode | enum | Normal | 27 modes | |
| Opacity | percent | 100 | 0–100 | |
| Fill Type | enum | Color | Color, Gradient, Pattern | |
| Color / Gradient / Pattern | picker | foreground `(inferred)` | per type | gradient adds Reverse/Style/Align/Angle/Scale/**Dither (CS6)**; pattern adds Link With Layer/Scale |

General effect controls defined in Help: `Altitude`, `Angle`, `Anti-alias`,
`Blend Mode`, `Choke`, `Color`, `Contour`, `Distance`, `Depth`, `Use Global
Light`, `Gloss Contour`, `Gradient`, `Highlight/Shadow Mode`, `Jitter`, `Layer
Knocks Out Drop Shadow`, `Noise`, `Opacity`, `Pattern`, `Position`, `Range`,
`Size`, `Soften`, `Source`, `Spread`, `Style`, `Technique`, `Texture`.

## Algorithms & pipeline

Adobe's effect rendering is closed; the following is a **behavioral-parity
proposal**, built from the observable controls and standard image-processing
primitives. Every step is *(inferred)* unless it is a control definition above.

### Shared primitives

1. **Matte extraction.** From the layer's alpha (after masks, before `Fill`), build
   a coverage matte `M` in [0,1]. Interior effects use `M`; exterior effects use
   the "outside" region `1 - M`.
2. **Spread / Choke.** `Spread` dilates `M` (max filter) and `Choke` erodes it,
   by a radius proportional to the control value, *before* blurring. This is why
   `Spread = 100 %` yields a hard edge and `0 %` a soft one.
3. **Blur.** A Gaussian (separable) blur of radius `Size` on the matte. `Soften`
   is an additional blur of the shading result.
4. **Contour LUT.** A contour is a 1-D piecewise curve (input `i` → output `o`,
   editable in the Contour Editor with linear corners or smooth `Corner` points).
   It remaps the matte/effect value; `Anti-alias` blends edge pixels for complex
   contours.
5. **Colorize and composite.** Multiply the matte by the effect color/gradient/
   pattern and `Opacity`, then composite with the effect's own `Blend Mode`
   (exterior effects against layers beneath; interior effects against the layer).
6. **Noise / Jitter.** `Noise` perturbs the matte opacity stochastically;
   `Jitter` varies gradient color/opacity. Both affect only the effect output.

### Drop Shadow / Inner Shadow

Drop Shadow: offset the coverage matte by `Distance` in the `Angle` direction,
apply `Spread` then `Size` blur, multiply by the contour LUT and shadow color,
composite under the layer with the shadow blend mode. `Layer Knocks Out Drop
Shadow` on (default) restricts the visible shadow to where the layer is not
semi-transparent; off lets the shadow show through the layer's alpha, matching a
"shadow visible through glass" look.

Inner Shadow: invert the matte, offset/erode by `Distance`, `Choke`, `Size`, and
contour, then composite *inside* the layer's opaque area with the shadow mode.

### Glows

Outer Glow = blurred/spread exterior matte (Softer) or a distance-transform band
(Precise), contoured via `Range`, colorized, composited outside. Inner Glow = same
on the interior matte, with `Source = Edge` (default) using the edge distance or
`Source = Center` using distance from the content center. Gradient glows evaluate
a gradient across the effect (with `Jitter`); solid glows use `Contour` to create
transparency rings.

### Bevel & Emboss

1. **Height field.** Compute a signed distance / matte transition across the layer
   edge over width `Size`. `Technique` selects smoothing (Smooth) or distance
   measurement (Chisel Hard/Soft), which changes how well detail is preserved.
2. **Direction.** `Up` raises the surface, `Down` inverts it.
3. **Normal & light.** Derive a surface normal from the height gradient; build the
   light vector from `Angle`/`Altitude`; Lambertian dot product yields a signed
   shading value.
4. **Highlight/Shadow.** Positive shading → highlight color × highlight mode ×
   opacity; negative → shadow color × shadow mode × opacity.
5. **Gloss Contour** remaps the shading (applied *after* shading) for a
   metallic/glossy look; the bevel edge **Contour** remaps the profile, with
   `Range`.
6. **Texture** (optional) modulates the height field with a repeating pattern
   (`Scale`, `Depth`, `Invert`), embossing the surface.
7. `Soften` blurs the final shading to remove artifacts. `Style` (Inner/Outer/
   Emboss/Pillow/Stroke) selects which side of the matte the bevel occupies.

### Satin

Build an interior distance field at `Angle`; use `Distance` and `Size` to shape a
band, apply the contour and `Invert`, then composite inside the layer with the
Satin blend mode. It is the bevel's distance field without the lighting.

### Overlays and Stroke

- **Overlays** replace the coverage-matte color: Color Overlay fills `M` with a
  flat color; Gradient Overlay evaluates a gradient (`Style`, `Align With Layer`,
  `Angle`, `Scale`, `Reverse`, CS6 `Dither`); Pattern Overlay tiles a pattern
  (`Scale`, `Link With Layer`, `Snap To Origin`).
- **Stroke** computes a band at the coverage edge: dilate `M` outward (Outside),
  inward (Inside), or straddle it (Center) by `Size`, then fill with color/
  gradient/pattern. Layer-style strokes are built from the layer's *bitmap* matte,
  so they are always integer-width and have rounded corners, do not follow open
  paths, and do not feather with a feathered vector mask (Bjango).
- **Stroke Emboss** on Bevel & Emboss confines the emboss to the stroke band.

### Global light and Scale Effects

- **Global light:** one `(Angle, Altitude)` pair stored at document level (image
  resource 1037). Every effect with `Use Global Light` reads it; editing it
  updates all such effects. Local mode stores its own angle on the effect.
- **Scale Effects:** multiply every pixel-valued parameter (Distance, Size, Spread/
  Choke effective radius, Stroke Size, bevel Size/Soften, Texture/Gradient/Pattern
  scale, Satin Distance/Size) by the percentage. Opacity, angle, colors, contours,
  and blend modes are unchanged (proposed behavior; Adobe's exact parameter set is
  unverified).

### Effect application order

CS6 Help notes the effects list was **reordered in CS6 to the order in which effects
are applied**. The rendering order is a design decision that must reproduce CS6's
observed stacking: Drop Shadow is composited beneath the layer, the surface effects
(Bevel, Satin, Overlays, Stroke) on/within it, and glows around it. Interior
effects grouped by `Blend Interior Effects As Group` are composited as a unit
before the layer blends with the backdrop.

## Rust module mapping

Design proposal — layer styles are render passes over the layer's matte, so they
build on `pictura_blend` and the compositor.

- `pictura_style::StyleEffect` — enum mirroring the `lrFX` set plus the extended
  style keys: `DropShadow`, `InnerShadow`, `OuterGlow`, `InnerGlow`, `BevelEmboss`,
  `Satin`, `ColorOverlay`, `GradientOverlay`, `PatternOverlay`, `Stroke`.
- `pictura_style::params` — one struct per effect carrying every control above with
  PSD units; `Contour` is a `Vec<(f32, f32)>` LUT with a `corner` flag.
- `pictura_style::matte` — `fn coverage_matte(layer) -> Mask`, `fn spread(Mask,
  f32)`, `fn choke(Mask, f32)`, `fn blur(Mask, sigma)`.
- `pictura_style::contour` — `fn eval(contour, x) -> f32`, contour library
  serialization (`.acv`-like).
- `pictura_style::shadow` / `::glow` / `::bevel` / `::satin` / `::overlay` /
  `::stroke` — per-effect renderers returning an RGBA tile plus a blend mode and
  opacity, consumed by `pictura_core::composite`.
- `pictura_style::light` — `GlobalLight { angle, altitude }` from the document
  resource; `fn vector(angle, altitude) -> [f32; 3]`.
- `pictura_style::scale` — `fn scale_effects(style, pct) -> StyleEffect`.
- `pictura_style::preset` — `StylePreset { name, effects, preview }`; `.asl`-style
  library load/save.
- `pictura_core::style` — owns `Vec<StyleEffect>` on `Node` (per `ARCH-008`) and
  the Advanced Blending flags.

Crossing types: `Mask`/tile refs, `BlendMode` (`pictura_blend`), `Color`,
`GradientId`, `PatternId`, `ContourId`, `Rect`.

## Qt6 component mapping

Widgets (consistent with `ARCH-003`).

- `LayerStyleDialog` (`QDialog`) — left effect list with enable checkboxes, stacked
  option pages, `Make Default` / `Reset To Default`, `New Style`.
- `EffectOptionsPage` subclasses — `DropShadowPage`, `InnerShadowPage`,
  `GlowPage`, `BevelEmbossPage` (with nested Contour/Texture), `SatinPage`,
  `OverlayPage`, `StrokePage`.
- `ContourEditorWidget` (`QWidget`) — interactive contour curve, Input/Output
  fields, `Corner` toggle, New/Load/Save.
- `GradientPickerWidget` / `PatternPickerWidget` — re-used from `07-color-painting`.
- `StylesPanel` (`QDockWidget`) + `StylePresetModel` (`QAbstractListModel`) +
  `StylePresetView` — thumbnails/list/stroke views; Create/Clear; library menu.
- `LayerStyleBadgeDelegate` (`QStyledItemDelegate`) — draws the `fx` badge and the
  Blade-If/Blending-Options badge.
- `GlobalLightDialog` (`QDialog`) — Angle + Altitude rotary control.
- `ScaleEffectsDialog` (`QDialog`) — percentage + Preview.
- `EffectDragController` — panel drag/drop for moving/copying effects and styles.

## Data-model impact

- `Node.styles: Vec<StyleEffect>` (proposed in `ARCH-008`) stores typed parameters
  for every control. Each effect owns its blend mode, color, opacity, and (for
  glows/overlays/stroke) a gradient/pattern/contour reference.
- **PSD `lrFX`** stores the classic effect set (`cmnS`, `dsdw`, `isdw`, `oglw`,
  `iglw`, `bevl`, `sofi`) with fields: version, effect count, and per effect
  `8BIM`+key, size, then type-specific `UnitFloat`/`Fixed`/boolean values
  (blur, intensity, angle, distance, opacity, color, blend mode, enabled,
  global-angle flag, etc.). PSD units per `ARCH-008`: blur px, intensity percent,
  angle degrees (default 30), distance px, opacity percent.
- Effects newer/extended beyond `lrFX` (gradient overlay, pattern overlay,
  stroke, satin, bevel contour/texture) are carried in **additional layer
  information** blocks with Pascal-string unit-float descriptors (e.g. `SoLd`,
  `vstk`, `vsms`, `vscg`, `vogk` families). Unknown keys and unknown descriptor
  fields are **preserved verbatim** so CS6 files round-trip.
- **Global light** is document image resource 1037; **Scale Effects** and copying/
  pasting are commands, not stored state.
- Style presets are external `.asl` libraries plus in-panel presets; a new preset
  saved from the panel lives in preferences until explicitly saved to a library.
- Advanced Blending flags and `Blend If` ranges are node fields (see `LAY-010`).
  Changing them shows the "Blending Options customized" badge in CS6.
- Undo: every effect add/remove/parameter edit is a command with a scalar diff;
  no pixel backup is required because effect pixels are derived. `Create Layers`
  / `Rasterize Layer Style` are destructive commands that create new pixel layers
  and need pixel/structural undo records (`ARCH-009`).

## Edge cases

- **Background/locked layers.** Styles cannot be applied; converting the background
  is required. Whether CS6 permits group styles is contradictory across sources
  (see `## Open questions`).
- **Semi-transparent layer + Drop Shadow.** `Layer Knocks Out Drop Shadow` on
  (default) hides the shadow under partial alpha; off shows it through. A naive
  implementation that always reserves a shadow hole will fail this.
- **Bevel `Stroke Emboss` with no Stroke** renders nothing (documented).
- **Stroke at large sizes** is capped at 250 px, integer-width, rounded corners,
  does not respect open paths, and ignores vector-mask feather; vector strokes in
  the Shape properties are a different feature (`03-tools/shape-tools.md`).
- **Layer mask / vector mask with `Hides Effects`** restricts effects; without it,
  effects extend beyond the mask. `Transparency Shapes Layers` (default on) keeps
  knockouts inside opaque pixels.
- **32-bit HDR.** Layer styles and supported blending modes are listed as
  available at 32 bpc; not all effects may be equally meaningful, and the exact
  32-bit clamps are undocumented.
- **CMYK/Lab.** Colors and blend modes are limited by the document mode
  (`LAY-010`); the effect color picker must use the document space.
- **Empty / 1-px content.** Effects on zero-area layers must be no-ops; tiny masks
  must not divide by zero in distance transforms.
- **PSB / huge docs.** Effect rendering blurs large mattes; it must tile/stream and
  not allocate a full-canvas float buffer per effect.
- **Scaling a document.** Layer effects do **not** scale with `Image Size`; the
  user must run `Scale Effects` (or the effect sizes silently mismatch — a common
  real-world bug, documented by Bjango/Adobe community).
- **Many effects.** All effects on a layer share one undoable style edit; stacking
  dozens of large blurs is a performance/memory risk and should be budgeted.
- **GPU unavailable.** Effects must render through the CPU fallback; bevel
  distance transforms are the expensive step.
- **Unknown effect keys.** Must be preserved and reported, never dropped.

## Parity acceptance criteria

1. Given a layer and each effect with default parameters, Kooka Pictura's rendered
   result matches a CS6 render within the tolerance `T` from
   `11-cross-cutting/testing-strategy.md` for 8- and 16-bit RGB.
2. Given Drop Shadow `Distance = d` and `Angle = θ`, the shadow centroid is offset
   `d` px in direction `θ` (±1 px).
3. Given Drop Shadow `Spread = 0` and `100`, the shadow edge softness increases
   monotonically and `100 %` produces a hard edge (Bjango/Help contract).
4. Given `Layer Knocks Out Drop Shadow` on, no shadow is visible through a 50 %
   opaque layer; off, the shadow is visible through it.
5. Given `Use Global Light` on for Drop Shadow and Bevel, changing the global angle
   updates both consistently; turning it off makes them independent.
6. Given a contour (Linear, Gaussian, Ring, custom) the effect's opacity falloff
   matches the contour LUT within tolerance.
7. Given `Technique = Chisel Hard` vs `Smooth`, an anti-aliased type mask retains
   sharper bevel detail under Chisel Hard (observable feature-preservation
   difference).
8. Given Stroke `Size = 250` and `300`, the first is accepted and the second is
   clamped/unavailable; stroke width is integer.
9. Given a PSD with a full style, opening and re-saving preserves every effect,
   its enabled/global-light flags, and unknown additional-layer keys byte-for-byte.
10. Given `Copy Layer Style` then `Paste Layer Style`, the destination's style is
    replaced exactly; with `Shift`-click from the Styles panel it is appended.
11. Given `Scale Effects 200 %`, pixel-valued parameters double and opacity, angle,
    colors, and blend modes are unchanged; the preview matches the applied result.
12. Given `Create Layers` on a style, the produced layers compose to the same
    appearance as the live style within tolerance (the Help warns it may not be
    exact).

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf`
  (downloaded, `pdftotext -layout`) — official CS6 Help. Sections used: "Layer
  effects and styles" (pp. 201–207): about effects/styles, preset styles, the
  Layer Style dialog overview (the seven effect families), apply/edit, custom
  defaults, "Layer style options" (the full control glossary: Altitude, Angle,
  Anti-alias, Blend Mode, Choke, Color, Contour, Distance, Depth, Use Global
  Light, Gloss Contour, Gradient, Highlight/Shadow Mode, Jitter, Layer Knocks Out
  Drop Shadow, Noise, Opacity, Pattern, Position, Range, Size, Soften, Source,
  Spread, Style, Technique, Texture), contour editing, global lighting, display/
  hide, copy, scale, remove, create layers, and create/manage presets; "Group
  blend effects" (pp. 193–194) for Blend Interior Effects As Group and Blend
  Clipped Layers As Group; "What's new in CS6 > Layers" (p. 10) for dither, the
  effects reorder, Rasterize Layer Style, and the Blend If badge (the same list
  does not mention group styles); "Keys for layers" (pp. 88–90) for panel
  shortcuts.
- `https://web.archive.org/web/2023id_/https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/`
  — Adobe Photoshop File Formats Specification. Established the `lrFX` effects
  layer record (`cmnS`, `dsdw`, `isdw`, `oglw`, `iglw`, `bevl`, `sofi`), global
  angle resource 1037, and the additional-layer-information preservation rule.
  Also used by `ARCH-008`.
- `http://www.photoshopforphotographers.com/3101-1901/Help_guide/layerstyle.html`
  — community reference for CS6 layer-style semantics: Drop Shadow uses Multiply
  at 75 % by default, Spread `0 %` = softest / `100 %` = hard edge, and the same
  controls are shared between Drop Shadow and Inner Shadow. Secondary source.
- `https://bjango.com/articles/photoshopcs6strokes` — established that layer-style
  strokes are integer-width, cap at 250 px, have rounded corners, do not follow
  open paths, and do not blur with mask feathering (unlike the CS6 vector stroke
  feature). Secondary/community source.
- `https://search.brave.com/search?q=Photoshop+CS6+%22layer+style%22+size+maximum+250+px+spread+choke+distance+units+reference`
  — search results page only (not a fetched primary page). Corroborated the
  community "Photoshop Styles File Format" `Size: 0 to 250 pixels` / `Noise` in
  `#Prc` units statement and the 250 px layer-style stroke cap. Because only the
  snippet was seen, no other numeric range from it is asserted.

Not fetched / not used as a primary source: `helpx.adobe.com` (documented HTTP 403
in `README.md`).

## Open questions

- **Layer styles on groups.** The CS6 Help ("You cannot apply layer styles to a
  background, locked layer, or group") contradicts a community CS6 reference
  claiming group styles are new in CS6. *Resolves with:* a CS6 test applying a
  Style to a group, or a definitive Adobe release-note.
- **Numeric ranges and defaults** for Distance, Size, Technique details, Depth,
  Soften, Altitude, Angle, Jitter, Range, Scale, Noise, Satin parameters, and each
  effect's default color/mode/opacity are not stated in the CS6 Help PDF. The
  `0–250 px` Size and integer/250 px Stroke limit are corroborated; the rest is
  `(inferred)`. *Resolves with:* a CS6 UI capture of each effect dialog or the
  community "Photoshop Styles File Format" reference fetched in full.
- **Default effect values.** Only Drop Shadow's Multiply/75 % black default is
  sourced. The defaults in the parameter tables above marked `(inferred)` need a
  CS6 dialog capture. *Resolves with:* a CS6 install screenshot set.
- **Effect render order in CS6.** The Help says the list was reordered to the
  application order but does not state the order. *Resolves with:* empirical
  comparison of a style containing overlapping Drop Shadow/Outer Glow/Stroke.
- **Exact shading model** for Bevel & Emboss (distance metric, normal smoothing,
  `Gloss Contour` placement, `Depth` scaling) is closed. *Resolves with:*
  pixel-diff against CS6 bevels on a hard-edged shape.
- **`Spread`/`Choke` mapping to matte dilation** — whether linear in percent or
  proportional to Size, and the exact radius — is inferred. *Resolves with:* CS6
  measurements of edge position vs. control values.
- **Gradient/Pattern Overlay serialization keys** (the extended style descriptor
  blocks) are not fully catalogued in the fetched file-format excerpt. *Resolves
  with:* the full additional-layer-information section or CS6-made PSDs.
- **`Scale Effects` covered parameters.** Which parameters Adobe scales and
  whether contours/texture depth scale is unverified. *Resolves with:* CS6 test
  styles scaled at 50 %/200 % and compared.
- **Copy/paste across color modes and bit depths.** Whether styles adapt or are
  clamped when pasted into a CMYK/Lab or higher-bit-depth document is unverified.
  *Resolves with:* a CS6 cross-mode paste test.
- **`Create Layers` fidelity.** The Help explicitly warns the result may differ;
  the acceptable tolerance is unresolved. *Resolves with:* a documented tolerance
  in `11-cross-cutting/testing-strategy.md`.
