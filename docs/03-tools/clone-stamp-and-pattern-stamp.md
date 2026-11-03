# Clone Stamp and Pattern Stamp

- **Spec ID:** `TOOL-030`
- **Status:** `Draft`
- **Parity tier:** `Core` — both tools are in Photoshop CS6 Standard; the video/animation-frame cloning described below is `Extended-only`.
- **New in CS6:** `No` — both tools and the Clone Source panel predate CS6; CS6 carries them forward unchanged. The panel's transform/overlay controls are available to Clone Stamp and Healing Brush alike.
- **Depends on:** `ARCH-008` document-model, `ARCH-009` undo-history, `ARCH-006` gpu-rendering-pipeline, `07-color-painting/brush-engine.md`, `03-tools/brush-and-pencil.md`, `03-tools/healing-brushes.md`, `02-ui-ux/toolbox-and-options-bar.md`, `02-ui-ux/panels/clone-source-panel.md`, `07-color-painting/pattern-presets.md`

> All crate, module, and widget names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Facts taken from the fetched CS6 Help reference PDF
> and CS6-for-Photographers material are marked by source; anything inferred is
> marked *(inferred)*. The exact Adobe sampling and pattern-tiling code is closed.

## CS6 behavior

**Clone Stamp** (`S`) paints pixels sampled from one part of an image over
another part of the same image, over another open document with the **same color
mode**, or from one layer onto another. It is used to duplicate objects and to
remove defects. The user Alt-clicks (Windows) or Option-clicks (macOS) in any
open image window to set the **sampling point**, then paints over the target.

- **Aligned** — sampled pixels advance continuously with the brush, keeping a
  fixed source↔destination offset even across separate strokes. With Aligned off,
  every new stroke starts from the original sampling point again.
- **Sample** — `Current Layer` (default, active layer only), `Current And Below`
  (active layer plus visible layers below it), or `All Layers` (all visible
  layers). When `All Layers` is chosen, an **Ignore Adjustment Layers** toggle
  appears to its right.
- **Clone Source panel** (`Window > Clone Source`) — up to **five** saved sample
  sources; the panel stores them until the document is closed. Per source it
  exposes an overlay, a scale, a rotation, horizontal/vertical flips, and an x/y
  pixel offset.
- **Overlay** — a preview of the source. `Show Overlay`, `Auto Hide` (hide while
  painting), `Clipped` (clip overlay to brush size), `Opacity`, an overlay blend
  mode (`Normal`, `Darken`, `Lighten`, `Difference`), and `Invert`. Setting
  Opacity 50% + Invert + unclipped makes aligned areas render solid gray.
- **Scale/rotate/flip** — `W`/`H` percentages (proportions constrained by
  default), a rotation in degrees, and `Flip Horizontal` / `Flip Vertical`.
- **Offset** — explicit x/y pixel offset for the sampled source.
- Holding `Alt+Shift` (`Option+Shift`) temporarily switches to the *Move Source
  Overlay* tool to drag the overlay's position.
- Any brush tip may be used, with blending mode, opacity, and flow from the
  options bar. The tool **does not work on adjustment layers**.
- *(Photoshop Extended)* Clone Stamp can paint onto video/animation frames and
  sample across frames/documents; the Clone Source panel then gains frame-
  relationship controls. Out of core parity.

**Pattern Stamp** (`S`, same tool group) paints a chosen **pattern** rather than
sampled pixels. The user picks a brush, mode, and opacity, selects a pattern from
the Pattern pop-up panel in the options bar, and drags.

- **Aligned** — keeps the pattern's continuity with the original start point
  across strokes; deselected, the pattern restarts at each stroke.
- **Impressionist** — applies the pattern with an impressionistic effect: adds
  jitter to the pattern source, producing a more diffuse texture *(secondary
  source)*.

