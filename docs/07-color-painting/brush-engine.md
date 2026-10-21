# Brush Engine

- **Spec ID:** `BRU-001`
- **Status:** `Draft`
- **Parity tier:** `Core` — the Brush and Pencil tools are in CS6 Standard; the same engine underlies every painting, erasing, toning, and focus tool.
- **New in CS6:** `Changed` — Erodible and Airbrush tip categories, Brush Pose, Brush Projection, per-stroke (rather than per-dab) Color Dynamics by default, texture Brightness/Contrast sliders, and the maximum brush size raised to 5000 px.
- **Depends on:** `ARCH-006` gpu-rendering-pipeline, `ARCH-008` document-model, `ARCH-009` undo-history, `03-tools/brush-and-pencil.md`, `03-tools/eraser-tools.md`, `07-color-painting/brush-dynamics.md` (`BRU-002`), `07-color-painting/bristle-brushes.md` (`BRU-003`), `07-color-painting/mixer-brush-engine.md` (`BRU-004`), `07-color-painting/airbrush-and-flow.md` (`BRU-005`), `07-color-painting/brush-presets.md` (`BRU-006`).

> Module and widget names are **design proposals**. The repository contains a
> working implementation; this file stays the behavioural contract for the
> feature, and `docs/dev/doc-code-parity-gaps.md` records what shipped and what
> did not. The exact Adobe sampling, compositing, and deformation kernels are
> closed; where they are not described by the CS6 Help they are marked
> **behavioral parity only, algorithm TBD**. Behavioral facts below are from
> the fetched CS6 Help PDF and the cited secondary sources; anything not
> verified is called out in Open questions rather than asserted.

## CS6 behavior

The brush engine converts a pointer path into a sequence of **dabs** (Help calls
them brush marks; community usage calls each stamped tip image a *tip stamp*)
painted into the active layer, using the current foreground/background color,
the selected brush preset, and the options-bar mode, opacity, flow, and airbrush
settings.

**Tools that share the engine.** CS6 lists the Brush, Pencil, Eraser, Background
Eraser, Magic Eraser, Clone Stamp, Pattern Stamp, History Brush, Art History
Brush, Smudge, Blur, Sharpen, Dodge, Burn, Sponge, and (via the same stamping
pipeline) the Mixer Brush and Airbrush tips. Each tool maps a subset of the
controls in the Brush panel; the Brush and Pencil are the reference cases.

**Tip categories.** The Brush Tip Shape section of the Brush panel selects and
edits one of:

- **Standard round/elliptical** — procedural tip with Size, Hardness, Roundness,
  Angle, Spacing, Flip X/Y.
