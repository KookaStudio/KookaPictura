# Mixer Brush Engine

- **Spec ID:** `BRU-004`
- **Status:** `Draft`
- **Parity tier:** `Core` — the Mixer Brush is in CS6 Standard (introduced CS5).
- **New in CS6:** `No` — the Mixer Brush arrived in CS5 and its wet/load/mix model is unchanged in CS6; CS6 adds the general brush features (Brush Pose, Brush Projection, 5000 px) to it.
- **Depends on:** `BRU-001` brush-engine, `BRU-002` brush-dynamics, `BRU-003` bristle-brushes, `BRU-005` airbrush-and-flow, `BRU-006` brush-presets, `ARCH-008` document-model, `ARCH-009` undo-history.

> Module and widget names are **design proposals**. The repository contains a
> working implementation; this file stays the behavioural contract for the
> feature, and `docs/dev/doc-code-parity-gaps.md` records what shipped and what
> did not. Adobe's reservoir/pickup mixing math is closed; this spec gives a
> **behavioral model** only. Everything not established by the fetched sources
> is marked **algorithm TBD** or listed under Open questions.

## CS6 behavior

Help: "The Mixer Brush simulates realistic painting techniques such as mixing
colors on the canvas, combining colors on a brush, and varying paint wetness
across a stroke."

The defining mechanic is **two paint wells**:

- **Reservoir** — "stores the final color deposited onto the canvas and has more
  paint capacity."
- **Pickup** — "receives paint only from the canvas; its contents are
  continuously mixed with canvas colors."

Practical CS6 behavior:

- **Loading.** To load paint into the reservoir, Alt-click (Windows) / Option-
  click (Mac OS) the canvas, or choose a foreground color. When loaded from the
  canvas, the brush tip reflects color variation in the sampled area; select
  **Load Solid Colors Only** (in the Current Brush Load pop-up) for a uniform
  color instead.
- **Brush selection.** Any brush preset may be used; the Mixer Brush is commonly
  paired with bristle and wet-media brushes.
- **Options bar (Mixer-unique):**
  - **Current Brush Load swatch** — pop-up with `Load Brush`, `Clean Brush`, and
    `Load Solid Colors Only`; the automatic `Load`/`Clean` options perform these
    after each stroke.
  - **Preset pop-up** — "applies popular combinations of Wet, Load, and Mix
    settings."
  - **Wet** — "how much paint the brush picks up from the canvas. Higher settings
    produce longer paint streaks."
  - **Load** — "the amount of paint loaded in the reservoir. At low load rates,
    paint strokes dry out more quickly."
  - **Mix** — "the ratio of canvas paint to reservoir paint. At 100%, all paint is
    picked up from the canvas; at 0%, all paint comes from the reservoir. (The Wet
    setting, however, continues to determine how paints mix on the canvas.)"
  - **Flow** — the rate of application (shared with other paint tools;
    `BRU-005`).
  - **Sample All Layers** — "picks up canvas color from all visible layers."
  - **Airbrush / Build-up** and Smoothing are shared.
- **Keyboard:** number keys change the Wet setting; `Alt+Shift`+number changes
  Mix; `00` sets Wet and Mix to zero.
- **Gestures:** drag paints; click + Shift-click draws a straight line; with the
  airbrush option, holding still builds up.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Tools panel, Brush group | Tool | `B` (cycle) | Hold the Brush tool to reveal the Mixer Brush |
