# Healing Brushes

- **Spec ID:** `TOOL-031`
- **Status:** `Draft`
- **Parity tier:** `Core` — Spot Healing Brush and Healing Brush are in Photoshop CS6 Standard; the video/animation-frame healing is `Extended-only`.
- **New in CS6:** `No` — both brushes predate CS6 (Healing Brush since Photoshop 7, Spot Healing Brush with its Content-Aware type since CS5). CS6 behavior is unchanged. The **Diffusion slider** sometimes associated with these tools was added later, in Photoshop CC 2015.1, and is **not** part of CS6; the underlying diffusion-like blending is, however, the heart of the tool.
- **Depends on:** `ARCH-008` document-model, `ARCH-009` undo-history, `ARCH-006` gpu-rendering-pipeline, `03-tools/clone-stamp-and-pattern-stamp.md`, `03-tools/content-aware-move-and-patch.md`, `07-color-painting/brush-engine.md`, `02-ui-ux/panels/clone-source-panel.md`

> Names below are **design proposals**. The repository contains a working
> implementation; this file stays the behavioural contract for the feature, and
> `docs/dev/doc-code-parity-gaps.md` records what shipped and what did not. The
> behavioral description is sourced from the fetched CS6 Help PDF and
> CS6-for-Photographers material; the algorithmic explanation is sourced from an
> Adobe researcher's paper on the Healing Brush. Adobe does not publish its
> production solver, so parity is **behavioral parity only, algorithm TBD** in
> all cases.

## CS6 behavior

Both healing tools paint with **sampled** pixels and then *reconcile* them with
the destination: the source's **texture** is kept while its **color, luminosity,
and shading** are adapted to the pixels surrounding the area being healed, so the
repair melds into the image instead of showing a cloned edge.

**Healing Brush** (`J`) — the user Alt-clicks / Option-clicks to set a sample
point, then paints over the defect. Options:

- **Mode** — a blending mode; `Replace` preserves noise, film grain, and texture
  at the edges of a soft brush stroke.
- **Source** — `Sampled` (pixels from the current image) or `Pattern` (a chosen
  pattern).
- **Aligned** — keeps a continuous sample point across strokes; deselected, the
  brush returns to the initial sample point each time.
- **Sample** — `Current Layer` / `Current And Below` / `All Layers`, with the
  `Ignore Adjustment Layers` icon when `All Layers` is selected.
- Uses the same **Clone Source panel** as Clone Stamp: up to five sample sources,
  scale/rotate, and source overlay.
- If there is strong contrast at the edge of the area to heal, the user makes a
  selection *larger* than the repair and precisely following the contrasting
  boundary; the selection prevents outside colors from bleeding in. The sampled
  pixels are melded with the existing pixels **each time the mouse is released**.
- Can be applied to video/animation frames in Extended.

**Spot Healing Brush** (`J`) — no sample point is required; the brush
automatically samples around the retouched area. A brush slightly larger than the
defect is recommended so it can be covered in one click. Options:

- **Mode** — as above; `Replace` preserves grain/noise.
- **Type** —
  - `Proximity Match` — uses pixels around the edge of the selection to find a
    patch.
  - `Create Texture` — uses pixels in the selection to synthesize a texture; if
    it fails, dragging through the area a second time may help.
  - `Content-Aware` — compares nearby content to fill the selection, maintaining
    shadows and object edges.
- **Sample All Layers** — sample all visible layers instead of the active layer.
- Click for a spot, or drag to smooth a larger area. For larger or more precise
  selections the CS6 Help directs the user to `Edit > Fill > Content-Aware`.

**Patch tool** shares the Healing Brush's blend mathematics but operates on
selection-defined areas rather than a brush (see
`03-tools/content-aware-move-and-patch.md`).

