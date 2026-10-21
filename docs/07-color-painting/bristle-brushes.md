# Bristle Brushes

- **Spec ID:** `BRU-003`
- **Status:** `Draft`
- **Parity tier:** `Core` — bristle tips ship with CS6 Standard and are used by both the Brush and Mixer Brush tools.
- **New in CS6:** `Changed` (inherited) — bristle tips themselves are CS5; in CS6 they gain Brush Pose and Brush Projection and the 5000 px maximum, and their live preview participates in the CS6 pen-tilt preview.
- **Depends on:** `BRU-001` brush-engine, `BRU-002` brush-dynamics, `BRU-004` mixer-brush-engine, `BRU-005` airbrush-and-flow, `BRU-006` brush-presets, `ARCH-006` gpu-rendering-pipeline.

> Module and widget names are **design proposals**. The repository contains a
> working implementation; this file stays the behavioural contract for the
> feature, and `docs/dev/doc-code-parity-gaps.md` records what shipped and what
> did not. Adobe's bristle simulation is closed; this spec is **behavioral
> parity only, algorithm TBD**. Facts from the CS6 Help PDF and the cited
> CS5/CS6 secondary sources; unverified claims are listed under Open questions.

## CS6 behavior

A **bristle tip** is a distinct tip category whose marks are drawn as bundles of
individual bristles rather than a single grayscale stamp. Help: "Bristle tips let
you specify precise bristle characteristics, creating highly realistic,
natural-looking strokes." The category was introduced in CS5 alongside the Mixer
Brush and is most often paired with it, but bristle tips are also selectable as
ordinary Brush tool tips.

Bristle tips appear in the Brush panel's Brush Tip Shape section with their own
option set (the CS6 Help calls these the **Bristle tip shape options**). They are
visually distinguished in the preset picker by previews that look like real
brush heads; a subset ships as default presets (a CS5/CS6-era tutorial counts ten
loaded by default *(unverified)*).

Key CS6 behaviors:

- **Shape** — the overall arrangement of bristles. Secondary sources document
  both round and flat families (round, round fan, round curve, flat, flat fan,
  flat curve) and the presence of fan and angled variants; the exact CS6
  enumeration is not in the fetched primary source (see Open questions).
- **Bristles** (%), **Length**, **Thickness**, **Stiffness** — density, bristle
  length, individual bristle width, and flexibility. Low Stiffness deforms the
  brush shape easily; high Stiffness keeps it rigid and, per a tutorial, drags
  little paint across the canvas. Help explicitly notes: "To vary stroke creation
  when using a mouse, adjust the stiffness setting."
- **Spacing** (%) — distance between marks as a percentage of tip diameter; when
  unchecked, pointer speed determines spacing (same rule as standard tips).
- **Angle** — the tip angle when painting with a mouse (with a tablet, tilt and
  rotation can drive it).
- **Brush preview** — reflects the settings plus current pressure and stroke
  angle, and can be rotated to view from different sides. **Bristle previews
  require OpenGL** in CS6; without it only a static representation is available.
- **Brush Pose (CS6)** — tilt/rotation/pressure can be locked or overridden, and
  the effects are visible in the Live Tip Brush Preview.
- **Brush Projection (CS6)** — stylus tilt/rotation warp the tip shape.
- **Mixer Brush coupling** — bristle strokes interact strongly with Mixer Brush
  wetness; a tutorial stresses that "we ALSO have to factor in our mixer brush
  settings. Obviously having a wet brush will carry over more color than a dry
  brush." Mixer-specific modes (Dry/Wet/Moist presets) change whether a bristle
  stroke blends existing paint or lays down crisp edges.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Brush panel > Brush Tip Shape | Options | `F5` | Bristle-specific Shape/Bristles/Length/Thickness/Stiffness/Spacing/Angle |
| Brush panel preview | Widget | — | Rotatable 3D-ish preview; OpenGL required |
| Brush panel > Shape Dynamics | Options | — | Brush Projection (CS6) for tilt/rotation tip warping |
| Brush panel > Brush Pose | Options | — | CS6 tilt/rotation/pressure override; feeds the live preview |
| Brush Presets panel | Dock | `F5` / `Window > Brush Presets` | Default bristle presets grouped with other tips |
| Options bar | Bar | — | Mixer Brush Wet/Load/Mix/Flow when a bristle tip is used with the Mixer |
| Live Tip Brush Preview | On-canvas | — | CS6; shows bristle wear/pose at upper-left while painting |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Size | int px | preset | 1–5000 px (CS6 max) | Tip diameter |
| Shape | enum | Round *(unverified)* | Round/Flat × plain/fan/curve (+angled/fan variants) | Overall bristle arrangement; exact CS6 list unverified |
| Bristles | int % | preset | 0–100% *(unverified)* | Bristle density |
| Length | int % | preset | 0–100% *(unverified)* | Bristle length; longer carries more paint |
| Thickness | int % | preset | 0–100% *(unverified)* | Individual bristle width |
| Stiffness | int % | preset | 0–100% *(unverified)* | Low = flexible/deforming; high = rigid, little drag |
| Spacing | int % + checkbox | 25 *(unverified)* | 0–1000% *(unverified)* | Unchecked: pointer speed sets spacing |
| Angle | int deg | 0 | −180…180 *(unverified)* | Tip angle for mouse input |
| Brush Projection | checkbox | off | on/off | CS6; tilt/rotation warp tip |
| Brush Pose Tilt X/Y, Rotation, Pressure | int + Override | device | see `BRU-002` | CS6 |
| Hardness | — | n/a | — | Not a bristle parameter (bristles provide the edge) |
| Mixer Wet/Load/Mix/Flow | int % | per Mixer preset | 0–100 | See `BRU-004` |

