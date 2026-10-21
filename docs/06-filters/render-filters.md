# Render Filters

- **Spec ID:** `FILT-060`
- **Status:** `Draft`
- **Parity tier:** `Core` (Clouds, Difference Clouds, Fibers, Lens Flare, Lighting Effects) + `Core` for the CS6 **Scripted Patterns** fill; **Flame / Tree remain non-goals for CS6 parity** (they do not exist in CS6). **Picture Frame** is a post-CS6 (CC 2014.2) entry that Kooka ships by decision (#227) and is likewise out of CS6 parity.
- **New in CS6:** `Yes` — the **Lighting Effects gallery** is rebuilt as a 64-bit workspace with on-canvas controls and GPU support; **Scripted Patterns** (`Edit > Fill > Use: Pattern > Scripted Patterns`) is new in CS6. **Flame, Tree, and Picture Frame were added in Photoshop CC 2014.2, not CS6** — see "Post-CS6 render entries". Kooka surfaces Picture Frame as a deliberate addition.
- **Depends on:** `06-filters/filters-overview.md`, `06-filters/lighting-effects.md`, `06-filters/blur-filters.md`, `06-filters/texture-filters.md`, `05-layers/fill-layers.md`, `07-color-painting/pattern-presets.md`, `01-architecture/gpu-rendering-pipeline.md`, `01-architecture/document-model.md`, `05-layers/smart-filters.md`, `04-image-ops/bit-depth-and-conversion.md`

> All crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Behavior is from the fetched CS6 Help unless marked
> *(inferred)*. Adobe's noise functions, flare model, and lighting shader are
> closed and marked **behavioral parity only, algorithm TBD**.

## CS6 behavior

`Filter > Render` creates 3D shapes, cloud and refraction patterns, and simulated light reflections. The CS6 `Render` submenu contains **Clouds, Difference Clouds, Fibers, Lens Flare, Lighting Effects** (and, in CS6, no Flame/Tree/Picture Frame — see below).

| Filter | Menu path | What it does (CS6 Help) |
|---|---|---|
| Clouds | `Filter > Render > Clouds` | Soft cloud pattern from random values between foreground and background colors; `Alt`/`Option`-choose for a starker pattern. **Replaces the image data on the active layer.** |
| Difference Clouds | `Filter > Render > Difference Clouds` | Random values between foreground/background blended with the existing pixels using **Difference** mode. First run inverts portions in a cloud pattern; repeated runs create marble-like ribs and veins. **Replaces the image data.** |
| Fibers | `Filter > Render > Fibers` | Woven-fiber look from foreground/background colors. **Variance** controls color variation (low = long streaks, high = short, varied fibers); **Strength** controls the weave (low = loose, high = short stringy). **Randomize** re-rolls; **replaces the image data.** |
| Lens Flare | `Filter > Render > Lens Flare` | Simulates light refraction in a camera lens; set the flare center (click or drag crosshair), brightness, and lens type. |
| Lighting Effects | `Filter > Render > Lighting Effects` | CS6 gallery: 17 light styles, three light types, four sets of light properties, and grayscale **bump maps**. **RGB only**, requires a supported video card (GPU). |

### Scripted Patterns (CS6-new)

`Edit > Fill`, then `Use: Pattern`, choose a pattern, and tick **Scripted Patterns** to pick one of five scripts from the **Script** menu. Help describes them as "geometric fill patterns". The five CS6 scripts are **Brick Fill, Cross Weave, Random Fill, Spiral, and Symmetry Fill**. They are **not** a `Filter > Render` command; they live inside the Fill dialog. *(CS6-new — Help What's-New and the Planet Photoshop walkthrough.)*

### Post-CS6 render entries (documented for scope, not parity)

**Flame, Picture Frame, and Tree are not in shipped Photoshop CS6.** They were added in the **Photoshop CC 2014.2 update**, whose release notes list three new entries in the `Filter > Render` menu: Flame, Picture Frame, and Tree. Adobe's community answer is explicit that CS6 lacks `Filter > Render > Flame` and that it arrived only after Photoshop became Photoshop CC. The CS6 Help PDF contains no `Flame`, `Tree`, or `Picture Frame` render entries. They are recorded here only so downstream specs do not accidentally assume CS6 parity; the task note that Picture Frame came from Adobe Exchange could not be verified (see Open questions).

- **Flame** (CC 2014.2): flames generated along one or more user paths (paths must be 50–50,000 px; not Smart-Object compatible). Controls include Flame Type, Length, Width, Angle, Interval, Flame Lines, Turbulent/Jag, Opacity, Bottom Alignment, style (Normal/Violent/Flat), shape (Parallel/To The Center/Spread/Oval/Pointing), custom color, quality, Randomize Shapes. *(sourced from the CC 2014.2 update guide, not CS6.)*
- **Trace/Tree** (CC 2014.2): procedural tree rendering. *(sourced as existing; detailed behavior out of CS6 scope.)*

**Picture Frame is shipped by Kooka (#227), a deliberate post-CS6 addition.** It draws a decorative border over the layer from procedural geometry: 47 frames, each a recipe over the same three ingredients — a **vine** in one colour, **flowers** from a catalogue of 22 shapes in another, and **leaves** from 23 in a third — or, for frames 36–47, ruled and moulded borders drawn from a corner distance field. The Basic controls are Frame, Vines colour, Margin, Size, Arrangement, Flower (+ colour and size), and Leaf (+ colour and size); the Advanced controls are Number of Lines, Thickness, Angle, Fade, and Invert. The model is ported from photorust (`core/src/filters/frame.rs`, GPL-3.0); all artwork is procedural parametric geometry, with no traced Adobe asset. It is a `Filter ▸ Render ▸ Picture Frame…` entry with its own dialog (a frame thumbnail picker, a preview, and Basic / Advanced pages); the arrangement is a seed, so the result is deterministic. *(inferred family; CC 2014.2 presence sourced.)*

### Halftone Pattern is not Render

Help lists Halftone Pattern (which simulates a halftone screen) under **Sketch**, not Render. It belongs in `06-filters/sketch-filters.md`; noted here only to prevent mis-categorization.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Filter > Render > Clouds` | Menu | — | No dialog; `Alt`/`Option` for starker clouds |
| `Filter > Render > Difference Clouds` | Menu | — | No dialog; blends via Difference |
| `Filter > Render > Fibers` | Menu / modal dialog | — | Variance, Strength, Randomize |
| `Filter > Render > Lens Flare` | Menu / modal dialog | — | Flare center, Brightness, Lens Type |
| `Filter > Render > Lighting Effects` | Menu / workspace | — | CS6 gallery; GPU; RGB only |
| `Edit > Fill` → `Use: Pattern` → `Scripted Patterns` | Dialog | `Shift+F5` | Five scripts; CS6-new |
| `Edit > Define Pattern` | Menu | — | Feeds Scripted Patterns |
| `Filter > Render > Flame / Tree` | Menu | — | **Not CS6** (CC 2014.2+), unsurfaced |
| `Filter > Render > Picture Frame` | Menu / modal dialog | — | Post-CS6 (CC 2014.2), **shipped by Kooka** (#227); frame picker + Basic / Advanced |
| Smart Filter row | Layers panel | — | Fibers, Clouds, Difference Clouds, Lens Flare are Smart-Filter capable (see bit-depth list); Lighting Effects is a workspace |

## Parameters & ranges

Ranges marked **[AS]** come from the Photoshop CS6 AppleScript Scripting Reference. Ranges marked **[Help]** come from the CS6 Help. Fibers is not exposed as a scriptable filter class, and Help does not state its numeric limits, so those are *(inferred)*.

| Filter | Control | Type | Default | Range / options | Source |
|---|---|---|---|---|---|
| Clouds | — | — | — | No options | [Help] |
| Difference Clouds | — | — | — | No options | [Help] |
| Fibers | Variance | int | 16 *(inferred)* | range not stated in Help; *(inferred)* 1–64 | Help (control), range *(inferred)* |
| Fibers | Strength | int | 4 *(inferred)* | range not stated in Help; *(inferred)* 1–64 | Help (control), range *(inferred)* |
| Fibers | Randomize | button | — | Re-rolls the pattern | [Help] |
| Lens Flare | Brightness | int % | 100 *(inferred)* | 10–300% | [AS] |
| Lens Flare | Flare Center | point | image center *(inferred)* | x/y in image coordinates (unit value) | [AS] |
| Lens Flare | Lens Type | enum | 50–300mm Zoom *(inferred)* | 50–300mm Zoom / 35mm Prime / 105mm Prime / Movie Prime (AS: zoom / Prime 35 / Prime 105 / Movie Prime) | [AS][Help] |
| Lighting Effects | Preset | enum | Default | 17 styles (2 o'clock Spotlight, Blue Omni, Circle Of Light, Crossing, Crossing Down, Default, Five Lights Down/Up, Flashlight, Flood Light, Parallel Directional, RGB Lights, Soft Direct Lights, Soft Omni, Soft Spotlight, Three Down, Triple Spotlight) | [Help] |
| Lighting Effects | Light type | enum | Spot *(inferred)* | Point / Infinite / Spot | [Help] |
| Lighting Effects | Max lights | int | 1–16 | max 16 lights; only one editable at a time | [Help] |
| Lighting Effects | Intensity | int | ~50 normal | −100 (none) to 100 (brightest) | [Help] |
| Lighting Effects | Color | color | white *(inferred)* | per-light color | [Help] |
| Lighting Effects | Colorize | color/toggle | off | tint the overall lighting | [Help] |
| Lighting Effects | Exposure | int | 0 | increase/decrease light; 0 = no effect | [Help] |
| Lighting Effects | Gloss | slider | 0 *(inferred)* | Matte → Shiny | [Help] |
| Lighting Effects | Metallic | slider | 0 *(inferred)* | Plastic (light color) → Metallic (object color); CS6 labels it "Metallic", CS5 "Material" | [Help] |
| Lighting Effects | Ambience | int | 0 | 100 = light source only; −100 = remove the light source | [Help] |
| Lighting Effects | Texture (bump map) | channel | none | Red/Green/Blue or an alpha channel | [Help] |
| Lighting Effects | Height | slider | 0 | Flat (0) → Mountainous (100) | [Help] |
| Scripted Patterns | Pattern | preset | current | any pattern preset/library | [Help] |
| Scripted Patterns | Script | enum | Brick Fill *(inferred)* | Brick Fill / Cross Weave / Random Fill / Spiral / Symmetry Fill | [Help][Planet Photoshop] |

**Bit-depth gate (Help "Filter basics").** The 16-bit list includes **Fibers, Clouds, Difference Clouds, Lens Flare**. The 32-bit list includes **Clouds and Lens Flare**. Lighting Effects works only on **RGB** images and requires a supported GPU. Scripted Patterns fills operate in the document's normal fill path. (Sourced.)

## Algorithms & pipeline

**Behavioral parity only, algorithm TBD** where Adobe does not document the generator.

| Filter | Algorithm family | Notes |
|---|---|---|
| Clouds | Fractal/value noise | Random field over the selection, colored by interpolating foreground↔background; `Alt` produces a starker (higher-contrast or different-octave) pattern. Adobe's noise is closed. Replaces the layer's pixels (no blend with source). *(inferred family)* |
| Difference Clouds | Fractal noise + Difference blend | Same noise generator as Clouds, then blend with the existing pixels using the Difference formula (per `05-layers/blend-modes.md`). Replaces the layer's pixels. *[Help]* |
| Fibers | Directional/anisotropic noise | Value noise stretched down the picture in two layers: soft clumps tens of pixels wide and hard-edged hairs one or two pixels wide, contrast-stretched to blend background→foreground. Variance shortens the streaks and shifts the mix toward hairs; Strength lengthens them; seeded by Randomize. Replaces the layer's pixels. Kooka uses photorust's model, tuned by eye against CS6 at 500 %. *(inferred family)* |
| Lens Flare | Optical flare model | Additive light: an inverse-square core in a soft glow, a halo ring, rays (6 / 8 / 4 for Zoom / 35mm / 105mm) or Movie Prime's horizontal blue streak, and tinted hexagonal ghosts and rings strung from the flare through the middle of the frame and out the far side. Every size is a fraction of the half-diagonal and the centre a fraction of the frame, so a proxy preview matches the full-size result. Kooka uses photorust's model, tuned by eye against CS6. Adobe's model is closed. *(inferred family)* |
| Lighting Effects | 3D lighting over a bump map | A rig of 1–16 Point/Infinite/Spot lights over a surface whose normal comes from a grayscale bump map (Height); Gloss/Metallic/Exposure/Ambience/Colorize combine diffuse/specular responses. A Spot is an ellipse with its hotspot toward the aimed end; a Point falls off to its radius; an Infinite light comes from an angle and elevation. Kooka uses photorust's shading over the rig, CPU-rendered and tuned by eye against CS6; see `lighting-effects.md`. *(inferred family; CS6 gallery behavior sourced)* |
| Scripted Patterns | Pattern-tiling scripts (Deco engine) | Scripts place the chosen pattern repeatedly across the layer/selection using geometric rules (brick stagger, cross weave, random placement, spiral, symmetry). Adobe Research documents that these are the Deco scripts. *(inferred internals)* |
| Flame / Tree | Procedural per-path / L-system-ish | **Post-CS6 only**; out of parity scope. |
| Picture Frame | Procedural border synthesis | **Post-CS6 only**; shipped by Kooka (#227). Motif frames lay a vine, flowers, and leaves along a rounded-rectangle track; ruled/moulded frames draw lines and a bevelled band from a corner distance field. Every size is a fraction of the layer's shorter side; the arrangement is a seed. Ported from photorust. |

**Pipeline.** Clouds/Difference Clouds/Fibers *replace* the layer (no input dependency beyond fg/bg colors), so they need no halo and cannot be masked by the source. Lens Flare and Lighting Effects are additive-light passes over the composed layer. Scripted Patterns is a fill operation that should share the fill/paint path, not the filter pipeline.

## Rust module mapping

- `pictura_filter::render` — per-filter submodules implementing `Filter` (or a `Fill` trait for Scripted Patterns).
- `pictura_filter::noise::FractalNoise` — seeded value/fractal noise; shared by Clouds and Difference Clouds; `Rng` seed stored in params.
- `pictura_filter::render::fibers` — vertical clump + hair streaks (photorust's model) with `variance` 0–64 (0 = even blend), `strength` 1–64, `seed`.
- `pictura_filter::render::lens_flare` — photorust's flare model; `LensType` enum, unit-coordinate centre, brightness 10–300 %; adds light onto the RGB planes, alpha untouched.
- `pictura_filter::render::lighting_effects` — `Light { kind: Point|Infinite|Spot, on, color, intensity, hotspot, center, angle, size, width, elevation }`, `Lighting { lights: Vec<Light> (1–16), gloss, metallic, exposure, ambience, colorize, texture: None|Red|Green|Blue, height }`; CPU today, GPU path via `pictura_gpu` later (`01-architecture/gpu-rendering-pipeline.md`).
- `pictura_filter::render::picture_frame` — photorust's frame model; `FrameOptions { frame, vine, margin, size, arrangement, flower, flower_colour, flower_size, leaf, leaf_colour, leaf_size, lines, thickness, angle, fade, invert }` plus the frame/flower/leaf name tables; procedural contours scan-converted source-over, alpha-moving, deterministic.
- `pictura_paint::fill::scripted` — `ScriptedPattern::{BrickFill, CrossWeave, RandomFill, Spiral, SymmetryFill}`; consumes a `PatternRef`.
- `pictura_filter::registry` — filter id + CS6 four-char event ids (`'Clou'` unconfirmed), `'DrfC'`, `'Fbrs'`, `'LnsF'`, `'LghE'` → implementation + `supported(mode, depth)`.

Crossing types: `Tile`, `Rgb`, `ChannelRef` (bump map), `PatternRef`, `Seed`, `BitDepth`, `FilterParams`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `RenderOptionsDialog` | `QDialog` | Fibers / Lens Flare forms in a `QStackedWidget` |
| `FibersOptionsPanel` | `QWidget` | Variance/Strength sliders + Randomize button |
| `LensFlareDialog` | `QDialog` | Whole-picture proxy preview with the flare and a click/drag crosshair; OK / Cancel / Preview beside it; Brightness field + slider and a Lens Type radio group below |
| `LightingEffectsDialog` | `QDialog` | CS6 workspace: options bar (Presets, add Spot/Point/Infinite, Reset, Preview, Cancel, OK), the picture with light controls, Properties panel, Lights panel; Save/Delete preset not built |
| `LightingCanvas` | `QWidget` | Proxy preview with draggable Point/Spot/Infinite light controls and Intensity rings |
| `ScriptedPatternsCombo` | `QComboBox` | Sits in the Fill dialog; five scripts; disabled until a pattern is chosen |
| `FillDialog` | `QDialog` | Extended with `Use: Pattern` + `Scripted Patterns` checkbox |
| `FilterMenuBuilder` | helper | Greys RGB/GPU-only Lighting Effects and unsupported depths |
| `GpuCapabilityProbe` | helper | Enables Lighting Effects only when a supported renderer exists |

Widgets over QML for the CS6 lighting workspace's gizmo interaction is a judgement call; a `QGraphicsView` scene (Widgets) is the lower-risk port of the CS6 canvas, matching `ARCH-003`.

## Data-model impact

- **Destructive replaces.** Clouds, Difference Clouds, and Fibers overwrite pixels on the active layer (Help is explicit). Undo therefore needs a pixel snapshot of that layer unless the layer is a Smart Object, where the result is derived and only parameters are stored. This is the notable exception to the "recompute from params" rule.
- **Lighting Effects scene** is a structured, saveable preset (all lights + properties); presets appear in the Style menu whenever the image is open and can be saved/deleted. The model needs a `LightingPreset` store separate from the document.
- **Scripted Patterns** parameters are `(pattern ref, script id)`. Fill operations already have a fill-dialog model (`05-layers/fill-layers.md`); this spec adds the script selector.
- **Bump map** for Lighting Effects references a document channel (R/G/B or alpha); serialize as a channel reference.
- **Smart Filters:** Fibers/Clouds/Difference Clouds/Lens Flare can be Smart Filters (their 16-bit listing supports this); Lighting Effects is a workspace and is not in the CS6 Help Smart-Filter exclusion list (Extract/Liquify/Pattern Maker/Vanishing Point), so it may be smart-capable — unconfirmed.
- **PSD:** no Render-specific additional-layer keys documented beyond the Smart Filter record (`LAY-021`).

## Edge cases

- **Destructive vs non-destructive.** On a normal layer, Clouds/Difference Clouds/Fibers replace pixels and need a full undo snapshot; on a Smart Object they are derived. The undo record shape differs by layer kind.
- **Foreground/background dependency.** Clouds/Difference Clouds/Fibers read the current fg/bg colors; the cache key must include them (and the Alt/Option variant).
- **16/32-bit.** Clouds, Difference Clouds, Fibers, Lens Flare run at 16-bit; only Clouds and Lens Flare run at 32-bit. Grey the rest.
- **Lighting Effects gating.** RGB only and requires a supported video card; must grey out with a tooltip when the GPU is unavailable (matching CS6's "supported video card" note).
- **Lighting Effects bump map depth.** Height 0–100; a missing/empty channel is a flat surface.
- **Max lights.** 16-light ceiling; the UI must stop adding lights and keep only one editable at a time.
- **Lens Flare center out of bounds.** Kooka clamps the centre to the canvas; whether CS6 allows an off-canvas centre is unverified.
- **Scripted Patterns without a pattern.** The script menu is disabled until a pattern preset exists (Help notes Pattern is dimmed until a library is loaded).
- **Scripted Patterns on huge/PSB documents.** Pattern tiling is area-proportional; stream and cap script iteration counts.
- **Difference Clouds iteration.** Cumulative marble patterning means repeated application differs; tests must render fresh each time.
- **Flame/Tree/Picture Frame.** Flame and Tree are not present in CS6; the menu builder must not surface them, and loading a CC 2014.2 PSD that used them should preserve the Smart Filter/effect data without claiming CS6 parity. Picture Frame *is* surfaced by Kooka (#227); it is applied to the active layer's rectangle, so a layer smaller than the document gets the border round the layer, not the canvas.
- **Undo/redo.** Store fg/bg colors, seeds, and (for replacing filters) a pixel snapshot; recompute the rest.

## Parity acceptance criteria

- Given a fresh layer, Clouds fills it with a fg/bg-colored cloud pattern and replaces any prior pixels; `Alt`/`Option` produces a starker pattern.
- Given Difference Clouds applied once, portions are inverted in a cloud pattern; applying it again changes the result (marble ribs/veins).
- Given Fibers Variance at minimum, streaks are long and run the full height; at maximum, fibers are short and varied; raising Strength lengthens the streaks; fibers run vertically; Randomize changes the pattern.
- Given Lens Flare Brightness 10, the flare is faint; at 300 it is bright; changing Lens Type changes the ghost geometry; moving the center moves the flare.
- Given Lighting Effects with a grayscale bump map at Height 100, the surface relief is maximal; at Height 0 the surface is flat.
- Given Lighting Effects on a non-RGB document or with no supported GPU, the filter is unavailable.
- Given Lighting Effects with 16 lights, adding a 17th is prevented; only one light edits at a time.
- Given `Edit > Fill` with `Use: Pattern` and Scripted Patterns then Brick Fill, the selection/layer is filled with a staggered-brick pattern; Cross Weave / Random Fill / Spiral / Symmetry Fill each produce their distinct geometry.
- Given the Scripted Patterns menu with no pattern preset loaded, the script control is disabled.
- Given a CS6 install, `Filter > Render` does **not** list Flame, Tree, or Picture Frame; Kooka additionally surfaces Picture Frame by decision (#227) but not Flame or Tree.
- Given a 32-bit document, Clouds and Lens Flare run and Fibers/Difference Clouds are unavailable.
- Given a normal layer and Clouds, `Edit > Undo` restores the exact prior pixels; on a Smart Object it removes the Smart Filter entry.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — Photoshop CS6 Help. Established: the Render submenu (Clouds, Difference Clouds, Fibers, Lens Flare, Lighting Effects) and each description; Clouds/Difference Clouds/Fibers "replace the image data"; Fibers Variance/Strength/Randomize; the CS6 Lighting Effects workspace (17 presets and their intensity/focus values, Point/Infinite/Spot types, Colorize/Exposure/Gloss/Metallic/Ambience/Texture, max 16 lights, Height 0–100, RGB-only and GPU requirement); Scripted Patterns at `Edit > Fill` with five included scripts; Halftone Pattern under Sketch; the 16/32-bit support lists.
- `https://applescriptlibrary.files.wordpress.com/2013/11/photoshop-cs6-applescript-reference.pdf` — Photoshop CS6 AppleScript Scripting Reference. Established Lens Flare ranges (brightness 10–300%, flare center unit value, lens types zoom/Prime 35/Prime 105/Movie Prime) and the four-char event IDs `'DrfC'`, `'Fbrs'`, `'LnsF'`, `'LghE'`; established that Fibers/Clouds/Lighting Effects are not exposed as scriptable filter-options classes (so no sourced Fibers ranges).
- `https://planetphotoshop.com/scripted-patterns.html` — KelbyOne/Planet Photoshop CS6 walkthrough. Established the five CS6 Scripted Pattern presets by name: Brick Fill, Cross Weave, Random Fill, Spiral, Symmetry Fill, and the `Edit > Fill > Use: Pattern > Scripted Patterns` workflow.
- `http://www.photoshopforphotographers.com/CC_2013/Help_guide/downloads/PhotoshopCCupdate-2014-2.pdf` — Adobe Photoshop CC 2014.2 update guide. Established that Flame, Picture Frame, and Tree were added in the **CC 2014.2** Render menu (not CS6) and summarized the Flame controls.
- `https://community.adobe.com/questions-712/cannot-find-filter-render-flame-in-photoshop-cc-2014-branched-1079848` — Adobe community. Established: CS6 has no `Filter > Render > Flame`; that entry was added later, after Photoshop became Photoshop CC.
- `https://web.archive.org/web/2014id_/https://helpx.adobe.com/photoshop/using/filter-effects-reference.html` — archived Adobe reference; corroborated the Render prose.

Not used in this pass:

- `https://adoberesearch.ctlprojects.com/wp-content/uploads/2018/05/ProgrammingDecoScriptsInPhotoshopCC.pdf` — referenced by search as the Deco script programming guide (five patterns); not fetched. Listed under Open questions as the place to confirm the script internals.

## Open questions

- **Fibers ranges/defaults.** Help names the sliders but states no limits or defaults; the AppleScript reference does not script Fibers. Kooka uses 1–64 for both, as photorust does. Resolve from a CS6 UI capture.
- **Fibers Strength direction.** Help says a high Strength gives "short, stringy fibers"; photorust's model, tuned by eye against CS6, lengthens the streaks as Strength rises. Resolve from CS6 captures at Strength 1 and 64.
- **Clouds noise model.** Adobe's fractal noise octave count, lacunarity, and the exact `Alt`/`Option` variant are undocumented. Resolve by fitting.
- **Difference Clouds blend.** Whether the Difference blend is applied in the working space and how it interacts with fg/bg interpolation is inferred. Resolve by fitting.
- **Lighting Effects Smart Filter support.** The Help exclusion list does not name Lighting Effects; whether CS6 lets it stack as a Smart Filter is unconfirmed. Resolve with a CS6 build test.
- **Lighting Effects lighting model.** Diffuse/specular weighting, the bump-map normal reconstruction, and the Gloss/Metallic mapping are closed. Resolve by fitting.
- **Lens Flare model.** Ghost positions/sizes and starburst geometry per lens type are closed; Kooka's are photorust's by-eye tuning. Resolve by fitting.
- **Scripted Patterns internals and parameters.** The Deco script semantics (and whether CS6 exposed any script parameters) are not in the Help; the Adobe Research "Programming Scripted Patterns" guide is the likely source. The task's suggestion that Picture Frame came "from Adobe Exchange" for CS6 could not be verified — no fetched source supports it, and the only sourced statement places Picture Frame in CC 2014.2. Resolve with a CS6.0 install or an Adobe Exchange archive.
- **`Clouds` four-char event id.** Not captured from the fetched event-code table; confirm the exact code if the Rust registry keys on it.
- **Scripted Patterns file/undo shape.** Whether the fill is recorded as a script invocation or as a raster fill for undo is unconfirmed.