**On diffusion:** the CS6 healing tools expose no diffusion control. Later CC
releases (2015.1) added a `Diffusion` slider (1–7) to Spot Healing, Healing
Brush, and Patch controlling how quickly the pasted region adapts to its
surroundings. We implement diffusion as an internal parameter of the healing
solver (see below) while keeping the CS6 UI free of the slider.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Tools panel, Retouch group | Tool | `J` | Spot Healing Brush, Healing Brush, Patch, Red Eye share `J`; fly-out selects |
| Options bar (Spot Healing) | Bar | — | Brush size/preset, Mode, Type (Proximity Match / Create Texture / Content-Aware), Sample All Layers |
| Options bar (Healing) | Bar | — | Brush, Mode, Source (Sampled / Pattern) + pattern picker, Aligned, Sample (Current Layer / Current And Below / All Layers), Ignore Adjustment Layers |
| `Window > Clone Source` | Dock panel | — | Five sample sources, scale/rotate/flip, overlay (shared with Clone Stamp) |
| Selection (any tool) | Canvas | — | A pre-made selection constrains/insulates healing near strong edges |
| Brush Presets panel | Dock | `F5` | Tip shape and dynamics |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Brush size | int px | last used | 1 … document | Slightly larger than the defect is advised |
| Brush hardness | int % | preset | 0–100 | Healing is generally used hard-edged |
| Mode | enum | Normal | CS6 blend list | `Replace` preserves grain/texture |
| Source (Healing) | enum | Sampled | Sampled / Pattern | Pattern requires a pattern picker |
| Pattern | pattern ref | last used | loaded patterns | Used when Source = Pattern |
| Aligned | bool | On (recommended) | on / off | Continuous vs. reset sample point |
| Sample (Healing) | enum | Current Layer | Current Layer / Current And Below / All Layers | Interactive sample of the image |
| Ignore Adjustment Layers | bool | Off | on / off | With `All Layers` |
| Type (Spot Healing) | enum | Proximity Match | Proximity Match / Create Texture / Content-Aware | *(default unverified for CS6)* |
| Sample All Layers (Spot) | bool | Off | on / off | All visible layers vs. active |
| Diffusion | — | — | 1–7 (CC 2015.1) | **Not in CS6**; internal solver parameter here |

## Algorithms & pipeline

### Behavioral model

The healing problem: given a **destination** region `Ω` (the brush footprint or
selection), a **source** region `S` (sampled offset or pattern), and the pixels
`g` just outside/around `Ω`, produce pixels inside `Ω` whose *texture/gradients*
match `S` while their *values* (color, luminosity, shading) blend continuously
into the surrounding `g`.

### Adobe's account (Georgiev, Adobe Systems)

An Adobe researcher's paper describes the Healing Brush as solving a **fourth-
order PDE** (the **biharmonic** equation, `Δ²f = 0`) rather than the second-order
**Poisson** equation used by Pérez et al.'s seamless cloning (`Δf = div v`).
Consequences stated in the paper:

- Poisson/Dirichlet matching makes the result **continuous in value** at the
  boundary but not in its derivatives; healing also matches derivatives, which is
  visibly better for smooth transitions.
- Biharmonic functions are a superset of harmonic ones, giving more freedom to
  follow brightness/color variation without becoming "too stiff."
- Higher-order PDEs give still better reconstruction but converge too slowly, so
  Adobe limited the production tool to biharmonic.
- The practical algorithm:
  1. Form the difference `h_in = f − g` between source `f` and surround `g`.
  2. Reconstruct `h_in` in `Ω` by solving the PDE with difference boundary
     conditions, giving `h_0`.
  3. The healed image is `h = h_0 + g`.
- Numerically it uses an iterative **Laplace** solver (5-point kernel `[0 1 0; 1
  0 1; 0 1 0]/4`) as the first approximation, then iterates the **biharmonic**
  kernel (13-point, `/32`), which converges quickly near the boundary and slowly
  in the interior; since the interior is already acceptable from the harmonic
  pass, the process is stopped short of a true biharmonic interior.
- Boundary handling uses two masks — the brush mask and the selection mask. Where
  a kernel neighbor lies outside the selection, its value is not read; instead
  the central pixel's value is substituted. This acts like a **Neumann boundary
  condition** (zero normal derivative at the boundary) and prevents a strongly
  different outside color from "contaminating" the reconstruction. Mask values in
  `[0,1]` interpolate the solved result with the original pixel values.

This is the closest public description of the CS6 algorithm. The production
implementation (exact kernels, stopping criteria, multi-resolution scheme) is
closed, so Kooka Pictura targets **behavioral parity only, algorithm TBD**.

### Spot Healing source search

- **Proximity Match** — search the annulus around the brush footprint for the
  best-matching source patch (the paper's "searching automatically to find the
  best pixels to sample"), then run the same heal solve. *(The search metric is
  undocumented.)*
- **Create Texture** — synthesize a texture from the pixels inside the selection
  and heal with it. *(inferred)*
- **Content-Aware** — delegate to patch-based synthesis (PatchMatch-family; see
  `03-tools/content-aware-move-and-patch.md`) and then blend.

### Diffusion

CS6 exposes no diffusion slider. Internally, "diffusion" is the **extent of the
surrounding area the solver considers and how quickly the pasted region adopts
it**: low diffusion keeps more of the source's local texture (good for noisy/
detailed areas), high diffusion smooths toward the surround (good for smooth
areas). We model this as the radius/weight of the boundary band `g` and the
number of PDE iterations.

## Rust module mapping

Proposals:

- `pictura-retouch::heal::HealingBrush` — `HealConfig { source: HealSource
  (Sampled | Pattern), aligned, sample_mode, mode, diffusion, brush }`; drives the
  dab loop.
