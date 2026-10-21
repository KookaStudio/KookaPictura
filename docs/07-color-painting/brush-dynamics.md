# Brush Dynamics

- **Spec ID:** `BRU-002`
- **Status:** `Draft`
- **Parity tier:** `Core` — all dynamics are in CS6 Standard and available to any brush-based tool.
- **New in CS6:** `Changed` — Brush Pose panel added; Color Dynamics apply once per stroke by default (Apply Per Tip reverts to the older per-dab behavior); Brush Projection added to Shape Dynamics; texture Brightness/Contrast sliders added.
- **Depends on:** `BRU-001` brush-engine, `BRU-003` bristle-brushes, `BRU-005` airbrush-and-flow, `BRU-006` brush-presets, `ARCH-006` gpu-rendering-pipeline, `ARCH-008` document-model.

> Module and widget names are **design proposals**. The repository contains a
> working implementation; this file stays the behavioural contract for the
> feature, and `docs/dev/doc-code-parity-gaps.md` records what shipped and what
> did not. Adobe's exact curve/Jitter-to-dab mapping is closed; where the Help
> does not state it, this spec is **behavioral parity only, algorithm TBD** and
> the unknown is listed in Open questions.

## CS6 behavior

The Brush panel's left column is a list of **option sets**, each enable-able by
its checkbox and editable by clicking its name; its controls appear on the right.
Dynamics add change to brush marks over a stroke. Help defines two primitives:

- **Jitter** — a percentage of randomness. `0%` = the element does not change
  over the stroke; `100%` = maximum randomness.
- **Control** — a pop-up menu that ties the variance to an input instead of (or
  in addition to) random jitter. The available inputs are `Off`, `Fade`
  (1–9999 steps), `Pen Pressure`, `Pen Tilt`, `Stylus Wheel`, `Rotation`, and —
  only for Angle Jitter — `Initial Direction` and `Direction`.

The Help explicitly warns that pen controls need a pressure-sensitive digitizing
tablet and supported pen; a warning triangle appears if the feature is missing.

The option sets below are the CS6 Brush panel sections. **Color Dynamics** in CS6
varies color once at the start of each stroke by default; checking **Apply Per
Tip** restores the pre-CS6 per-stamp behavior. **Brush Pose** and **Brush
Projection** are new in CS6.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Brush panel, left option-set list | Dock | `F5` | Checkbox toggles a set without opening it; clicking the name opens it |
| Brush panel, right side | Options | — | Controls for the selected set |
| Brush panel lock/unlock | Toggle | — | Lock retains tip-shape attributes across preset changes |
| Brush panel menu | Menu | — | `Clear Brush Controls` resets all non-shape options; `Copy Texture to Other Tools` |
| Brush Presets panel | Dock | `F5` | Stores dynamics in the preset |
| Shape Dynamics | Options | — | Size/Angle/Roundness jitter, Minimum Diameter/Roundness, Tilt Scale, Flip X/Y Jitter, Brush Projection |
| Scattering | Options | — | Scatter, Count, both with Jitter + Control; Both Axes |
| Texture | Options | — | Pattern, Invert, Scale, Texture Each Tip, Mode, Depth, Minimum Depth, Depth Jitter + Control, Brightness, Contrast |
| Dual Brush | Options | — | Secondary tip + Mode, Diameter, Spacing, Scatter, Count |
| Color Dynamics | Options | — | Apply Per Tip, Foreground/Background Jitter + Control, Hue/Saturation/Brightness Jitter, Purity |
| Transfer | Options | — | Opacity Jitter + Control, Flow Jitter + Control (plus Mixer Brush Wet/Load/Mix jitter) |
| Brush Pose | Options | — | Tilt X, Tilt Y, Rotation, Pressure + Override toggles |
| Other toggles | Checkbox | — | Noise, Wet Edges, Airbrush/Build-up, Smoothing, Protect Texture |