| Options bar | Bar | — | Current Brush Load swatch, Preset pop-up, Wet, Load, Mix, Flow, Sample All Layers, Airbrush, Smoothing |
| Current Brush Load pop-up | Menu | — | Load Brush, Clean Brush, Load Solid Colors Only, auto Load/Clean after each stroke |
| Canvas | Gesture | Alt/Option-click | Loads sampled canvas color into the reservoir |
| Keyboard | — | number keys | Wet setting |
| Keyboard | — | `Alt+Shift`+number | Mix setting |
| Keyboard | — | `00` | Wet and Mix to zero |
| Brush panel | Dock | `F5` | Tip + dynamics; bristle tips common (`BRU-003`) |
| Brush Presets panel | Dock | `F5` | Wet-media / Mixer presets |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Current Brush Load | swatch + menu | current | Load Brush, Clean Brush, Load Solid Colors Only | Auto Load/Clean after stroke toggles |
| Preset | enum | — | popular Wet/Load/Mix combinations | Applies a setting bundle |
| Wet | int % | per preset | 0–100 | Canvas pickup; higher = longer streaks |
| Load | int % | per preset | 1–100 *(unverified)* | Reservoir amount; low = dries out faster |
| Mix | int % | per preset | 0–100 | 0 = all reservoir, 100 = all canvas |
| Flow | int % | 100 *(unverified)* | 0–100 | Rate of deposition |
| Opacity | int % | 100 *(unverified)* | 0–100 | Per-stroke paint cap |
| Sample All Layers | checkbox | off | on/off | Pick up color from all visible layers |
| Load Solid Colors Only | checkbox | off | on/off | Uniform tip color from a sampled load |
| Auto Load / Auto Clean | checkbox | off | on/off | After each stroke |
| Airbrush / Build-up | checkbox | off | on/off | See `BRU-005` |
| Smoothing | checkbox + value | off | 0–100 | See `BRU-001` |

Exact CS6 defaults for Wet/Load/Mix depend on the selected preset and are not
stated by the fetched primary source.

## Algorithms & pipeline

**Behavioral model** (exact Adobe math TBD). Each stroke maintains:

- `reservoir: Pigment` — the loaded deposit color; `reservoir_volume ∈ [0,1]`
  proportional to Load.
- `pickup: Pigment` — continuously overwritten/averaged from the canvas
  underneath the dab.

Per dab at canvas color `C`:

1. **Pickup (Wet).** Mix the sampled canvas color into the pickup well at a rate
   set by Wet: higher Wet takes up more canvas color and holds it longer, which
   is what produces the long smeared streaks. With Wet = 0 the pickup stays
   empty and the stroke lays down reservoir color only.
2. **Mix.** Form the deposited color as a blend of reservoir and pickup weighted
   by Mix: `deposit = lerp(reservoir_color, pickup_color, mix)`. At Mix = 0 all
   reservoir; at Mix = 100 all canvas. Help notes Wet still governs how the
   paints mix on the canvas even when Mix = 100.
3. **Deposit (Flow/Load).** Composite `deposit` into the canvas at Flow (capped
   per stroke by Opacity) and deplete `reservoir_volume` by an amount related to
   Flow/Load. Low Load drains quickly, after which the stroke is mostly
   canvas-picked-up color; high Load sustains the reservoir color over a longer
   distance.
4. **Reservoir only mixes if Wet > 0.** A dry reservoir with Wet = 0 behaves much
   like a normal hard-edged paint.
5. **Clean / Load.** `Clean Brush` empties both wells; `Load Brush` refills the
   reservoir with the current foreground color; the auto toggles do these on
   stroke release. `Load Solid Colors Only` bypasses the per-pixel color
   variation of a sampled load.
6. **Sample All Layers** resolves `C` from a flattened read of all visible layers
   rather than the active layer only. When off, `C` comes from the active layer
   (transparent regions contribute nothing).

This model explains the documented diagnoses: a brush "running out of color too
soon" is a Load issue; "pulling canvas color too strongly" is Wet/Mix; "new color
laying down too thickly" is Flow/Load/pressure.

Per-stroke vs per-dab: the reservoir is a stroke-scoped state machine, not a
pure per-dab function; the pickup well is per-dab. This is why two strokes with
identical settings can start differently depending on auto Load/Clean.

## Rust module mapping

- `pictura-brush::mixer::MixerBrush` — owns `StrokeState` with `reservoir` and
  `pickup` wells.