Both tools sample only from pixels; their result is a destructive pixel edit on
the active layer unless the user first creates a separate retouch layer and
samples `All Layers` / `Current And Below`.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Tools panel, Retouch group | Tool | `S` | Clone Stamp and Pattern Stamp share the `S` slot; the fly-out lists both |
| Options bar (Clone Stamp) | Bar | — | Brush preset, Mode, Opacity, Flow, **Airbrush**, **Aligned**, **Sample** (Current Layer / Current And Below / All Layers), Ignore Adjustment Layers |
| Options bar (Pattern Stamp) | Bar | — | Brush preset, Mode, Opacity, Flow, **Airbrush**, **Aligned**, Pattern picker, **Impressionist** |
| `Window > Clone Source` | Dock panel | — | Five source buttons, W/H/Rotate/Flip, Offset x/y, overlay options |
| Clone Source overlay | Canvas overlay | `Alt+Shift`/`Option+Shift` (hold) | Temporarily becomes Move Source Overlay |
| Pattern pop-up panel | Picker | — | Pattern preset picker; library load from its panel menu |
| Brush Presets / Brush panel | Dock | `F5` / `F5`+menu | Brush tip shape and dynamics for both tools |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Brush preset | preset ref | last used | any tip | Size, hardness, spacing, dynamics |
| Mode | enum | Normal | CS6 blend list | Standard paint blending |
| Opacity | int % | 100 | 0–100 | Per-stroke coverage |
| Flow | int % | 100 | 0–100 | Per-dab paint rate |
| Aligned | bool | On (recommended) | on / off | Fixed source↔dest offset vs. reset |
| Sample (Clone) | enum | Current Layer | Current Layer / Current And Below / All Layers | Working note says active-layer-only is the default |
| Ignore Adjustment Layers | bool | Off | on / off | Shown only with `All Layers` |
| Clone source slot | enum | 1 | 1–5 | Persists until document close |
| Source W / H | double % | 100 | > 0 | Proportions constrained by default |
| Source rotation | double degrees | 0 | any | Degrees; scrub or type |
| Flip H / Flip V | bool | Off | on / off | Mirrors the source transform |
| Source offset X / Y | int px | 0 | any (signed) | Source placement in target |
| Show Overlay | bool | Off | on / off | Source preview on canvas |
| Auto Hide | bool | Off | on / off | Hide overlay while painting |
| Clipped | bool | Off | on / off | Clip overlay to brush size |
| Overlay Opacity | int % | 100 | 0–100 | Overlay-only |
| Overlay mode | enum | Normal | Normal / Darken / Lighten / Difference | Overlay-only |
| Invert | bool | Off | on / off | Invert overlay colors |
| Pattern (Pattern Stamp) | pattern ref | last used | loaded patterns | Pattern preset |
| Impressionist | bool | Off | on / off | Adds jitter / diffuse effect |

## Algorithms & pipeline

### Clone Stamp

Sampling model:

1. Resolve a **sample source surface** from the `Sample` mode: the active layer,
   the active layer plus visible layers below composited, or all visible layers
   composited (optionally excluding adjustment layers).
2. The Alt-click establishes an anchor `(sx, sy)` in source space; each dab has a
   destination center `(dx, dy)`.
3. **Aligned** maintains a constant offset `Δ = anchor - first_dest`; the source
   coordinate for a dab is `src = dest - Δ`. Non-aligned resets `src` to the
   anchor for every stroke.
4. Apply the **source transform** (scale, rotate, flip, additional offset) to map
   the brush footprint in destination space into source space.
5. For each pixel in the brush mask, bilinearly sample the source surface, then
   composite into the destination using the brush mask, opacity, flow, and blend
   mode. Standard alpha composition; the mask is the brush tip.

The source→destination mapping is an affine transform; the transform, offset, and
per-slot anchors are *tool session state*, not document state.

### Pattern Stamp

1. A **pattern** is a repeating tile (PSD pattern resource / preset).
2. **Aligned on**: the pattern is anchored to the document at the first stroke's
   origin, so strokes tile continuously. **Aligned off**: the pattern origin
   resets to each stroke's start.
3. Each dab samples the pattern at `((x - origin_x) mod w, (y - origin_y) mod h)`
   and composites through the brush mask and paint parameters.
4. **Impressionist** perturbs the pattern lookup (jitter of origin/scale/rotation
   or per-dab offset) so the tiled result reads as a diffuse, painterly texture.
   Adobe does not document the jitter distribution, so exact parity is
   **behavioral parity only, algorithm TBD**. *(inferred)*

Both tools feed the same brush-dab pipeline as the paintbrush; only the color
source differs (sampled surface vs. pattern vs. foreground color).

## Rust module mapping

Proposals (tool logic lives in a retouch/tool crate, not in `pictura-core`):

- `pictura-retouch::clone::CloneStamp` — `CloneStampConfig { aligned, sample_mode,
  brush, mode, opacity, flow }`, `CloneAnchor`, `CloneTransform`.
- `pictura-retouch::clone::SampleSurface` — resolves `SampleMode` into a readable
  tile view over one or more layers (a `pictura-render` composite handle).
