# Align, Distribute, Auto-Align, and Auto-Blend Layers

- **Spec ID:** `LAY-030`
- **Status:** `Draft`
- **Parity tier:** `Core` (Auto-Blend Layers' 32-bit path is `Core`; 3D/video inputs are out of scope)
- **New in CS6:** `Changed` — the CS6 Help documents Align/Distribute, Auto-Align Layers (with `Auto`, `Perspective`, `Cylindrical`, `Spherical`, `Scene Collage`, `Reposition Only`, and `Lens Correction`) and Auto-Blend Layers (`Panorama`, `Stack Images`, `Seamless Tones And Colors`). Auto-Align and Auto-Blend themselves predate CS6 (community sources attribute them to CS4); the exact CS5→CS6 delta for the projection list is not confirmed in this pass.
- **Depends on:** `05-layers/layers-overview.md`, `05-layers/artboards.md`, `05-layers/layer-masks.md`, `05-layers/blend-modes.md`, `03-tools/move-and-transform.md`, `08-selection/selection-model.md`, `05-layers/merge-and-flatten.md`, `ARCH-008` document-model, `ARCH-009` undo-history, `06-filters/lens-correction.md`

> All module and type names below are **design proposals**. The repository
> contains a working implementation; this file stays the behavioural contract
> for the feature, and `docs/dev/doc-code-parity-gaps.md` records what shipped
> and what did not. Algorithm notes not backed by the fetched CS6 Help are
> marked *(inferred)* or *(behavioral parity only)*.

## CS6 behavior

### Align Layers

With the Move tool active, selecting multiple layers and choosing
`Layer > Align` translates each selected layer so a chosen edge or center lines
up. When a pixel selection exists, `Layer > Align Layers To Selection` aligns
each layer to the selection border instead of to the other layers. The six
commands are:

- **Top Edges** — align each layer's topmost content pixel to the topmost
  content pixel among all selected layers, or to the top edge of the selection.
- **Vertical Centers** — align each layer's vertical content center to the
  common vertical center, or to the vertical center of the selection.
- **Bottom Edges** — bottom content pixel to the bottommost content pixel, or
  the selection's bottom edge.
- **Left Edges** — left content pixel to the leftmost content pixel, or the
  selection's left edge.
- **Horizontal Centers** — horizontal content center to the common horizontal
  center, or the selection's horizontal center.
- **Right Edges** — right content pixel to the rightmost content pixel, or the
  selection's right edge.

The target for the "align to each other" case is derived from the selection's
*farthest* edge in that axis (e.g. Top Edges snaps to the topmost of the
selected layers; Bottom Edges to the bottommost). With a selection present,
every layer is aligned to the selection bounds. The Move tool options bar
exposes the same six buttons.

### Distribute Layers and Groups

`Layer > Distribute` (or the Move tool options bar) requires **three or more**
selected layers or groups. It keeps the extreme layers fixed and repositions
the middle ones so the chosen anchor lines are evenly spaced:

- **Top Edges**, **Vertical Centers**, **Bottom Edges** — distribute along the
  vertical axis using each layer's top / vertical center / bottom as the anchor.
- **Left Edges**, **Horizontal Centers**, **Right Edges** — distribute along the
  horizontal axis using each layer's left / horizontal center / right anchor.

"Spacing" and "centers" are two views of the same command set: edge anchors
space *gaps* between extents, center anchors space *centers*. CS6 exposes no
separate "distribute spacing" command (that is an Illustrator-style concept);
the six options above are the whole set.

### Auto-Align Layers

`Edit > Auto-Align Layers` aligns layers by matching overlapping content
(corners and edges). One layer is the **reference layer**: if a layer is locked,
it is the reference; otherwise Photoshop analyzes all layers and picks the one
at the center of the final composition *(inferred — the Help says "the one at
the center of the final composition")*. Other layers are transformed so matching
content overlays the reference. The Help cautions against selecting adjustment
layers, vector layers, or Smart Objects that carry no alignment information.

Projection options (CS6 Help):

| Option | Behavior |
|---|---|
| **Auto** | Analyzes the sources and applies whichever of Perspective or Cylindrical produces the better composite. |
| **Perspective** | Designates one source (middle image by default) as reference; others are repositioned, stretched, or skewed to match overlapping content. |
| **Cylindrical** | Maps images onto an unfolded cylinder to reduce the "bow-tie" distortion of Perspective; reference placed at center; best for wide panoramas. |
| **Spherical** | For wide fields of view (vertical and horizontal); spherically transforms non-reference images to match overlaps. |
| **Scene Collage** | Matches overlapping content without changing object shape (a circle stays a circle). |
| **Reposition Only** | Matches overlapping content without stretching or skewing (translation only); intended for scanned images with offset content. |
| **Lens Correction** | Corrects lens defects with sub-options **Vignette Removal** (lighten dark corners) and **Geometric Distortion** (barrel/pincushion/fisheye). Fisheye metadata is handled specially. |

`File > Scripts > Load Files into Stack` is the documented bulk way to get images
into separate layers first. A group-portrait recipe in the Help uses
`Edit > Auto-Align Layers` then layer masks to combine two near-identical shots.

### Auto-Blend Layers

`Edit > Auto-Blend Layers` stitches or combines layers by auto-generating layer
masks, and optionally matching tonal ranges. Two objectives:

- **Panorama** — blends overlapping layers into a panorama.
- **Stack Images** — blends the best details in each corresponding area
  (focus stacking / illumination stacking); works best when the layers are
  already aligned.

**Seamless Tones And Colors** is an optional checkbox that adjusts color and
tonality while blending. Constraints stated by the Help:

- Only **RGB or Grayscale** images.
- Does **not** work with Smart Objects, video layers, 3D layers, or background
  layers.

Auto-Blend writes a layer mask per processed layer to hide content that should
not contribute (over/under-exposed or out-of-focus areas).

### Path components (same menu family)

Align and distribute also exist for **path components inside a single path**
(Path Selection tool). CS6 moves these controls into `Path Alignment` and
`Path Arrangement` drop-down menus in the options bar (CS5 exposed them as
options-bar buttons). To align shapes on separate layers, the Help directs the
user to the Move tool.

### Artboards

Artboards are **not a CS6 feature** (added in CC 2015), so there is no
artboard-relative align/distribute behavior to specify for parity. CS6 aligns
and distributes only within the single document canvas; see
`05-layers/artboards.md`.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Move tool options bar | Tool options | `V` | Six align buttons + six distribute buttons; selection-dependent |
| `Layer > Align` | Menu | n/a | Aligns selected layers to each other |
| `Layer > Align Layers To Selection` | Menu | n/a | Enabled when a pixel selection exists |
| `Layer > Distribute` | Menu | n/a | Enabled with 3+ layers |
| `Edit > Auto-Align Layers` | Dialog | n/a | Projection radio list + Lens Correction sub-options |
| `Edit > Auto-Blend Layers` | Dialog | n/a | Objective radio list + Seamless Tones And Colors |
| `File > Scripts > Load Files into Stack` | Menu | n/a | Loads images as layers before auto-align |
| `Layer > Align Layers To Selection` / artboard context | Menu | n/a | CS6 artboard-relative behavior unverified |
| Path Selection options bar | Tool options | `A` | `Path Alignment` / `Path Arrangement` drop-downs |
| Layers panel | Dock | `F7` | Multi-layer selection is the precondition for all of the above |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Align command | enum | none (user-invoked) | Top Edges, Vertical Centers, Bottom Edges, Left Edges, Horizontal Centers, Right Edges | Six options |
| Align target | enum | layers | layers / selection | `Align Layers To Selection` switches target |
| Distribute command | enum | none | same six anchors | Requires 3+ layers |
| Auto-Align projection | enum | `Auto` *(inferred default)* | Auto, Perspective, Cylindrical, Spherical, Scene Collage, Reposition Only, Lens Correction | Auto picks Perspective or Cylindrical |
| Auto-Align Lens Correction | enum (multi) | off | Vignette Removal, Geometric Distortion | Sub-options of Lens Correction |
| Auto-Blend objective | enum | `Panorama` *(inferred default)* | Panorama, Stack Images | Stack Images expects prior alignment |
| Seamless Tones And Colors | bool | off *(inferred)* | on / off | Tonal/color match while blending |
| Reference layer | layer ref | auto | any locked layer; else auto-picked | Lock a layer to force reference |
| Auto-Blend permitted modes | enum | n/a | RGB, Grayscale | CMYK/Lab rejected |

## Algorithms & pipeline

### Align (exact, cheap)

1. For each selected layer, compute a content bounding box: the tight bounds of
   pixels with alpha > 0 (for a `Background`-like layer, the document rect).
   If a selection exists and the target is the selection, compute the selection
   bounds instead.
2. Compute the target scalar for the chosen axis/anchor:
   - to layers: `min` for Top/Left, `max` for Bottom/Right, or the mean of the
     layer centers for Vertical/Horizontal Centers;
   - to selection: the corresponding selection-border coordinate/center.
3. For each layer, derive an integer translation `Δ` on that axis and add it to
   the layer's transform. No resampling, no scaling — align is translation only.
4. Emit one undo record.

Rounding for "centers" (odd/even content dimensions) is a source of off-by-one
differences; parity needs a defined rounding rule *(behavioral parity only)*.

### Distribute (exact, cheap)

1. Compute each layer's anchor scalar (edge or center) along the chosen axis.
2. Sort the anchors; the first and last layers are pinned.
3. The free span is `last - first`. Place the `n-2` interior anchors at
   `first + i * span/(n-1)` for `i = 1..n-2`, then translate each interior layer
   by the delta between its old anchor and its assigned anchor.
4. Round to integer pixels; emit one undo record.

If two layers share an anchor value, the sort order is a tie-break decision;
define it before claiming parity *(inferred)*.

### Auto-Align (behavioral parity only, algorithm TBD)

The Help documents *what* the projections do, not *how* matching is computed.
The engine is closed. A credible reconstruction *(inferred)* is:

1. Detect features per layer (corners / blobs).
2. Match features across layers in overlap regions.
3. Estimate a pairwise transform for the chosen projection:
   - Reposition Only → translation;
   - Perspective / Scene Collage → homography (with a shape-preserving
     constraint for Scene Collage);
   - Cylindrical / Spherical → homography in a cylindrical/spherical warp space;
   - Auto → run Perspective and Cylindrical and keep the better composite by an
     internal quality metric.
4. Chain pairwise transforms to the reference layer and apply.

The projection mathematics (cylindrical, spherical, perspective warp) follow
standard panoramic projection formulas; feature detection uses the SIFT/SURF-
class local descriptors. Exact Adobe feature choice, matching thresholds, and
the "better composite" metric are unknown. **Do not guess** — spec parity as
observable alignment accuracy, not byte equality.

`Lens Correction` is the same family as `06-filters/lens-correction.md`
(barrel/pincushion/fisheye + vignette). Whether Auto-Align reuses that filter's
correction model or a separate one is unverified.

### Auto-Blend (behavioral parity only, algorithm TBD)

Observable pipeline:

1. For each selected layer, analyze the overlap content:
   - **Stack Images**: per-region focus/illumination quality; keep the sharpest
     or best-exposed contribution, hiding the rest with a generated mask.
   - **Panorama**: seam finding across overlaps.
2. Write a grayscale layer mask per layer; optionally adjust tones/colors when
   **Seamless Tones And Colors** is on.
3. Composite the masked layers normally.

A plausible reconstruction *(inferred from the panorama-stitching literature)*
is multi-band (Laplacian-pyramid) blending plus a global exposure/color
correction for Seamless Tones And Colors, but Adobe's exact focus metric,
mask generation, and tonal model are unpublished. Treat as `behavioral parity
only, algorithm TBD`; validate with image-difference acceptance criteria rather
than pixel identity.

### Path component align/distribute

Same math as layer align/distribute, applied to the bounds of selected path
components within one path rather than to layers.

## Rust module mapping

Proposals:

- `pictura_layers::align` — `AlignAxis { Vertical, Horizontal }`,
  `AlignAnchor { Top, VCenter, Bottom, Left, HCenter, Right }`, `align(layers,
  anchor, target)`; pure translation, returns `Vec<NodeId>` + `Vec<Translation>`.
- `pictura_layers::distribute` — `distribute(layers, anchor)`; keeps extremes,
  returns interior translations.
- `pictura_geometry::bounds` — `content_bounds(&Layer) -> RectI`,
  `selection_bounds(&Selection) -> Option<RectI>`.
- `pictura_layers::auto_align` — `Projection { Auto, Perspective, Cylindrical,
  Spherical, SceneCollage, RepositionOnly, LensCorrection(LensCorrectionOpts) }`,
  `auto_align(&mut Document, &[NodeId], Projection) -> Result<Vec<Transform2D>>`.
  Feature detection/matching live in `pictura_imaging::features` and
  `pictura_imaging::warp`.
- `pictura_layers::auto_blend` — `BlendObjective { Panorama, StackImages }`,
  `AutoBlendOpts { seamless_tones_colors: bool }`,
  `auto_blend(&mut Document, &[NodeId], AutoBlendOpts) -> Result<Vec<MaskId>>`.
- `pictura_layers::path_align` — same enums over `BezierPath` component bounds.

Crossing types: `NodeId(u64)`, `Transform2D`/`Affine2D`, `RectI`, `MaskId(u32)`.
Auto-Blend writes masks through the normal mask API; Auto-Align writes
transforms through the normal transform API, so both reuse compositing and undo.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `MoveOptionsBar` | `QWidget` | Hosts six align + six distribute `QToolButton`s; enable/disable from selection size and selection presence |
| `AlignActionGroup` | `QActionGroup` | Shared actions used by both the options bar and `Layer > Align` menus |
| `AutoAlignDialog` | `QDialog` | Projection radio list, Lens Correction checkboxes, OK/Cancel; runs off-thread with a progress indicator |
| `AutoBlendDialog` | `QDialog` | Objective radios, Seamless Tones And Colors checkbox |
| `StackFilesAction` | `QAction` | `File > Scripts > Load Files into Stack` entry point |
| `AutoAlignProgress` | `QProgressDialog` | Cancelable; alignment is a long task on a worker |

The action group is shared so menu and options-bar state never diverge. Long
algorithms execute on a worker (`ARCH-004`); the UI is blocked from editing the
affected layers until commit, then a single `dataChanged`/`layoutChanged` batch
is emitted.

## Data-model impact

- **Align / Distribute / Auto-Align** mutate each layer's `transform`
  (`Affine2D`) only. Auto-Align may set non-integer affine terms (rotation,
  scale, perspective), so `Transform2D` must support a full 3×3 homography or a
  documented approximation.
- **Auto-Blend** creates or replaces one raster **layer mask** per processed
  layer, and may store a tonal correction. It does not change layer pixels.
- **Path align/distribute** mutates path component control points.
- **Undo:** each user invocation is one history state (`ARCH-009`). Auto-Align
  and Auto-Blend are compound: undo restores every affected transform / removes
  every generated mask in one step. Undo record shape:
  `Command { label, affected: Vec<NodeId>, before: Vec<StateDiff>, after:
  Vec<StateDiff>, pixel_backups: Vec<TileRef> }` (masks may reference tiles).
- **Serialization:** transforms and masks are native PSD layer data, so results
  round-trip. Auto-Align's homography must be stored as the layer's transform
  (PSD stores a transform only for smart objects; a raster layer's Move-tool
  transform is baked into pixels on save, so Auto-Align must resample before
  save — verify against CS6 behavior).

## Edge cases

- **One layer selected** — Align is available only for `Align Layers To
  Selection`; Distribute needs 3+.
- **No selection** — `Align Layers To Selection` is disabled.
- **All-transparent / empty layer** — no content bounds; decide whether it is
  skipped or anchored to the document/selection rect.
- **Locked layers** — a locked layer becomes the Auto-Align reference; position
  locks must prevent subsequent manual nudge but not the auto-align result
  (behavior to verify).
- **Layer position locked** — Align/Distribute should fail or skip.
- **Adjustment / fill / vector / Smart Object layers** — excluded from
  Auto-Align; adjustment and fill layers have no pixel bounds and are skipped by
  content-based align.
- **Auto-Blend mode guard** — CMYK/Lab and Bitmap/Indexed are rejected; Smart
  Objects, video, 3D, and Background layers are rejected.
- **Background layer** — cannot be freely transformed; the user must convert it
  to a regular layer before auto-align/blend. The command does not convert it.
- **8/16/32-bit** — align/distribute are geometry-only and depth-independent;
  Auto-Blend's focus/exposure analysis should run at the document depth (32-bit
  float supported for RGB/Gray).
- **Very large / PSB documents** — auto-align feature matching is
  memory-heavy; tile/stream as in `ARCH-006`.
- **No GPU** — auto-align/blend must complete on the CPU with a progress
  indicator and cancellation.
- **Artboards** — target selection across artboards unverified.
- **Path align in a layer with multiple subpaths** — only selected components
  move; unselected components and the path's own transform are untouched.

## Parity acceptance criteria

- Given two layers whose content boxes differ, `Layer > Align > Left Edges`
  translates both so their left content pixels share the same `x` (integer).
- Given a selection and `Layer > Align Layers To Selection`, every selected
  layer's chosen edge/center equals the selection's corresponding
  edge/center.
- Given N≥3 layers and `Layer > Distribute > Horizontal Centers`, the extreme
  centers are unchanged and the interior centers are equally spaced within ±1 px.
- Given N<3 layers, Distribute is disabled (or a no-op); given N=1, Align to
  layers is disabled.
- Given two overlapping panoramas and `Auto-Align > Reposition Only`, no
  rotation/scale is introduced (transform is a pure translation).
- Given a locked reference layer, Auto-Align leaves that layer's transform
  unchanged and aligns the others to it.
- Given `Auto-Blend > Stack Images` with `Seamless Tones And Colors` on two
  aligned focus-bracketed RGB layers, each output layer gains a mask, and the
  composite's focus metric is better than any single input over the overlap
  region.
- Given a CMYK document, or a selection containing a Smart Object,
  `Auto-Blend Layers` is rejected without changing the document. A Smart Object
  outside the selection does not block it.
- Given 32-bit RGB layers, Auto-Blend runs and the result is 32-bit.
- Given any of these commands, exactly one history state is added and a single
  undo restores all affected transforms/masks.
- Given a path with selected and unselected components, path alignment moves
  only the selected components.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  official Photoshop CS6 Help. Established: Align command set and target
  semantics (page 183); Distribute requires 3+ layers; Auto-Blend Layers
  objective names, Seamless Tones And Colors, and the RGB/Grayscale + no-Smart-
  Object/video/3D/background restrictions (page 182); Auto-Align Layers
  reference-layer rule and the full projection list including Lens Correction's
  Vignette Removal and Geometric Distortion (page 183); PATH component align/
  distribute via `Path Alignment`/`Path Arrangement` drop-downs (page 439);
  layer-align shortcut/menu references.
- `https://www.psdvault.com/basics/auto-blend-layers-photoshop` — community
  tutorial (later Photoshop versions). Established: Auto-Blend generates layer
  masks; RGB/Grayscale-only and layer-type restrictions; common failure modes
  (blurry results without prior alignment, visible seams). Not CS6-specific.
- `https://designshack.net/articles/software/the-master-guide-to-the-photoshop-layers-panel`
  — CS6-era Layers-panel guide. Established that layer filtering and related
  CS6 panel behavior exist; used mainly by `LAY-032`.
- `https://www.richardharrington.com/blog/2010/08/26/panoramic-layout-options`
  and `http://exquisitelines.com/ExquisiteLines/photomerge-notes` — Photomerge
  layout descriptions (Perspective/Cylindrical/Spherical) that parallel the
  Auto-Align projections; pre-CS6 community sources.

Search-result snippets only (not fetched; treat as unverified):

- `https://www.manualzz.com/doc/o/mw0wn/adobe-photoshop-cs6-user-manual-aligning-layers`
  and `https://www.libble.eu/...` (page 188) — CS6 manual Auto-Align text
  matching the PDF.
- `https://layersmagazine.com/extended-depth-of-field-with-auto-blend-layers-in-photoshop-cs4.html`
  and Creative Bloq's Photoshop timeline — used only to place Auto-Blend/
  Auto-Align introduction at CS4, not CS6.

## Open questions

- **CS5→CS6 delta for Auto-Align projections.** Community sources place
  Auto-Align/Auto-Blend in CS4 and list a similar projection set. Was
  `Lens Correction` (and `Scene Collage`) new in CS6, and what was the CS5
  option list exactly? Resolve with a CS5 Help capture or CS5 screenshot.
- **Lens Correction in Auto-Align.** Does it reuse `06-filters/lens-correction.md`
  parameters, and are the Vignette/Geometric sub-options independent toggles?
  Resolve from a CS6 dialog screenshot or the CS6 manual.
- **Auto-Align matching algorithm.** Feature detector, matcher, transform
  model, and the "better composite" Auto metric are unpublished. Resolve only by
  choosing our own algorithm and defining parity as accuracy tolerance.
- **Auto-Blend mask/tonal model.** Focus metric, seam algorithm, and Seamless
  Tones And Colors math are unpublished. Resolve by defining test tolerances.
- **Center-anchor rounding.** Which integer rounding CS6 uses for Vertical/
  Horizontal Centers alignment and distribution is unverified.
- **Reference-layer selection without a lock.** The Help says "center of the
  final composition"; the exact rule is unclear.
- **Artboard-relative align/distribute.** Behavior when selected layers belong
  to different artboards is undocumented in the fetched Help.
- **Layer position-lock interaction.** Whether Auto-Align respects the position
  lock or ignores it is unverified.
- **Raster layer transform persistence.** How (or whether) CS6 stores a
  non-integer Auto-Align transform on a saved raster layer needs a CS6 PSD
  round-trip test.