## Parameters & ranges

### Shape Dynamics

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Size Jitter | int % + Control | 0 | 0–100% | Random size within [Minimum Diameter, 100%] |
| Minimum Diameter | int % | 0 *(unverified)* | 0–100% | Floor when Size Jitter/Control is on |
| Tilt Scale | int % | 100 *(unverified)* | 0–100%+ *(unverified)* | Height scale before rotation when Size Control = Pen Tilt |
| Angle Jitter | int % + Control | 0 | 0–100% of 360° | Control may be Off/Fade/Pen Pressure/Pen Tilt/Stylus Wheel/Rotation/Initial Direction/Direction |
| Roundness Jitter | int % + Control | 0 | 0–100% | Varies short/long axis ratio |
| Minimum Roundness | int % | 100 *(unverified)* | 0–100% | Floor when Roundness Jitter/Control is on |
| Flip X / Flip Y Jitter | checkbox + % | off | on/off | Random axis flips; present in later builds — CS6 status *(unverified)* |
| Brush Projection | checkbox | off | on/off | CS6; stylus tilt/rotation warp tip shape |

### Scattering

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Scatter | int % + Control | 0 | 0–100%+ *(unverified)* | Radial if Both Axes, else perpendicular to path |
| Both Axes | checkbox | off | on/off | Radial vs perpendicular distribution |
| Count | int + Control | 1 | 1–… *(unverified)* | Marks per spacing interval; high count hurts performance |
| Count Jitter | int % + Control | 0 | 0–100% | Varies marks per interval |

### Texture

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Pattern | pattern ref | none | pattern libraries | Same patterns as Pattern Stamp |
| Invert | checkbox | off | on/off | Swaps high/low points (which areas get most paint) |
| Scale | int % | 100 | pattern-size % | Pattern scale |
| Texture Each Tip | checkbox | off | on/off | Per-stamp texture; required for Depth variance |
| Mode | enum | Multiply *(unverified)* | Multiply, Subtract, Darken, Overlay, Color Dodge, Color Burn, Linear Burn, Hard Mix, Linear Height, Height | Help says "blending mode used to combine the brush and the pattern" |
| Depth | int % | 100 *(unverified)* | 0–100% | 100% = low points get no paint; 0% hides the pattern |
| Minimum Depth | int % | 0 *(unverified)* | 0–100% | Floor for Depth Control/Depth Jitter |
| Depth Jitter | int % + Control | 0 | 0–100% | Only when Texture Each Tip is on |
| Brightness | int | 0 | −100…100 *(unverified)* | CS6 texture slider; shifts pattern tones |
| Contrast | int | 0 | −100…100 *(unverified)* | CS6 texture slider; hardens/softens the pattern |

### Dual Brush

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Secondary tip | tip ref | none | any tip (not bristle/erodible/airbrush in some builds) | Intersection with primary tip is painted |
| Mode | enum | Multiply *(unverified)* | blend list | Combines primary and secondary marks |
| Diameter | int px | tip's own | 1–5000 | `Use Sample Size` available for sampled tips |
| Spacing | int % | 25 *(unverified)* | 0–1000% *(unverified)* | Distance between secondary marks |
| Scatter | int % + Both Axes | 0 | 0–100%+ | Distribution of secondary marks |
| Count | int | 1 | 1–… | Secondary marks per interval |

### Color Dynamics

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Apply Per Tip | checkbox | off (CS6) | on/off | Off = one color per stroke; on = per stamp (pre-CS6 behavior) |
| Foreground/Background Jitter | int % + Control | 0 | 0–100% | Mixes foreground/background; Control Off/Fade/Pen Pressure/Pen Tilt/Stylus Wheel/Rotation |
| Hue Jitter | int % | 0 | 0–100% | Hue deviation from foreground hue |
| Saturation Jitter | int % | 0 | 0–100% | Saturation deviation |
| Brightness Jitter | int % | 0 | 0–100% | Brightness deviation |
| Purity | int % | 0 | −100…100 | −100 fully desaturated, +100 fully saturated |