- `pictura-retouch::clone::CloneSourceSlot` — `{ anchor, transform, overlay }`;
  a 5-element session array (`CloneSourceSet`).
- `pictura-retouch::pattern::PatternStamp` — `PatternStampConfig { aligned,
  impressionist, pattern: PatternId, brush, ... }`.
- `pictura-core::pattern::Pattern` — pattern tile buffer + id + name.
- `pictura-render::sample::BilinearSampler` — shared point sampler with affine
  source transform; used by Clone Stamp and Healing Brush.
- `pictura-core::command::PaintStroke` — the command emitted on stroke release
  (one history state).

Crossing types: `PatternId(u32)`, `CloneSourceId(u8)`, `Affine2D`, and borrowed
tile slices. The tool never touches a `Node` directly; it emits a `Command`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `CloneStampOptions` | `QWidget` (options bar) | Mode/opacity/flow, Aligned toggle, Sample combo + Ignore Adjustment Layers, brush preset |
| `PatternStampOptions` | `QWidget` (options bar) | As above plus Pattern picker and Impressionist toggle |
| `CloneSourcePanel` | `QWidget` dock (`QDockWidget`) | Five source buttons, transform fields (W/H/rotate/flip), offset X/Y, overlay controls |
| `CloneSourceModel` | `QObject` list model | Five `CloneSourceSlot` values; notified via cxx-qt to Rust |
| `PatternPopup` | `QWidget` pop-up | Pattern thumbnails and library menu |
| `OverlayView` | `QQuickItem` on the canvas | Draws the transformed source overlay (blend mode, opacity, clip, invert) |

Rationale: panel and options bar are classic QWidget docks with form controls
(`QDoubleSpinBox`, `QComboBox`, `QToolButton`); the overlay is canvas-drawn next
to the GPU compositor, so it belongs in the QML/`QQuickItem` scene, not in a
widget. All model writes go through the Rust command API (`ARCH-005`/`ARCH-009`).

## Data-model impact

- **No PSD fields.** Clone sources, anchors, overlay settings, and pattern
  selection are tool session state; CS6 keeps clone sources only until the
  document is closed. Nothing is serialized.
- **Undo:** one history state per completed stroke (per `ARCH-009`), recorded as a
  `PaintStroke` command with pre-edit tile backups.
- **Retouch on a separate layer:** the user targets an empty pixel layer and sets
  `Sample = All Layers` or `Current And Below`; nothing in the document model
  changes, only the active-layer designation at stroke time.
- **Pattern** belongs to the pattern-preset subsystem
  (`07-color-painting/pattern-presets.md`); the Pattern Stamp stores only a
  `PatternId` reference plus its own aligned/impressionist flags.
- **Texture sharing:** the Brush panel's *Copy Texture to Other Tools* can push a
  texture pattern/scale to Clone Stamp and Pattern Stamp among others; this is a
  brush-preset concern, not a document field.

## Edge cases

- **Cross-document cloning** requires the same color mode (a Grayscale source is
  accepted against another mode for healing tools, not for cloning). Mixed modes
  must be rejected or converted explicitly, never silently.
- **Adjustment layers** are not valid Clone Stamp targets; the tool is a no-op on
  them and should report it.
- **Sample All Layers with hidden layers** — hidden layers are excluded from the
  composite.
- **Empty/transparent target layer** with `Sample = Current Layer` yields nothing
  to clone; the user must choose `All Layers`/`Current And Below`.
- **1-pixel and empty documents** — a 1-px brush must not divide by zero in the
  dab spacing math.
- **8/16/32-bit** — the Clone Stamp is listed among the tools usable on 32-bpc HDR
  images and is available at 16 bpc; keep the buffer variant through the stroke.
- **CMYK/Lab/Grayscale** — clone in the document's channel model; do not convert
  implicitly.
- **Huge PSB documents** — sampling must be tile-local; never materialize a whole-
  canvas source copy.
- **GPU unavailable** — fall back to a CPU compositor/sampler; the tool must
  remain functional, only slower.
- **Aligned offset across undo** — a stroke is atomic; undo restores pre-stroke
  pixels and the offset state must remain coherent.
- **Import/preset loss** — an undefined pattern reference (e.g. a preset pointing
  at a missing pattern) must surface an error rather than paint nothing silently.

## Parity acceptance criteria