- `pictura-retouch::heal::SpotHealing` — `SpotConfig { kind: ProximityMatch |
  CreateTexture | ContentAware, sample_all_layers, mode, brush }`; performs the
  source search then calls the solver.
- `pictura-retouch::heal::HealRegion` — `Ω` mask + boundary band `g` extraction
  from a `SampleSurface` (shared with Clone Stamp).
- `pictura-retouch::solve::laplace` / `::biharmonic` — iterative 5-point /
  13-point kernels over a masked region; convergence and iteration-cap inputs.
  Uses `pictura-filters` numeric helpers; parallelized with `rayon`.
- `pictura-retouch::solve::poisson` — optional Pérez-style guided interpolation
  for Patch/content-aware seams (see TOOL-032).
- `pictura-retouch::blend::mask_interpolate` — final `result = mask*solved +
  (1-mask)*original`.
- `pictura-core::command::HealStroke` — emitted per stroke / per patch commit.

Crossing types: `Mask` (f32 coverage per pixel), `SampleSurface`, and borrowed
tile slices. The solver is pure over tiles; cancellation/progress is threaded per
`ARCH-004`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `HealingBrushOptions` | `QWidget` (options bar) | Mode, Source + pattern, Aligned, Sample combo, brush preset |
| `SpotHealingOptions` | `QWidget` (options bar) | Mode, Type combo, Sample All Layers, brush preset |
| `CloneSourcePanel` | `QWidget` dock | Shared sample-source slots and overlay (see TOOL-030) |
| `HealOverlayView` | `QQuickItem` | Source overlay while healing (shared with Clone Stamp) |
| `ProgressReporter` | `QObject` | Optional progress/cancel for large-area solves (no CS6 progress bar for brush dabs; kept for responsiveness) |

Widgets over QML for the options bar; the canvas overlay is a `QQuickItem` next
to the GPU compositor. `SampleSurface` reads go through the render crate; the UI
never mutates nodes directly.

## Data-model impact

- **No PSD fields.** Brush, sample source, diffusion, and Clone Source slot state
  are session-only.
- **Undo:** one history state per completed stroke (per `ARCH-009`), recorded as a
  `HealStroke` command carrying pre-edit tile backups. `Patch`/Spot operations
  that are committed on release likewise create one state.
- **Retouch on a separate layer:** Healing Brush and Spot Healing Brush permit
  non-destructive retouching on a separate layer when `Sample All Layers` is
  selected; the healed result is written to the active (empty) layer. No document
  schema change.
- **Pattern source** is a `PatternId` reference into
  `07-color-painting/pattern-presets.md`.
- **Selection interaction:** a pre-existing selection bounds the solve; the
  selection mask and the brush mask are both inputs to the boundary-condition
  handling above.

## Edge cases

- **Strong edge next to the repair** — the solver may blend with a contrasting
  outside color; this is why the Help recommends a selection that follows the
  contrasting boundary. The Neumann-like mask handling is what prevents gross
  contamination; keep it.
- **16-bpc** — Spot Healing Brush and Healing Brush are **excluded**; CS6 lists
  them among tools *not* available on 16-bpc images. Implement them to refuse
  cleanly on unsupported depths rather than silently down-converting.
- **32-bpc HDR** — also excluded (Healing tools are not in the CS6 HDR tool
  list).
- **CMYK / Lab / Grayscale / Multichannel** — solve per-channel in the document's
  color model; do not convert implicitly.
- **Cross-document sampling** — both images must share a color mode, except that
  a Grayscale source is allowed.
- **Pattern with insufficient content** — a pattern that cannot supply texture
  must fail visibly, not paint a flat patch.
- **Large areas / slow convergence** — biharmonic iteration is slow in the
  interior; cap iterations and stop short (as Adobe does) or use a multi-grid /
  FFT solver. Never block the UI thread.
- **GPU unavailable** — CPU fallback solver; the tool remains functional.
- **Undo mid-stroke** — strokes are atomic on release; a heal solve for a single
  committed stroke must restore bit-exactly.
- **Empty region / zero-area mask** — no-op, no panic.

## Parity acceptance criteria

- Given a blemish on a smooth gradient background, a Healing Brush stroke that
  samples clean texture produces a result that is seamless to the eye: the mean
  color error along the healed boundary is below a small tolerance and no hard
  seam (a step in luminance) appears at the brush edge.
- Given the same document and a hard-edged brush at 100% hardness, healing with a
  selection that tightly follows a high-contrast boundary does not bleed the
  outside color into the repaired area.
- Given Spot Healing Brush in Proximity Match, a click on a small blemish removes
  it with surrounding texture, without a visible rectangular patch seam.
- Given Spot Healing Brush in Create Texture, the fill is synthesized from inside
  the selection (no external sample), and re-dragging changes the result.