## Algorithms & pipeline

Bristle rendering differs fundamentally from stamped brushes:

- **A standard tip is one bitmap/procedural alpha mask composited repeatedly**
  (`BRU-001`). A **bristle tip is a set of individual bristle tracks** distributed
  across the tip profile; each track is drawn along the stroke (or as a local
  deformation field) with its own width and offset. The visible mark is the union
  of those tracks, so edges are ragged and internal streaks appear.
- **Stiffness controls deformation.** Each bristle's lateral offset bends in
  response to stroke direction, pressure, and (with Brush Projection) tilt/
  rotation. Low Stiffness = large bend / shape collapse; high Stiffness = the
  bristle tracks stay on their radial lines and drag little paint.
- **Bristles / Length / Thickness** set track count, track length, and track
  width. Longer bristles and higher density carry/transfer more paint per stroke
  and produce longer streaks; thicker bristles give coarser, more separated
  marks.
- **Shape** sets the initial distribution of track roots (a round head vs a flat
  head, with fan/curve profiles spreading the tracks); this is what makes a fan
  brush shed widely and a flat brush lay a broad chisel edge.
- **Spacing** steps the bristle stamp along the path like any other tip, but the
  per-stamp deformation is what makes bristle strokes look continuous and
  streaked rather than dotted.
- **Mixer coupling** adds a per-bristle paint reservoir/pickup model: tracks carry
  loaded color and pick up canvas color according to Wet/Load/Mix/Flow
  (`BRU-004`). Dry settings keep tracks crisp; wet settings smear and blend.
- **Brush Pose / Projection** feed the tilt/rotation into the same deformation
  term, so a tilted pen flattens and redirects the bristle fan.
- **Color dynamics in the bristle context** applies per-bristle or per-stamp as
  in `BRU-002`; with the Mixer, the reservoir color and canvas pickup dominate
  before color jitter is applied.

Exact Adobe deformation physics (spring model, per-bristle integration, paint
transfer rates) are **algorithm TBD**.

Proposed model: represent the tip as `N` bristle roots on a profile curve; each
root has a rest offset and width; per dab, offset by `direction · compliance(1 −
stiffness) · f(pressure, tilt)`. Composite the union of the resulting tapered
tracks. Seed the per-bristle randomness so redo is stable.

## Rust module mapping

- `pictura-brush::tip::BristleTip` — `{ shape: BristleShape, bristles: f32,
  length: f32, thickness: f32, stiffness: f32, angle: f32 }`.
- `pictura-brush::tip::BristleShape` — enum of round/flat × plain/fan/curve
  families; a `profile(t)` generator.
- `pictura-brush::tip::BristleTrack` — `{ rest_offset, width, seed }`; per-dab
  `deform(dir, pressure, tilt, rotation) -> [Point]`.
- `pictura-brush::render::BristleRasterizer` — draws the union of tracks into the
  dab alpha; optionally GPU-accelerated.
- `pictura-brush::mixer` — bristle/mixer paint transfer (shared with `BRU-004`).
- `pictura-core::command::BrushStroke` — as in `BRU-001`.

Boundary types: `BristleShape`, immutable `BristleTip`, per-dab `DeformInput`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `BristleTipEditor` | `QWidget` | Shape picker, Bristles/Length/Thickness/Stiffness sliders, Spacing, Angle |
| `BristleShapeCombo` | `QComboBox` | Shape list with small preview icons |
| `BristlePreview3D` | `QQuickItem` | Rotatable tip preview; requires OpenGL/QRhi, falls back to a 2D thumbnail |
| `BrushPoseEditor` | `QWidget` | Shared with `BRU-002`; feeds the preview |
| `LiveTipPreview` | `QQuickItem` | CS6 on-canvas live tip/wear preview |

The rotatable preview is a QQuick item rendered through Qt's RHI (matching the
Mercury/OpenGL requirement); a static label is the no-GPU fallback. The rest are
widgets.

## Data-model impact

- **No PSD fields.** Bristle ints are preset state.
- **Preset serialization:** tip params use the standard keys plus bristle-specific
  values in the `.abr` descriptor; the exact bristle descriptor keys are not
  fully mapped by the community ABR analysis (`BRU-006`), so round-trip fidelity
  of bristle presets is an open risk.