### Transfer

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Opacity Jitter | int % + Control | 0 | 0–100% | Capped by options-bar Opacity; Control Off/Fade/Pen Pressure/Pen Tilt/Stylus Wheel |
| Flow Jitter | int % + Control | 0 | 0–100% | Capped by options-bar Flow |
| Wet Jitter (Mixer) | int % + Control | 0 | 0–100% | Mixer Brush only (`BRU-004`) |
| Load Jitter (Mixer) | int % + Control | 0 | 0–100% | Mixer Brush only |
| Mix Jitter (Mixer) | int % + Control | 0 | 0–100% | Mixer Brush only |

### Brush Pose (CS6)

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Tilt X | int deg + Override | device | −90…90 *(unverified)* | Left/right tilt from vertical |
| Tilt Y | int deg + Override | device | −90…90 *(unverified)* | Front/back tilt |
| Rotation | int deg + Override | device | 0–360 *(unverified)* | Bristle/tip barrel rotation |
| Pressure | int % + Override | device | 0–100 *(unverified)* | Applied pressure |
| Override | checkbox ×4 | off | on/off | Keep a static pose, ignoring device input |

### Standalone toggles

Noise, Wet Edges, Airbrush/Build-up, Smoothing, and Protect Texture are described in `BRU-001`; their semantics in a dynamics context: Noise is best on soft tips, Wet Edges darkens rims, Build-up enables airbrush accumulation (`BRU-005`), Smoothing filters input (and may add lag), Protect Texture pins the texture across presets.

## Algorithms & pipeline

**Jitter/Control mapping model (behavioral; exact Adobe curves TBD).**

For a dynamic attribute `A` with base value `A₀` and range `[A_min, A_max]`:

1. Compute a normalized control signal `c ∈ [0,1]`:
   - `Off` → jitter only (random).
   - `Fade` → `c` ramps from 1 to 0 across the first `N` dabs (`N` ∈ 1–9999);
     Help describes Fade as fading "from maximum … to no … in the specified
     number of steps" and "between the initial diameter and the minimum
     diameter".
   - Pen Pressure / Pen Tilt / Stylus Wheel / Rotation → `c` is the normalized
     hardware value.
   - `Initial Direction` / `Direction` (Angle only) → `c` derives from the stroke
     tangent at the first sample / the current sample.