- `pictura-brush::mixer::Reservoir` — `{ color: Pigment, volume: f32 }`.
- `pictura-brush::mixer::Pickup` — `{ color: Pigment, wetness: f32 }`.
- `pictura-brush::mixer::MixerConfig` — `{ wet, load, mix, flow, opacity,
  sample_all_layers, auto_load, auto_clean, load_solid }`.
- `pictura-brush::mixer::load_from_canvas(area) -> Pigment` — sampled area color
  (uniform or varying).
- `pictura-core::command::BrushStroke` — as `BRU-001` (pre-edit tiles + seed for
  undo/redo).

Boundary types: `Pigment` (color-model-aware), `MixerConfig`, `SampleArea`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `MixerBrushOptions` | `QWidget` (options bar) | Current Brush Load swatch/menu, Preset, Wet/Load/Mix/Flow, Sample All Layers, Airbrush |
| `BrushLoadSwatch` | `QToolButton` + menu | Shows current load preview; Load/Clean/Load Solid Colors Only; auto toggles |
| `MixerPresetCombo` | `QComboBox` | Bundled Wet/Load/Mix presets |
| `MixerPreview` | `QQuickItem` | Optional live tip/load preview |
| `BristleTipEditor` | `QWidget` | Bristle tips commonly paired (`BRU-003`) |

Widgets for the options bar. Wet/Load/Mix/Flow sliders must be live and, because
the reservoir is stroke-scoped, the UI must not imply the swatch is a static
color.

## Data-model impact

- **No PSD fields.** Wells, load, and wetness are session-only.
- **Preset serialization:** the Wet/Load/Mix/Flow bundle is stored in the `.abr`
  paint-dynamics descriptor group (`usePaintDynamics`); the community ABR map does
  not fully decode these, so round-trip fidelity is an open risk (`BRU-006`).
- **Undo:** one history state per completed stroke. Because the reservoir is
  stroke-scoped, the stroke command must capture the initial reservoir state so
  redo starts from the same paint.
- **Non-destructive workflows:** Sample All Layers lets the user paint on an empty
  layer while reading the composite; the command still targets the active layer.

## Edge cases

- **Empty / transparent active layer with Sample All Layers off.** No canvas color
  is available for pickup; the brush must still deposit reservoir color (or
  refuse if the reservoir is also empty).
- **Load from an empty area.** Loading via Alt-click on transparency should set
  no meaningful color; define behavior (likely no-op / keep prior load).
- **Load Solid Colors Only.** Must average/select a single color rather than
  sampling per-pixel variation.
- **Mixer on a layer with Lock Transparency.** Pickup may sample transparent
  regions; deposition is constrained by the lock.
- **CMYK/Lab/Indexed.** Pigment mixing must be defined in the document model;
  refuse where undefined (Indexed/Bitmap).
- **Auto Load/Clean interaction.** Auto Load with Auto Clean both on must
  reproduce Adobe's "clean then load" order; both off must carry paint across
  strokes.
- **Huge PSB / Sample All Layers.** Sampling must be tile-local and bounded; no
  canvas-wide flatten per dab.
- **GPU unavailable.** Mixing runs on CPU; previews degrade.
- **Undo mid-stroke.** Atomic on release; cancel restores scratch and wells.
- **Determinism.** Same starting reservoir + canvas + settings must reproduce a
  redo exactly.

## Parity acceptance criteria

- Given Wet = 0, a Mixer stroke lays down reservoir color with hard edges and
  does not blend with the underlying canvas.
- Given increasing Wet at fixed Load/Mix, canvas color is dragged farther along
  the stroke and streaks lengthen.
- Given Mix = 0, no canvas color enters the deposit; at Mix = 100, the stroke is
  essentially the picked-up canvas color.
- Given low Load, the stroke's reservoir color runs out quickly and later parts
  are dominated by canvas pickup; high Load sustains it longer.
- Given Auto Clean on, the next stroke starts free of the previous mixture; with
  Auto Load on, it starts refilled with the foreground color.