- Given Spot Healing Brush in Content-Aware, the fill preserves nearby shadows
  and object edges better than Proximity Match on a structured background.
- Given `Sample All Layers`, healing on an empty layer samples the visible
  composite and writes the result to that layer; toggling it off samples only the
  active layer.
- Given the same input, the solver's result is deterministic (fixed iteration
  order), so repeated runs match bit-for-bit; the *diffusion* parameter
  monotonically changes edge adaptation (higher = smoother blend) as verified on
  a reference image.
- Given a completed stroke, undo restores every touched pixel bit-exactly and
  exactly one history state is added.
- Given a 16-bpc document, the tools report "unavailable" rather than silently
  degrading to 8-bit math.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  Adobe Photoshop CS6 Help reference (downloaded and text-extracted). Established:
  Healing Brush and Spot Healing Brush descriptions; `J` shortcut; Mode/Source/
  Aligned/Sample options; `Replace` preserves grain/texture; Spot Healing Type =
  Proximity Match / Create Texture / Content-Aware; Sample All Layers; "sampled
  pixels melded with existing pixels each time you release the mouse"; selection
  to prevent bleeding; Clone Source panel shared with Clone Stamp; 16-bpc and
  32-bpc exclusion of the healing tools; Patch tool uses the same blend math.
  Primary source.
- `https://mirror.umd.edu/gimp/references/Photoshop_Healing_Brush_a_Tool_for_Seamless_Clonin.pdf`
  — Todor Georgiev (Adobe Systems), "Photoshop Healing Brush: a Tool for Seamless
  Cloning". Established: healing as a **fourth-order (biharmonic)** PDE solve;
  relationship to and differences from Pérez et al. Poisson seamless cloning;
  the `h = h_0 + g` algorithm; the 5-point Laplace and 13-point biharmonic
  iterative kernels; harmonic first pass then biharmonic refinement; two-mask
  boundary handling equivalent to a Neumann condition to avoid contamination;
  `[0,1]` mask interpolation with the original pixels; Adobe limited production
  to biharmonic order. Established the algorithmic explanation above.
- `http://www.photoshopforphotographers.com/3101-1901/Help_guide/tp/Healing_brush.html`
  — Martin Evening, *Adobe Photoshop CS6 for Photographers* support page.
  Established: texture-from-source blended with color/luminosity-of-surround
  framing; the tool reads within a feathered radius up to 10% outside the cursor
  perimeter; the recommendation to use a hard 100% edge; how edge contrast can
  make healing fail and the selection workaround; Spot Healing Proximity Match /
  Create Texture / Content-Aware descriptions; healing on a separate layer with
  Current & Below / All Layers. Secondary, CS6.
- `http://www.photoshopforphotographers.com/pscs6/downloads/patch-tool.pdf` —
  same author, free CS6 chapter. Established: the Patch tool uses the same
  algorithm as the Healing Brush with selection-defined areas; Transparent mode;
  Use Pattern option. Secondary, CS6.
- `http://www.photoshopforphotographers.com/pscc/downloads/Photoshop%20CC_2015_5_update.pdf`
  — same author, CC 2015.5 update. Established: the **Diffusion slider** was added
  to Spot Healing, Healing Brush, and Patch (Normal mode) in CC 2015.1, range
  1–7, low values suit noisy/detailed images and high values smooth areas; and
  improved edge-of-document healing behavior. Establishes that the slider is
  **not** a CS6 feature. Secondary.

Not parsed in this pass: `https://helpx.adobe.com/photoshop/using/healing-examples.html`
(helpx.adobe.com returns HTTP 403).

## Open questions

- **Exact production solver.** Georgiev describes the mathematical family and the
  iterative kernels but not CS6's production stopping criteria, pyramid scheme,
  or per-region parameters. Resolve by public analysis reference renders or
  accept behavioral parity. *Resolves with:* a comparison harness on a labelled
  corpus.
- **Proximity Match search metric.** The patch-search distance used by Spot
  Healing's Proximity Match is undocumented. Resolve by experiment or choose a
  principled SSD/normalized metric and validate visually.
- **Default Type for Spot Healing in CS6.** The Help PDF lists the three types but
  not the shipped default. Resolve from a CS6 install.
- **Diffusion without the slider.** Since CS6 has no slider, what internal
  diffusion CS6 used (and whether it is fixed or depends on brush hardness) is
  unknown. Resolve from reference renders.
- **Soft-brush interaction.** The Help/Evening sources recommend hard brushes and
  claim no need for a soft edge; how (or whether) CS6 weights the heal by a soft
  brush mask is unverified.
- **Post-CS6 parity scope.** Whether to expose the CC 2015.1 Diffusion slider as
  an optional control (documented as non-CS6) is a product decision; it should be
  off by default to preserve a CS6 options bar. *Resolves with:* the UI/UX
  tool-parity policy.