- **Undo:** one state per stroke (`BRU-001`).
- **OpenGL dependency:** unlike standard tips, the live bristle preview has a
  rendering-capability requirement that must be recorded as a capability flag,
  not a document property.

## Edge cases

- **No OpenGL / GPU unavailable.** Bristle strokes must still paint on the CPU;
  only the interactive 3D preview falls back to a static thumbnail.
- **Stiffness extremes.** Stiffness 0 must not make tracks collapse to a point
  (clamp deformation); stiffness 100 must still produce a mark.
- **Bristle count 0 / Bristles % very low.** Define a minimum one-bristle fallback
  so a stroke is never invisible.
- **Dual Brush with a bristle tip.** Whether a bristle tip may be a dual (primary
  or secondary) tip is unverified; restrict or degrade.
- **Mixer on an empty/transparent layer.** Pickup well samples nothing; behavior
  should match the Mixer edge cases (`BRU-004`).
- **Huge tips.** 5000 px bristle tips with high bristle counts are expensive;
  prune off-canvas tracks and cache the profile.
- **CMYK/Lab/Indexed.** Bristle compositing is a color-model operation; refuse or
  degrade where undefined (Indexed/Bitmap).
- **Undo/redo.** Per-bristle seeds must replay deterministically.
- **Sampled vs bristle.** Defining a preset from an image yields a sampled tip,
  not a bristle tip; the two must not be conflated.

## Parity acceptance criteria

- Given a bristle tip, a stroke shows individually resolvable bristle streaks
  (not a single smooth gradient), and changing Bristles/Length/Thickness/
  Stiffness changes the streak count, length, width, and deformation visibly.
- Given Stiffness low, repeated strokes at different directions deform the tip
  shape; at high Stiffness the tip shape is stable.
- Given a flat vs round Shape, the mark profile is chisel-like vs round.
- Given Spacing increased, the bristle marks separate; unchecked, faster pointer
  motion widens the gap.
- Given the Mixer Brush with a bristle tip, increasing Wet increases how far
  canvas color is dragged and reduces crisp edge retention; Dry presets leave
  hard-edged, unblended bristle strokes.
- Given a stylus with tilt and Brush Projection, tilting the pen elongates and
  redirects the bristle fan.
- Given no OpenGL, bristle strokes still paint correctly and the preview falls
  back to a static image.
- Given a saved bristle preset, all bristle settings survive save/reload.
- Given a completed bristle stroke, undo restores touched pixels bit-exactly.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  Adobe Photoshop Help reference (downloaded and text-extracted). Established:
  the Bristle tip shape options (Shape, Bristles, Length, Thickness, Stiffness,
  Spacing, Angle), the "vary stroke creation with a mouse via stiffness" note,
  the rotatable brush preview and its OpenGL requirement, and that bristle tips
  were introduced in CS5 (the CS5 What's-new "Extraordinary painting effects"
  entry). Primary source.
- `https://christianlim.wordpress.com/bristle-brush-tip-settings` — bristle tip
  setting walkthrough. Established: bristle presets ship in a set (counted as ten
  by the author); Size/Shape/Bristles/Length/Thickness/Stiffness/Angle/Spacing
  behavior; longer bristles carry more color; low Thickness gives finer tips;
  Stiffness rigid vs flexible; spacing on/off and its dabbing effect; strong
  coupling to Mixer Brush wetness. Secondary.
- `https://glensmith.co.uk/photoshop/mixer-brush` — Mixer Brush walkthrough.
  Established: Mixer tool options, use with bristle and wet-media brushes, and
  that the same Wet/Load/Mix/Flow controls govern how bristle strokes blend;
  brush presets record tool settings. Secondary.
- `https://community.wacom.com/en-co/complete-guide-to-photoshop-brushes-pt-3` —
  Wacom brush guide. Established: natural-media brush category framing and that
  these tips expose medium-specific settings in place of the standard ones.
  Secondary.

## Open questions

- **Full CS6 Bristle Shape enumeration.** The primary source lists no shapes.
  Secondary material references round/flat, fan, and curve variants; the exact
  ordered list and defaults need a CS6 build.
- **Default bristle values** (Bristles/Length/Thickness/Stiffness ranges and
  defaults) are not in the primary source; marked *(unverified)*.
- **Number and names of shipped CS6 bristle presets.** One tutorial says ten;
  confirm.
- **Deformation model.** The bristle physics (spring stiffness, pressure→bend,
  tilt mapping) is undocumented — algorithm TBD.
- **Paint transfer per bristle.** How the Mixer reservoir/pickup distributes
  across individual bristles is unverified.
- **Bristle + dynamics interaction.** Whether all of `BRU-002` applies to bristle
  tips (e.g. Roundness jitter may be meaningless) needs confirmation.
- **`.abr` bristle descriptor keys.** Community ABR parsing does not fully map
  bristle parameters; round-trip fidelity of bristle presets is unproven.