- Given Load Solid Colors Only, loading from a multi-color canvas area yields a
  uniform tip color; unchecking it reflects the sampled variation.
- Given Sample All Layers on, painting on an empty layer picks up colors from
  visible layers below; off, it does not.
- Given two strokes with identical settings but Clean enabled between them, the
  second starts from the same color as the first.
- Given a completed Mixer stroke, undo restores touched pixels bit-exactly and a
  redo reproduces the stroke.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  Adobe Photoshop Help reference (downloaded and text-extracted). Established:
  the Mixer Brush simulates mixing/combining colors and varying wetness; two
  wells (reservoir with more capacity; pickup receiving canvas paint and mixing
  continuously); Alt/Option-click or foreground color to load; Load Solid Colors
  Only; Current Brush Load swatch and `Load Brush`/`Clean Brush` with automatic
  after-stroke options; Preset combinations of Wet/Load/Mix; Wet = canvas pickup
  (longer streaks); Load = reservoir amount (low dries faster); Mix = canvas:
  reservoir ratio (100% canvas, 0% reservoir; Wet still governs mixing on
  canvas); Flow; Sample All Layers; the `Alt+Shift`+number / number / `00`
  keyboard mappings; the CS5 introduction. Primary source.
- `https://glensmith.co.uk/photoshop/mixer-brush` — Mixer Brush walkthrough.
  Established: Load Brush/Clean Brush/Load Solid Colors Only menu; auto Load/
  Clean after each stroke and how they change successive strokes; Wet/Load/Mix/
  Flow meanings; airbrush-style build-up; Smoothing options (pulled string,
  stroke catch-up, catch-up on stroke end, adjust for zoom); Sample All Layers
  workflow and performance trade-off; creating a Mixer brush preset that captures
  tool settings. Secondary.
- `https://nextgz.net/en/blog/photoshop-mixer-brush-wet-load-mix-flow-digital-painting`
  — Mixer Brush guide. Established: the reservoir/pickup model restated; Wet as
  canvas drag; Load as reservoir amount; Mix as ratio; Flow as deposition rate;
  Auto Load/Clean guidance; Sample All Layers non-destructive workflow and its
  risks. Secondary.
- `https://community.wacom.com/en-co/complete-guide-to-photoshop-brushes-pt-3` —
  Wacom brush guide. Established: Transfer panel contains Mixer-only Wet/Load/Mix
  jitter rows (i.e. the Mixer taps the same dynamics system) and that the Mixer
  Brush is the route to smooth color-transition strokes. Secondary.

Not parsed: `https://www.photoshopforphotographers.com/3101-1901/Help_guide/tp/mixer_brush.html`
(SSL/transport error on fetch; its search-result snippet independently confirms
"two wells: a reservoir and a pickup" and that the reservoir color is set by the
foreground color).

## Open questions

- **Reservoir/pickup blending formula.** The exact interpolation, depletion rate,
  and how Wet and Mix compose are undocumented — algorithm TBD. Resolve by
  measuring deposited color vs iteration count on controlled canvases.
- **Default Wet/Load/Mix/Flow values per shipped preset.** Not in the primary
  source; need a CS6 install to enumerate.
- **Whether Mix can exceed the Wet gate.** Help says Wet still determines mixing
  when Mix = 100; the precise coupling is unverified.
- **Load range.** Whether Load's minimum is 0% or 1% (Help's illustration shows
  1% and 100%) needs confirmation.
- **Auto Load source.** Whether Auto Load always uses the foreground color or the
  last explicit load is unverified.
- **Sample All Layers color space.** Whether sampling occurs before or after color
  management in CMYK/Lab is undocumented.
- **`.abr` paint-dynamics keys.** The community ABR map covers color dynamics at
  ~20% and does not fully decode Wet/Load/Mix; preset round-trip is unproven.
- **Interaction with Color Dynamics.** Whether `BRU-002` hue/sat/brightness jitter
  applies to the reservoir, the pickup, or the final deposit is unknown.