2. Combine jitter and control. The commonly used model is
   `A = lerp(A_min, A_max, c · (1 − j) + j · rand())` — i.e. jitter is added on
   top of the controlled value, not a replacement. CS6 Help says jitter and
   control coexist ("At 0%, an element does not change… at 100% maximum
   randomness"; the control "specifies how you want to control the variance").
3. Clamp to the attribute's valid range.

Per-set specifics:

- **Size/Opacity/Flow** default toward "bigger/stronger at full control", with
  Minimum Diameter/Roundness as floors. The Wacom guide notes Fade applies at
  the *beginning* of the stroke and reduces the attribute to nothing over the
  step count.
- **Angle** jitter is a percentage of a full 360° turn. `Direction` re-aims the
  tip along the path each dab (used to make stamps follow curves); `Initial
  Direction` locks to the first motion direction.
- **Scatter** offsets each dab by a random vector; both-axes vs perpendicular
  selects the distribution. `Count` multiplies marks per step; each mark gets its
  own jitter draw.
- **Color** is evaluated in the document color model. Foreground/Background
  jitter interpolates between the two swatch colors; Hue/Saturation/Brightness
  jitter then perturbs within the first color's neighborhood; Purity scales
  saturation.
- **Transfer** multiplies the dab's opacity/flow by the jittered value, always
  bounded by the options-bar Opacity/Flow.
- **Texture** multiplies dab alpha by a sampled pattern value: `Depth` is the
  penetration, so at 100% texture low points receive no paint. `Texture Each Tip`
  samples the pattern per stamp (with optional depth jitter), otherwise the
  pattern is fixed across the stroke. `Invert` flips the high/low mapping.
- **Dual Brush** rasterizes a second stamp and intersects it with the primary
  stamp; the exact per-pixel operator is mode-dependent and, per the Wacom
  guide, differs from ordinary layer/brush modes (Multiply/Subtract appear
  inverted relative to their layer behavior).
- **Brush Pose** produces a static hardware input value when Override is on;
  otherwise device values pass through. Help notes the sliders do not change the
  tip preview until another control references them.
- **Brush Projection** maps tilt/rotation onto the tip's angle/roundness
  transform, so an upright pen gives a circle and a tilted pen an ellipse.

Persistence/RNG: the engine captures a seed at stroke start (`BRU-001`) so each
dynamic draw is reproducible on redo. Each dab consumes a deterministic stream;
per-stroke vs per-dab color selection (Apply Per Tip) changes when color is
drawn from the stream.

## Rust module mapping

- `pictura-brush::dynamics::Attr<T>` — `{ base: T, jitter: f32, control: Control,
  min: T, max: T }`; `eval(signal, rng) -> T`.
- `pictura-brush::dynamics::Control` — enum `{ Off, Fade { steps: u16 },
  PenPressure, PenTilt, StylusWheel, Rotation, InitialDirection, Direction }`.
- `pictura-brush::dynamics::ShapeDynamics`, `Scatter`, `Texture`,
  `DualBrush`, `ColorDynamics`, `Transfer`, `BrushPose` — one struct per option
  set; each `enabled: bool` mirrors the panel checkbox.
- `pictura-brush::dynamics::PoseInput` — static values from Brush Pose Override.
- `pictura-brush::docs` — mirror of every default/range for the UI and validation.
- `pictura-core::command::BrushStroke` — carries the full dynamics config + seed.

Boundary types: `Control`, `Attr`, `PoseInput`, `TextureMode`, `DualMode`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `DynamicsSectionEditor` | `QWidget` | Generic Jitter + Control row (slider/spin + combo + Fade spin) |
| `ShapeDynamicsEditor` | `DynamicsSectionEditor` | Size/Angle/Roundness, Min Diameter/Roundness, Tilt Scale, Flip X/Y, Brush Projection |
| `ScatterEditor` | `DynamicsSectionEditor` | Scatter/Count + Jitter + Both Axes |
| `TextureEditor` | `QWidget` | Pattern picker, Invert, Scale, Texture Each Tip, Mode, Depth, Min Depth, Depth Jitter, Brightness/Contrast |
| `DualBrushEditor` | `QWidget` | Secondary tip picker + Mode/Diameter/Spacing/Scatter/Count |
| `ColorDynamicsEditor` | `QWidget` | Apply Per Tip, FG/BG Jitter, Hue/Sat/Brightness, Purity + color preview |
| `TransferEditor` | `DynamicsSectionEditor` | Opacity/Flow (+ Mixer Wet/Load/Mix when a Mixer preset) |
| `BrushPoseEditor` | `QWidget` | Tilt X/Y, Rotation, Pressure + Override toggles |
| `ControlCombo` | `QComboBox` | Shared Control selector; emits the Fade spin state |

Widgets throughout — the panel is dense and desktop-oriented. `ControlCombo`
owns the single source of truth for which Control options are valid per attribute
(e.g. Angle-only `Initial Direction`/`Direction`), so validation is not duplicated.
The Mixer-only Transfer rows are shown conditionally based on tool/preset type.

## Data-model impact

- **No PSD fields.** Dynamics live in brush presets and session state only.
- **Preset serialization** uses the `.abr` descriptor keys (`BRU-006`):
  `useTipDynamics` + `szVr`/`minimumDiameter`/`angleDynamics`/
  `roundnessDynamics`/`minimumRoundness`/`tiltScale`; `useScatter`; `useTexture`
  + pattern UUID; `dualBrush`; `useColorDynamics` + `usePaintDynamics`; `Wtdg`,
  `Nose`, `Rpt `; curve descriptors (`inpt`, `grad`, `Cl  `, `Ofst`, `Type`,
  `Loc `, `Mdpn`). The community ABR analysis maps only ~40% of curve semantics —
  our preset model may store the parsed curves but must not claim full fidelity.
- **Undo:** unchanged from `BRU-001` — one state per stroke.
- **Tool presets vs brush presets:** dynamics belong to the brush preset;
  options-bar opacity/flow/mode belong to the tool preset (`BRU-006`).

## Edge cases

- **No tablet / missing pen feature.** Every pen-mapped Control falls back to Off
  (or Fade where selectable) and surfaces Adobe's warning affordance; values must
  not snap to 0.
- **Fade step count 0.** `Fade` with steps = 0 must be treated as Off or 1;
  define and test.
- **Minimum > base.** Minimum Diameter/Roundness above the base must clamp.
- **Scatter Count × low spacing.** Dab explosion — cap and warn.
- **Texture with no pattern.** The set is a no-op; no error.
- **Texture Each Tip off + Depth Jitter.** Depth variance is disabled (Help
  requires Texture Each Tip for Depth variance); Min Depth also inactive.
- **Apply Per Tip off.** Color is drawn once per stroke; changing
  foreground/background mid-stroke must not retroactively recolor the stroke.
- **Hue/Sat/Brightness in CMYK/Lab/Indexed.** Jitter is a color-model operation;
  define per model and refuse where undefined (Indexed/Bitmap).
- **Purity vs Saturation Jitter.** Purity clamps saturation; ensure ordering
  (jitter then purity, or the reverse) is fixed and documented.
- **Dual Brush with natural-media primary.** Bristle/erodible/airbrush tips may
  not be valid dual tips; UI should restrict.
- **Brush Pose with `Override` + a Control referencing that input.** The override
  supplies the constant signal; must not deadlock or double-apply.
- **Undo/redo determinism.** RNG seed and consumption order must be identical on
  replay.
- **GPU unavailable.** Dynamics still evaluate on CPU; only previews degrade.

## Parity acceptance criteria

- Given Size Jitter = 0%, Size Control = Off, dab size is constant over a stroke;
  at 100% jitter, size varies within [Minimum Diameter, base].
- Given Size Control = Pen Pressure and a pressure-capable tablet, increasing pen
  pressure monotonically increases dab size between Minimum Diameter and the base.
- Given Fade with N steps, the attribute reaches its minimum after about N dabs,
  starting from maximum at the stroke start.
- Given Angle Control = Direction, each dab's long axis follows the local stroke
  tangent; with Initial Direction it follows only the first tangent.
- Given Both Axes checked, scatter is radial; unchecked, marks spread
  perpendicular to the path.
- Given Count > 1, each spacing interval lays that many marks; Count Jitter varies
  it per interval.
- Given Texture Each Tip off, the pattern is fixed across the stroke; on, each
  dab samples the pattern independently and Depth Jitter becomes effective.
- Given Depth = 100%, texture low points receive no paint; at 0% the pattern is
  invisible.
- Given a Dual Brush preset, no paint appears outside the intersection of the two
  tips.
- Given Color Dynamics Apply Per Tip off, one stroke uses one color; on, stamps
  differ.
- Given Purity = −100%, output is fully desaturated; +100% fully saturated.
- Given Transfer Opacity Jitter = 100% with Pen Pressure, dab coverage tracks pen
  pressure and never exceeds the options-bar Opacity.
- Given Brush Pose Override on Pressure = 50%, the stroke behaves as if a
  constant 50% pressure were applied regardless of device input.
- Given a Brush Projection brush, tilting the stylus transforms the tip from
  circle toward ellipse.
- Given a saved preset, every dynamics setting round-trips through save/reload.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  Adobe Photoshop Help reference (downloaded and text-extracted). Established:
  Jitter and Control definitions; Shape Dynamics controls and the Control menus
  (Off/Fade/Pen Pressure/Pen Tilt/Stylus Wheel/Rotation, plus Initial
  Direction/Direction for Angle); Minimum Diameter/Roundness and Tilt Scale;
  Brush Projection (CS6); Scattering (Scatter/Count both with Jitter + Control,
  Both Axes); Texture options (Invert, Scale, Texture Each Tip, Mode, Depth,
  Minimum Depth, Depth Jitter + Control); Dual Brush options (Mode, Diameter,
  Spacing, Scatter, Count); Color Dynamics (Apply per tip, FG/BG Jitter, Hue/
  Saturation/Brightness Jitter, Purity); Transfer (Opacity/Flow Jitter and
  controls); Brush Pose (Tilt X/Y, Rotation, Pressure, Override); noise/wet
  edges/airbrush/smoothing/protect texture; the CS6 "Color Dynamics remain
  consistent for each stroke by default" note and the per-stroke→Apply Per Tip
  revert. Primary source.
- `https://community.wacom.com/en-co/complete-guide-to-photoshop-brushes-pt-3` —
  Wacom/C.S. Jones brush-settings guide. Established: Jitter vs Control
  interaction ("Control methods don't fully override the sliders"); Fade applying
  at stroke start; Direction vs Initial Direction; Flip X/Y Jitter; Brush
  Projection behavior; Scattering hints; Texture mode observations and
  Brightness/Contrast; Dual Brush behavior and mode quirks; Color Dynamics
  semantics; Transfer contents; Brush Pose Override; Build-up; Protect Texture.
  Secondary.
- `https://pslover.com/guides/the-ultimate-guide-to-photoshop-brush-settings` —
  brush-settings guide. Established: SHape Dynamics/Tone-Dynamics purposes;
  Dual Brush clipping; Texture/Scatter/Transfer roles; Smoothing. Secondary.

Not parsed: `https://helpx.adobe.com/photoshop/using/adding-dynamic-elements-brushes.html`
(HTTP 403).

## Open questions

- **Numeric ranges and defaults.** CS6 Help states directions and semantics but
  rarely numeric bounds. Defaults marked *(unverified)* (Minimum Diameter,
  Tilt Scale, Minimum Roundness, Scatter/Depth defaults, Brightness/Contrast
  ranges, Brush Pose angle ranges) need a CS6 build or `.abr` v10 dump.
- **Jitter × Control combine rule.** Whether control replaces, scales, or adds to
  jitter (and the exact curve shape) is undocumented; our `lerp` model is a
  design proposal, not Adobe data.
- **Fade semantics.** Help describes fading "from maximum to no/zero"; whether it
  fades from the *base* value or from the jitter maximum, and the step unit (dab
  vs stamp) needs confirmation.
- **Flip X/Y Jitter in CS6.** Present in the cited modern guide; the CS6 Help
  Shape Dynamics list does not enumerate it. Confirm CS6 presence.
- **Texture Mode list and per-mode math.** Help names "Mode (blending mode)" but
  does not enumerate modes or formulas; the Wacom guide documents behavior for a
  modern build only. Exact CS6 list and operators TBD.
- **Dual Brush operator.** The exact per-pixel intersection function is
  undocumented and reportedly not the layer-mode operator.
- **Hue/Sat/Brightness jitter model.** Whether they operate in HSB of the current
  color model or an internal Lab/HSB space is not stated.
- **Brush Pose ranges and whether Tilt/Pressure Override applies per stroke or per
  dab.** Resolve by experiment.
- **Default Enable states.** Which dynamics are on by default for the shipped CS6
  presets needs enumeration from a CS6 install.