- **Square** — Hardness applies (Help: "Available only for round and square
  brushes"); the preset picker's Hardness control is likewise limited to round
  and square.
- **Sampled** — created from an image selection via `Edit > Define Brush Preset`,
  up to **2500 × 2500 px**, converted to grayscale (any layer mask on the source
  does not affect the tip). Hardness cannot be changed on a sampled tip; edge
  softness is baked at define time (Feather). `Use Sample Size` resets to the
  original diameter.
- **Bristle** (CS5) — a deformable bristle tip; see `BRU-003`.
- **Erodible** (CS6) — pencils/pastels that wear down while drawing; Softness
  controls the rate of wear, Shape sets flat→round, Sharpen Tip restores
  crispness, and a Live Brush Tip Preview shows the wear. Erodible tips are
  **not** the Pencil tool's hard aliased line; they are a distinct tip type.
- **Airbrush** (CS6) — a conical 3D spray with Size, Hardness, Distortion,
  Granularity, Spatter Size, Spatter Amount, Spacing; pen pressure changes the
  spread. See `BRU-005`.

**Dab placement and spacing.** Spacing is a percentage of the brush diameter.
With `Spacing` checked, dabs are laid at the set interval. With it unchecked,
Help says "the speed of the cursor determines the spacing" — i.e. spacing
becomes pointer-velocity dependent. Increasing spacing makes the stroke skip
(separated marks). `[` / `]` adjust diameter; `Shift+[` / `Shift+]` adjust
hardness for hard round, soft round, and calligraphic brushes.

**Shape controls.** Size in pixels; Roundness as a percentage of the short/long
axis ratio (100% = circle, 0% = linear/elliptical crushed flat); Angle in
degrees rotates the ellipse or sampled tip; Flip X / Flip Y mirror the tip.
Hardness is the size of the hard center as a percentage of diameter; at 100% the
Brush tool is still anti-aliased, while the Pencil always paints a hard,
non-anti-aliased edge.

**Dynamic controls** modify Size, Angle, Roundness, Scatter, Count, color,
opacity, and flow over a stroke; they are specified in `BRU-002`. Two toggles
that are part of the engine rather than a dynamics section:

- **Noise** — adds randomness to individual tip pixels, most effective on soft
  (gray-valued) tips.
- **Wet Edges** — paint builds up along the stroke edges, giving a watercolor
  look.
- **Airbrush / Build-up** — applies gradual tones; the Brush panel toggle is the
  same flag as the options-bar Airbrush button. See `BRU-005`.
- **Smoothing** — smooths stroke curves (most useful for fast stylus strokes)
  at the cost of a slight rendering lag. The options bar exposes a Smoothing
  value and, in later builds, additional stroke-catch-up options.
- **Protect Texture** — forces the same pattern and scale on every textured
  brush preset, simulating one consistent canvas across multiple tips.

**Dual brush** composes a second tip inside the primary tip: "only the areas
where both brushstrokes intersect are painted." The secondary tip has its own
Mode, Diameter, Spacing, Scatter, and Count. See `BRU-002`.

**Tablet dynamics path.** With a pressure-sensitive tablet such as a Wacom, the
options bar offers **Tablet Pressure Controls Size** and **Tablet Pressure
Controls Opacity** buttons, which override the Brush panel size/opacity. The
Brush panel exposes additional controls driven by pen Pressure, Pen Tilt, Stylus
Wheel, and (Art Pen) Rotation; the help text warns with a triangle when the
selected input is unavailable (no tablet, or pen lacking the feature). CS6 adds
**Brush Pose** (lock tilt/rotation/pressure values or override device input) and
**Brush Projection** (tilt and rotation warp the tip shape). `HUD` brush-size
adjustment is available; with Control+Alt (Cmd+Option) drag it also changes
opacity unless the General preference "Vary Round Brush Hardness Based on HUD
Vertical Movement" is off.

**Painting gestures.** Click-drag paints; click start + Shift-click end paints a
straight line; holding the mouse button without dragging paints a build-up when
the Airbrush option is on. CS6 allows brush strokes to be recorded in actions
under `Allow Tool Recording` in the Actions panel menu.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Tools panel, Brush/Pencil slot | Tool | `B` (Brush), `B` cycles to Pencil | Pencil paints aliased hard edges |
| Options bar | Bar | — | Tool Preset picker, Brush preset picker, Mode, Opacity, Flow, Airbrush, tablet pressure buttons, Smoothing |
| Brush preset picker (options bar) | Pop-up panel | — | Diameter, Use Sample Size, Hardness; temporary until preset re-selected |
| Brush panel | Dock | `F5` / `Window > Brush` | Left list of option sets; right side options; stroke preview; lock/unlock tip |
| Brush Presets panel | Dock | `F5` / `Window > Brush Presets` | Preset thumbnails, panel menu for load/save/reset |
| Edit > Define Brush Preset | Menu | — | Creates a sampled tip from a selection (≤2500×2500) |
| Brush panel menu | Menu | — | New Brush Preset, Clear Brush Controls, Copy Texture to Other Tools |
| Preferences > Cursors | Dialog | — | Normal/Full-Size Brush Tip, Show Crosshair in Brush Tip, Show Only Crosshair While Painting; Caps Lock toggles |
| Actions panel menu | Menu | — | `Allow Tool Recording` records brush strokes |
| Keyboard | — | `[` / `]` | Decrease/increase brush diameter |
| Keyboard | — | `Shift+[` / `Shift+]` | Decrease/increase hardness (hard round, soft round, calligraphic) |
| Keyboard | — | `Shift+Alt+P` | Toggle the Airbrush option |
| Keyboard | — | number keys / `Shift`+number keys | Set opacity / flow (multiples of 10%) |
| HUD | On-canvas | Ctrl+Alt drag | Size/hardness; opacity with modifier (preference-dependent) |

## Parameters & ranges

Standard tip parameters (Brush Tip Shape). Ranges without a verified upper bound
are marked *(unverified)*.

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Size / Diameter | int px | preset | 1–5000 px (CS6 max 5000) | Sampled tips: `Use Sample Size` restores original |
| Hardness | int % | 100 | 0–100 | Round/square/sampled; not changeable on sampled tips |
| Spacing | int % + checkbox | 25 *(unverified default)* | 0–1000% *(unverified)* | Unchecked: pointer speed sets spacing |
| Angle | int deg | 0 | −180…180 *(unverified)* | Elliptical/sampled tip long axis from horizontal |
| Roundness | int % | 100 | 0–100 | 100=circle, 0=linear/elliptical |
| Flip X / Flip Y | checkbox | off | on/off | Mirrors tip about axis |
| Noise | checkbox | off | on/off | Randomizes individual tip pixels; best on soft tips |
| Wet Edges | checkbox | off | on/off | Edge build-up (watercolor) |
| Airbrush / Build-up | checkbox | off | on/off | Mirrors options-bar Airbrush |
| Smoothing | checkbox + value | off | 0–100 *(unverified)* | Adds slight stroke lag |
| Protect Texture | checkbox | off | on/off | Same pattern/scale across textured presets |
| Bristle tip | tip type | — | Shape, Bristles, Length, Thickness, Stiffness, Spacing, Angle | See `BRU-003` |
| Erodible tip | tip type | — | Size, Softness, Shape, Sharpen Tip, Spacing | CS6; Live Brush Tip Preview |
| Airbrush tip | tip type | — | Size, Hardness, Distortion, Granularity, Spatter Size, Spatter Amount, Spacing | CS6; see `BRU-005` |
| Brush Pose | subtree | — | Tilt X, Tilt Y, Rotation, Pressure + Override | CS6; see `BRU-002` |

Options-bar parameters shared by paint tools are specified in `03-tools/brush-and-pencil.md`; Mode, Opacity, Flow, Airbrush, and the tablet-pressure buttons are summarized in `BRU-005`.

## Algorithms & pipeline

The exact Adobe rasterizer is closed. The following is a **behavioral model** —
standard, widely used dab-splatting architecture consistent with what CS6
documents; exact Adobe kernels remain **algorithm TBD**.

Proposed pipeline:

1. **Input path** — collect pointer samples `(x, y, pressure, tilt_x, tilt_y,
   rotation, time)` at device rate. Apply **Smoothing** (a lag/EMA or spline
   filter over the input polyline) before dabbing.
2. **Stroke state** — on button-down, evaluate once per stroke the values that
   are *per-stroke* in CS6: with `Apply per tip` off, Color Dynamics pick a
   single foreground/background blend for the whole stroke; capture the RNG seed
   so redo is deterministic; reset the airbrush build-up accumulators.
3. **Path interpolation** — resample the smoothed polyline. When `Spacing` is
   checked the step is `spacing% × diameter`; when unchecked the step follows
   pointer speed so faster motion leaves wider gaps. For each step, compute the
   stroke tangent and (for spaced strokes) any carried-over remainder so spacing
   is continuous across samples.
4. **Per-dab evaluation** — resolve each dynamic attribute (size, angle,
   roundness, scatter offset, count, color, opacity, flow, texture depth) from
   its Jitter value and Control mapping. Controls are pen Pressure, Pen Tilt,
   Stylus Wheel, Rotation, Fade (1–9999 dabs), Initial Direction, or Direction;
   `Off` uses the raw jitter. Scatter offsets dabs radially (Both Axes) or
   perpendicular to the path.
5. **Tip sampling** — for a standard tip, sample a procedural hardness profile
   (hard center = hardness%, anti-aliased falloff) evaluated through the
   angle/roundness transform. For a sampled tip, sample its grayscale bitmap
   (re-scaled to Size; hardness fixed). Bristle, erodible, and airbrush tips use
   their own generators (`BRU-003`, `BRU-005`).
6. **Dual brush composition** — rasterize the secondary tip stamp and combine
   with the primary stamp using the Dual Brush Mode mask: paint only where both
   tips overlap (default multiply/darken-like intersection).
7. **Dab blending** — composite the tip alpha into the layer through the current
   tool Mode, the effective dab opacity, and dab flow. Within one held stroke,
   opacity is a cap per pixel (repeated coverage does not exceed the Opacity
   value); flow controls accumulation rate up to that cap. Airbrush build-up
   adds coverage while the pointer is stationary (`BRU-005`).
8. **Wet Edges / Noise** — Wet Edges redistributes coverage toward the rim of
   the stamped alpha; Noise perturbs per-pixel alpha of the tip.
9. **Texture and Protect Texture** — look up the pattern (CS6 `patt` data, loaded
   from `.pat`/library patterns or embedded in the brush preset), scale it, and
   modulate dab alpha/depth per the Texture Mode; `Texture Each Tip` applies it
   per stamp instead of across the whole stroke; `Protect Texture` pins the
   pattern+scale across presets.
10. **Commit** — accumulate the stroke into a scratch layer/tile set; on release
    emit one undo command carrying pre-edit tiles and the captured seed.

Performance note: high Scatter Count × low Spacing can create many dabs per
pixel; Help warns painting performance may decrease. The engine should cap dabs
per unit path and degrade gracefully.

## Rust module mapping

Proposals (crate names provisional):

- `pictura-brush::engine::BrushEngine` — orchestrates input → dabs → composite;
  owns `StrokeState` and the RNG.
- `pictura-brush::tip::Tip` — enum `{ Round, Square, Sampled(Arc<GrayBitmap>),
  Bristle(BristleTip), Erodible(ErodibleTip), Airbrush(AirbrushTip) }`; method
  `sample(transform) -> AlphaMask`.
- `pictura-brush::tip::GrayBitmap` — 8-bit grayscale tip (≤2500²) with optional
  PackBits decode from `.abr` (`BRU-006`).
- `pictura-brush::spacing::SpacingMode` — `{ Fixed(percent), VelocityDriven }`;
  a `Resampler` yields dab centers + tangents + residue.
- `pictura-brush::dynamics::Dynamics` — per-attribute `{ jitter: f32,
  control: Control }` (see `BRU-002`).
- `pictura-brush::composite::DabBlender` — mode + dab-opacity + flow compositing
  against a tile target; `BrushScratch` accumulator for opacity capping.
- `pictura-brush::wet_edges`, `pictura-brush::noise` — coverage post-processors.
- `pictura-brush::texture::TextureLayer` — pattern + mode + depth + scale;
  `ProtectTexture` state.
- `pictura-brush::smoothing::StrokeSmoother` — input filter.
- `pictura-input::TabletSample` — `{ x, y, pressure, tilt_x, tilt_y, rotation,
  wheel, time }` from the platform tablet backend.
- `pictura-core::command::BrushStroke` — emitted on release; stores tile diff +
  seed + preset id.

Boundary types: `StrokeConfig` (immutable per stroke), `TabletSample`,
`Dab`, `DabResult`. The engine writes only through `BrushStroke`, never directly
to document storage.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `BrushSettingsPanel` | `QWidget` dock | Hosts the option-set list + per-section editors; stroke preview |
| `BrushTipShapeEditor` | `QWidget` | Size/Hardness/Spacing/Angle/Roundness/Flip X/Y + editable preview |
| `BrushPresetPicker` | `QComboBox`/popup | Diameter + Hardness + Use Sample Size |
| `BrushPreview` | `QQuickItem` | Live stroke preview at panel bottom; re-renders on settings change |
| `DynamicsSectionEditor` | `QWidget` | Shared widget for Jitter + Control rows (`BRU-002`) |
| `BrushLockButton` | `QToolButton` | Per-section padlock (protect settings across preset changes) |
| `TabletPressureButtons` | `QToolButton` ×2 | Options-bar size/opacity pressure toggles |
| `BrushCursorOverlay` | `QQuickItem` | Normal/Full-Size tip cursor; crosshair options; HUD |

Widgets for the panel and options bar (dense, desktop-style); the stroke preview
and the on-canvas cursor overlay are QML/QQuick items because they redraw on the
GPU path and must not block the UI thread. The engine itself is Rust behind
`cxx-qt`; the Qt side sends `StrokeConfig` and pointer events and receives
preview frames.

## Data-model impact

- **No PSD fields for the live stroke.** Brush geometry, dynamics, RNG state, and
  scrub state are session-only.
- **Preset serialization.** A brush preset maps to the `.abr` descriptor
  vocabulary (`BRU-006`): tip params (`Dmtr`, `Hrdn`, `Angl`, `Rndn`, `Spcn`,
  `flipX`, `flipY`) and dynamics flags/curves (`useTipDynamics`, `szVr`,
  `minimumDiameter`, `angleDynamics`, `roundnessDynamics`, `dualBrush`,
  `useColorDynamics`, `usePaintDynamics`, `Wtdg`, `Nose`, `Rpt `…).
- **Undo:** one history state per completed stroke (paint tools default to
  per-stroke granularity). `BrushStroke` stores the pre-edit tile rectangle for
  bit-exact undo and the seed/config for deterministic redo.
- **Sampled tips** may be embedded in the preset (`samp` section); the document
  does not store them.

## Edge cases

- **No tablet.** All pen-controlled dynamics must fall back to `Off`/Fade and
  show the warning affordance Adobe shows; the brush must stay usable with a
  mouse.
- **Unavailable control.** Pen Tilt/rotation/wheel selected without the hardware
  must not silently zero the attribute; it should behave as `Off` plus a warning.
- **Sampled tip + hardness.** Hardness is inert on sampled tips; the UI must
  disable or ignore it.
- **Spacing unchecked + discrete input.** Velocity-driven spacing must degrade
  sensibly on low-rate or teleporting pointer input (avoid zero/negative steps).
- **Spacing 0 / very low.** Coincident dabs must not blow up the dab count;
  clamp to a minimum geometric step.
- **Huge PSB pointer travel.** Stream dabs tile-locally; never allocate a
  canvas-sized scratch for a stroke.
- **Bit depth / color model.** Painting is defined for 8/16/32-bpc RGB, Grayscale,
  CMYK, Lab, and (Indexed/Bitmap) 1-channel documents; mode set and channel
  writes differ per model. The Art History Brush and some tools are excluded at
  16/32 bpc — the generic engine must expose per-tool depth support.
- **GPU unavailable.** CPU dab splatter fallback; brush preview/OpenGL bristle
  preview degrade to static.
- **Undo mid-stroke.** Strokes are atomic on release; canceling a stroke restores
  scratch without adding history.
- **Protect Texture with no pattern.** No-op, not an error.
- **Seed and randomness.** Two identical strokes should differ (randomness) but
  redo must reproduce exactly.

## Parity acceptance criteria

- Given the Brush tool and a standard round tip, a dragged stroke produces dabs
  at `spacing% × diameter` intervals; increasing Spacing visibly separates the
  dabs, and unchecking Spacing makes spacing depend on pointer speed.
- Given Hardness = 100%, the Brush edge is anti-aliased (not a 1-px staircase),
  while the Pencil always produces a non-anti-aliased hard edge.
- Given a sampled tip, `Use Sample Size` restores its original pixel diameter and
  the Hardness control has no effect.
- Given a custom tip defined from a selection larger than 2500×2500, the define
  operation is refused or clamped.
- Given a pressure-sensitive tablet and `Tablet Pressure Controls Size`, harder
  pen pressure yields larger dabs; with the opacity button, it yields higher
  opacity, capped by the Opacity value.
- Given Opacity = 33% and Flow = 33%, repeated passes with the button held do not
  exceed 33% coverage; releasing and re-stroking adds another 33% step.
- Given Airbrush on, holding the pointer stationary over one spot builds coverage
  up to the Opacity cap.
- Given Wet Edges on, a stroke has darker edges than its center versus Wet Edges
  off.
- Given Noise on with a soft tip, dab alpha is visibly perturbed; with a hard tip
  the effect is negligible.
- Given Dual Brush enabled, paint appears only where the primary and secondary
  tips intersect.
- Given texture + `Texture Each Tip`, each dab carries its own texture; unchecking
  it applies the pattern across the whole stroke; `Protect Texture` keeps the
  same pattern/scale after switching presets.
- Given a completed stroke, undo restores every touched pixel bit-exactly and
  exactly one history state is added.
- Given a recorded brush action and `Allow Tool Recording`, replaying reproduces
  the stroke.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  Adobe Photoshop Help reference (downloaded and text-extracted). Established:
  the Brush panel and its option sets; standard tip options (Size, Use Sample
  Size, Flip X/Y, Angle, Roundness, Hardness, Spacing); sampled tips ≤2500×2500
  converted to grayscale and hardness-locked; `[`/`]` and `Shift+[`/`Shift+]`
  behavior; bristle, erodible, and airbrush tip categories and their controls;
  Brush Pose; Other brush options (Noise, Wet Edges, Airbrush/Build-up,
  Smoothing, Protect Texture); the graphics-tablet path and tablet pressure
  buttons; per-stroke Color Dynamics default; CS6 new-brush list including the
  5000 px maximum. Primary source.
- `https://community.wacom.com/en-co/complete-guide-to-photoshop-brushes-pt-3` —
  C.S. Jones / Wacom, "The complete guide to Adobe Photoshop brushes, Part 3".
  Established: the dab/tip-image vs tip-stamp vocabulary; Jitter/Control
  semantics; Spacing 1%–100%+ behavior; Direction/Initial Direction; Flip X/Y
  jitter; Brush Projection; Noise and Wet Edges descriptions; Dual Brush
  intersection behavior and notes; Color Dynamics (Apply per tip, Purity, hue/
  saturation/brightness); Transfer (Opacity/Flow jitter); Brush Pose override;
  Build-up toggle; Protect Texture and per-section padlocks. Secondary (modern
  Photoshop; behavior stable since CS5/CS6).
- `https://pslover.com/guides/the-ultimate-guide-to-photoshop-brush-settings` —
  "The Ultimate Guide to Photoshop Brush Settings". Established: panel layout
  and category list; Shape Dynamics purpose (Size Jitter, Minimum Diameter,
  Angle/Roundness Jitter, Flip X/Y, Brush Projection); Dual Brush clipping to
  the main tip; Texture/Scatter/Transfer roles; Smoothing and its interaction
  with spacing. Secondary (modern).
- `https://design.tutsplus.com/tutorials/new-brush-features-in-photoshop-cs6--psd-16508`
  — Martin Perhiniak, "New Brush Features in Photoshop CS6". Established only
  the framing that CS6 added Live Pen Tilt Preview, Brush Projection, Erodible
  and Airbrush tips (article body is largely a video). Secondary.
- `https://raw.githubusercontent.com/darkly-art/darkly/dev/docs/brush/abr-format.md`
  — "ABR Format: Public Analysis". Established: the brush parameter
  vocabulary and descriptor keys used for preset serialization (tip, dynamics,
  scatter, texture, dual brush, color/paint dynamics, wet edges/noise/protect
  texture). Community analysis, not Adobe.
- `https://christianlim.wordpress.com/bristle-brush-tip-settings` — bristle tip
  setting walkthrough. Established: how Bristles/Length/Thickness/Stiffness/
  Angle/Spacing alter a bristle stroke, and that bristle behavior is coupled to
  Mixer Brush wetness. Secondary.

Not parsed: `https://helpx.adobe.com/photoshop/using/adding-dynamic-elements-brushes.html`
(helpx.adobe.com returns HTTP 403).

## Open questions

- **Default Spacing value and its exact range in CS6.** The Help describes spacing
  only as "a percentage of the brush diameter" without stating the default or
  upper bound (the `.abr` v1 spec field is 0–999). Resolve from a CS6 build or a
  v10 sub1 `.abr` dump.
- **Exact dab-compositing math (per-stroke opacity cap vs flow accumulation).**
  The Help gives the intent ("does not exceed the set level… flow builds up…
  up to the opacity setting") but not the formula. Resolve by measuring coverage
  on controlled strokes or by behavioral parity.
- **Wet Edges kernel.** How edge accumulation is computed is undocumented;
  algorithm TBD.
- **Noise distribution.** Whether Noise is per-pixel alpha perturbation, a
  per-dab offset, or both, is not stated. Resolve by experiment.
- **Sampled-tip resampling filter.** The interpolation used when scaling a
  sampled tip to Size is undocumented.
- **Brush-stroke spacing carry-over.** How residual distance is carried across
  irregular pointer samples (to keep uniform spacing on fast strokes) is a
  design choice here, not an Adobe-verified detail.
- **Which CS6 tools share per-stroke vs per-dab defaults.** Help implies paint
  tools default to per-stroke opacity and Color Dynamics; confirm per tool.
- **Rolling-help caution.** The archived PDF is a rolling "Adobe Photoshop Help"
  (cover dated February 2013, PDF modified 2017) and contains some Creative
  Cloud-marked material. All CS6 feature attributions used here were cross-checked
  against CS6-era secondary sources, but a few ranges/defaults still need a CS6
  binary to confirm.