- Given a document and a Clone Stamp sample point, an Aligned stroke dragged with
  the same mouse path reproduces the source at a constant offset, and a second
  stroke continues that offset rather than restarting.
- Given Aligned off, every stroke restarts from the initial sample point.
- Given `Sample = All Layers`, painting over a region reproduces the visible
  composite of all layers above/at/below at that point (within 8-bit rounding),
  and hidden layers contribute nothing.
- Given `Sample = Current And Below`, layers above the active layer do not
  contribute.
- Given `All Layers` + Ignore Adjustment Layers, adjustment-layer output is
  excluded from the sampled surface.
- Given a source transform of scale 200%, rotate 90°, or Flip Horizontal, the
  painted pixels match the correspondingly transformed source within bilinear-
  sampling tolerance.
- Given overlay Opacity 50% + Invert + unclipped over an identical region, the
  overlay renders as solid gray.
- Given Pattern Stamp Aligned on, two strokes at different origins continue one
  continuous pattern tiling; with Aligned off, each stroke restarts the tile.
- Given Impressionist on, the result differs from the non-impressionist result
  for the same pattern and stroke (diffuse/jittered texture); exact pixels are
  out of scope (behavioral parity only).
- Given a completed stroke, undo restores every touched pixel bit-exactly at the
  document bit depth, and exactly one history state is added.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  Adobe Photoshop CS6 Help reference (downloaded and text-extracted). Established:
  Clone Stamp / Pattern Stamp tool descriptions and `S` shortcut; Alt-click
  sampling; Aligned semantics; Sample menu options and Ignore Adjustment Layers;
  restriction to same color mode and non-adjustment layers; Clone Source panel
  five sources saved until document close; W/H/rotate/flip/offset; overlay
  options (Show/Auto Hide/Clipped/Opacity/mode/Invert, 50%+Invert gray trick);
  `Alt+Shift` Move Source Overlay; Pattern Stamp Aligned and Impressionist;
  layout and shortcut tables; the "sample from all visible layers" and "retouch
  on a separate layer" notes; 32-bpc tool list including Clone/Pattern Stamp;
  Extended video/animation frame cloning. Primary source.
- `http://www.photoshopforphotographers.com/3101-1901/Help_guide/tp/Clone_stamp_tool.html`
  — Martin Evening, *Adobe Photoshop CS6 for Photographers* support page.
  Established: aligned recommendation, non-aligned reset behavior, Current &
  Below vs. Use All Layers, sampling from another image/layer. Secondary, CS6.
- `http://www.photoshopforphotographers.com/3101-1901/Help_guide/tp/Pattern_stamp_tool.html`
  — same series. Established: Pattern Stamp paints a selected pattern; the
  Impressionist option "adds some jitter to the pattern source" for a more
  diffuse texture. Secondary, CS6.
- `http://www.photoshopforphotographers.com/3101-1901/Help_guide/tp/Clone_Source_palette.html`
  — same series. Established: Clone Source panel previews alignment; multi-source
  value is mainly for video/registration work; rotate workflow. Secondary, CS6.

Not parsed in this pass: `https://helpx.adobe.com/photoshop/using/tool-techniques/clone-stamp-tool.html`
(helpx.adobe.com returns HTTP 403).

## Open questions

- **Default `Aligned` state and default `Sample` mode in CS6.** The Help PDF
  describes both states but does not pin the shipped defaults; the "sample from
  all visible layers" note says the default is active-layer-only, while Evening
  recommends Aligned on. Resolve from a fresh CS6 install or a CS6 screenshot.
- **Exact impressionist jitter model.** Adobe documents only that jitter is
  added. Resolve by analyzing a CS6 Pattern Stamp stroke or accept
  behavioral parity.
- **Overlay blend math.** The four overlay modes presumably reuse the standard
  blend formulas, but this is not confirmed for the overlay. Resolve against the
  blend-modes spec and a CS6 reference render.
- **Sampling interpolation.** Whether CS6 uses bilinear, bicubic, or nearest
  neighborhood when scaling/rotating the sample source is undocumented. Resolve
  with controlled scaled-source tests.
- **Cross-document sampling rules.** The Help PDF says "same color mode"; whether
  Grayscale↔RGB is allowed for the Healing Brush only (as stated) or also for
  Clone Stamp is not confirmed for CS6. Resolve with a CS6 experiment.
- **Source-slot persistence granularity.** "Until the document is closed" is
  stated; whether closing and reopening preserves slots per workspace is unclear.
