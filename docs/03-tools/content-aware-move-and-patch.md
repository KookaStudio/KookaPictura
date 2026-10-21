# Content-Aware Move and Patch

- **Spec ID:** `TOOL-032`
- **Status:** `Draft`
- **Parity tier:** `Core` — all three surfaces are in CS6 Standard. (Content-Aware *Scaling*, a different feature, is under `04-image-ops/`.)
- **New in CS6:** `Yes` — **Content-Aware Move** is new in CS6, and the **Patch** tool gained a **Content-Aware** mode in CS6. **Content-Aware Fill** (`Edit > Fill > Content-Aware`) existed since CS5.
- **Depends on:** `ARCH-008` document-model, `ARCH-009` undo-history, `ARCH-006` gpu-rendering-pipeline, `08-selection/selection-model.md`, `03-tools/healing-brushes.md`, `06-filters/*`

> Module and widget names are **design proposals**. The repository contains a
> working implementation; this file stays the behavioural contract for the
> feature, and `docs/dev/doc-code-parity-gaps.md` records what shipped and what
> did not. CS6 behavior is taken from the fetched CS6 Help PDF and
> CS6-for-Photographers chapter. The production algorithm is closed; the best
> public candidate is a PatchMatch-family randomized patch synthesis. Parity is
> **behavioral parity only, algorithm TBD**.

## CS6 behavior

### Content-Aware Move (`J` group, new in CS6)

Selects and moves part of a picture. The image is recomposed: the moved content is
placed at the destination and the hole left behind is filled with matching
elements sampled from the picture, without manual layering or complex selections.

- **Mode**
  - `Move` — place the selected object at a different location. Works best when
    the background remains similar (from the CS6 PDF: "most effectively when the
    background remains similar").
  - `Extend` — expand or contract objects such as hair, trees, or buildings.
    Architectural objects are best shot on a parallel plane; use photos shot on a
    parallel plane rather than at an angle.
- **Adaptation** — a five-level menu controlling how closely the result reflects
  existing image patterns: `Very Strict`, `Strict`, `Medium`, `Loose`,
  `Very Loose`. From Evening: `Strict` uses a rigid sampling from the surrounding
  area, `Loose` "jumbles things up more," and **the default is `Medium`**.
- **Sample All Layers** — look through all layers and create the result in the
  selected layer; with it checked the move can be applied to an empty new layer.
- Workflow: pick the tool, choose options, draw a selection (with this tool or any
  selection tool), then drag the selection to the destination.

### Content-Aware Patch (CS6 addition to the Patch tool)

The Patch tool repairs a selected area with pixels from another area or a pattern.
Its CS6 options bar gains a **Content-Aware** mode that synthesizes nearby content
for seamless blending.

- **Patch** (`Content-Aware`) — selected from the options bar.
- **Adaptation** — the same five-level menu as Content-Aware Move.
- **Sample All Layers** — create the result in another layer; select the target
  layer.
- The general Patch tool also has the pre-CS6 controls: `Source` / `Destination`
  mode, `Transparent` (extract texture with a transparent background), and
  `Use Pattern`. It works with **8- or 16-bit-per-channel** images and must be
  used on the Background layer or a pixel layer.

### Content-Aware Fill (`Edit > Fill`)

1. Select the part of the image to fill.
2. `Edit > Fill` (or `Delete` / `Backspace` on a Background layer).
3. `Use: Content-Aware` — seamlessly fills the selection with similar nearby
   content. The Help recommends a selection that **extends slightly into the area
   you want to replicate**; a quick lasso or marquee is often enough.
4. Fills are **randomly synthesized**, so repeating the fill produces different
   results; the Help tells the user to undo and re-apply to get a better sample.

Evening adds: expand the selection slightly (`Select > Modify > Expand` or Refine
Edge) before filling, and repeated fills can improve the result. The document can
be "trained" by using a layer mask to hide areas the fill should ignore, then
deleting the mask afterward.

**On "structure / color / skin":** CS6 exposes only the five-level `Adaptation`
menu (and Patch's `Sample All Layers`); it has **no** Structure, Color, or Skin
controls and **no** separate Content-Aware Fill workspace. Those explicit
controls belong to later CC releases. The CS6 adaptation levels *do* trade off
structure (edge/contour fidelity) against color/texture jumbling, which is the
same axis those later sliders name. See `## Open questions`.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Tools panel, Retouch group | Tool | `J` | Content-Aware Move is in the Spot Healing fly-out |
| Options bar (CA Move) | Bar | — | Mode (Move/Extend), Adaptation (5 levels), Sample All Layers |
| Options bar (Patch) | Bar | `J` | Patch group: `Content-Aware` vs. normal; Adaptation; Sample All Layers; Source/Destination; Transparent; Use Pattern |
| Options bar (Patch, normal) | Bar | — | Pattern picker + `Use Pattern` |
| `Edit > Fill` | Dialog | `Shift+F5` / `Delete`/`Backspace` on Background | Use menu: Content-Aware / Pattern / History / colors; Blending Mode; Opacity; Preserve Transparency |
| Selection overlay ("marching ants") | Canvas | — | Content-Aware Move drag preview; Patch selection border drag |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| CA Move Mode | enum | Move | Move / Extend | Extend needs sufficient opposite-side overlap to cover what it shortens |
| CA Move Adaptation | enum | Medium | Very Strict / Strict / Medium / Loose / Very Loose | Structure↔color trade-off |
| CA Move Sample All Layers | bool | Off | on / off | Result can go to an empty layer |
| Patch mode | enum | Normal (prior) | Normal / Content-Aware | CS6 adds Content-Aware |
| Patch Adaptation | enum | Medium | Very Strict … Very Loose | Same menu as CA Move |
| Patch Sample All Layers | bool | Off | on / off | Result to selected layer |
| Patch Source/Destination | enum | Source | Source / Destination | Normal Patch mode |
| Patch Transparent | bool | Off | on / off | For solid/gradient backgrounds with distinct textures |
| Patch Use Pattern | action | — | pattern picker | Normal Patch mode |
| Fill Use | enum | Foreground (dialog) | Foreground / Background / Color / Black / 50% Gray / White / Pattern / History / Content-Aware | `Content-Aware` is the relevant entry |
| Fill Blending Mode | enum | Normal | CS6 blend list | — |
| Fill Opacity | int % | 100 | 0–100 | — |
| Fill Preserve Transparency | bool | Off | on / off | — |
| Bit depth (Patch) | — | — | 8 or 16 bpc | Patch explicitly 8/16; content-aware fill via `Edit > Fill` follows the document |

## Algorithms & pipeline

### Behavioral model

All three surfaces solve the same class of problem: **content-aware fill** —
replace a masked region (brush footprint for Spot Healing's Content-Aware type, a
selection for Patch/Fill, a moved hole for CA Move) with content synthesized from
elsewhere in the *same* image, such that the result is plausible and seamless.

### Public candidate algorithm: PatchMatch-family patch synthesis

The strongest public account is that Adobe's Content-Aware Fill (introduced
CS5, reused by CS6) is based on **PatchMatch**, the randomized approximate
nearest-neighbor (NNF) patch-matching algorithm by Barnes et al. (SIGGRAPH 2009);
Barnes's own page states the technology was incorporated into Photoshop CS5 as
Content-Aware Fill, and independent write-ups describe the shipping feature as a
modified PatchMatch. The core is:

1. Divide the source and target into overlapping patches (e.g. 7×7).
2. Initialize a **nearest-neighbor field** mapping each target patch to a source
   patch; initialization is partly random.
3. Iterate **propagation** (borrow good matches from neighboring patches) and
   **random search** (probe exponentially shrinking neighborhoods), which
   converges quickly to good approximate matches.
4. **Synthesize** the missing region by aggregating matched patches (e.g.
   voting/averaging), then iterate to fill larger holes.

CS5/CS6 predate the 2011 multi-scale "PatchMatch + content-aware fill with
translation/scale" refinement; the older variant is roughly single-scale. The
exact patch size, weighting metric, sampling mask, and any Poisson/biharmonic
final blending Adobe uses are **closed** → **behavioral parity only, algorithm
TBD**.

### Adaptation levels

The Adaptation menu maps to how constrained the match search is. Evening's
description: `Strict` = rigid sampling from the immediate surroundings; `Loose` =
more jumbling. We model this as the size/looseness of the allowed source region
and the patch-match regularization weight:

| Level | Source-region scope | Match regularization | Effect |
|---|---|---|---|
| Very Strict | nearest surround only | high | most literal, least synthesis |
| Strict | small ring | high | literal |
| Medium (default) | moderate | medium | balanced |
| Loose | wide | low | more reshuffling |
| Very Loose | widest | lowest | most random/inventive |

*(inferred mapping; the exact Adobe parameterization is undocumented.)*

### Move vs. Extend vs. Patch vs. Fill

- **Content-Aware Move (Move):** two coupled solves — place the lifted object at
  the destination (optionally with feathered blending), and fill the vacated hole
  by patch synthesis. Adaptation governs both.
- **Content-Aware Move (Extend):** the selection is stretched and blended against
  its surroundings rather than hole-filled; when *contracting*, the opposite-side
  selection must be large enough to cover the object being shortened.
- **Content-Aware Patch:** synthesis restricted to a user-chosen source region
  (the dragged selection), which gives the user direct control over the candidate
  source pool; then the heal/blend reconcile.
- **Content-Aware Fill:** fully automatic source selection; no adaptation control
  in CS6, hence the Help's advice to re-roll the random synthesis.

### Blending

Patch-based synthesis alone can leave seams; production tools typically finish
with a gradient-domain reconcile. Adobe's Healing Brush uses a biharmonic solve
(see `03-tools/healing-brushes.md`); the Patch tool explicitly "uses the same
algorithm as the healing brush." Content-Aware Fill's exact finishing is
undocumented. We therefore propose an optional Poisson/biharmonic seam blend
(`pictura-retouch::solve`) and mark it *(inferred implementation choice)*.

## Rust module mapping

Proposals:

- `pictura-retouch::content_aware::ContentAwareMove` — `MoveConfig { mode: Move |
  Extend, adaptation, sample_all_layers, selection, delta }`.
- `pictura-retouch::content_aware::ContentAwarePatch` — `PatchConfig { adaptation,
  sample_all_layers, source_mode: Source | Destination, transparent, pattern }`.
- `pictura-retouch::content_aware::ContentAwareFill` — `FillConfig { selection,
  seed }` for `Edit > Fill`.
- `pictura-retouch::patchmatch::{Nnf, PatchMatch, PatchSize}` — randomized NNF
  init, propagation, random search; masked source/destination; seeded RNG.
- `pictura-retouch::synthesize::vote_synthesize` — patch aggregation into the hole.
- `pictura-retouch::solve::poisson` / `::biharmonic` — optional seam reconcile
  (shared with healing).
- `pictura-core::selection::Selection` — mask input for all three surfaces.
- `pictura-core::command::ContentAwareEdit` — one command per committed operation.

Crossing types: `Selection`, `PatchSize`, `Adaptation`, `Seed(u64)`, and tile
slices. The seed is stored in the command so redo reproduces the same fill; a
fresh user invoke draws a new seed, matching "undo and apply again gives a
different result."

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `ContentAwareMoveOptions` | `QWidget` (options bar) | Mode combo, Adaptation combo, Sample All Layers |
| `PatchOptions` | `QWidget` (options bar) | Normal/Content-Aware, Adaptation, Source/Destination, Transparent, Use Pattern, Sample All Layers |
| `FillDialog` | `QDialog` | `Edit > Fill` Use/Blending/Opacity/Preserve Transparency; exposes Content-Aware |
| `SelectionOverlayView` | `QQuickItem` | Marching-ants selection and live drag preview on the canvas |
| `ContentAwareProgress` | `QObject` | Progress/cancel for the multi-resolution solve; CS6 shows a progress bar for fills |

Widgets for the modal `FillDialog` and options bar; selection overlay on the QML
canvas. Long solves run on a worker thread with cancellation (`ARCH-004`), and the
result is committed through the command API on completion.

## Data-model impact

- **No PSD fields.** Content-aware operations are destructive pixel edits; the
  edited pixels are ordinary layer/channel tile data.
- **Undo:** one history state per committed operation (`ARCH-009`), recorded as a
  `ContentAwareEdit` command plus pre-edit tile backups. The random `Seed` is
  stored so redo is stable; undo does not need to reproduce it.
- **Result layer:** with `Sample All Layers`, the outcome can be written to a
  separate (possibly empty) layer rather than the sampled layer; this changes
  only which node receives the pixels, not the schema.
- **Selection** is a first-class document object (`08-selection/selection-model.md`);
  Patch/CA Move reuse it.
- **Content-Aware Fill is not a Smart Filter and is not re-editable** in CS6; it
  bakes into pixels (as with the other destructive fills).

## Edge cases

- **Patch is 8/16-bpc only.** Implement Patch to refuse 32-bpc rather than
  degrade.
- **Random synthesis / non-determinism.** Two identical fills differ; the UI must
  allow easy undo+re-roll. Determinism is required only for redo, via the stored
  seed.
- **Insufficient source material.** A fill with no plausible nearby content
  (e.g. a large hole in a flat region) may produce obvious artifacts; do not
  claim success — surface an `insufficient_source` signal.
- **Extend/contract overlap.** Contracting an object requires enough selection on
  the opposite side to cover what is shortened; validate and warn otherwise.
- **Selection at document edges / empty margins.** Content-aware fill is the
  recommended way to replace empty border areas; the source search must clamp to
  the document or fail cleanly.
- **Huge PSB documents.** Patch matching must be tile/window-based with bounded
  memory; never allocate an NNF for the whole canvas at maximum dimensions
  without a budget.
- **GPU unavailable.** Provide a CPU fallback; PatchMatch is parallel-friendly
  (`rayon`) but must not require a GPU.
- **32-bit / CMYK / Lab** via `Edit > Fill > Content-Aware` — behave per the
  document model; if unsupported, refuse cleanly.
- **Interrupt/cancel.** A cancelled solve must leave the document unchanged.

## Parity acceptance criteria

- Given a CS6-style photo, Content-Aware Move in Move mode relocates a selected
  object and fills the vacated hole with plausible matching content; the result
  has no hard seam at the old location.
- Given Extend mode on a parallel-plane building/cloud photo, the object is
  stretched and blends into its surroundings; contracting warns when the
  opposite-side selection is too small.
- Given the same operation at each Adaptation level with all else equal, `Very
  Strict` produces the most literal sampling and `Very Loose` the most
  synthesized/jumbled result, ordered consistently; `Medium` is the default.
- Given Content-Aware Patch, the fill is drawn only from the user-dragged source
  region (not the whole image), and `Sample All Layers` writes to the selected
  layer.
- Given `Edit > Fill > Content-Aware` on a selection that extends slightly into
  the source area, the fill is seamless; applying it again to the same selection
  after undo can yield a different result (randomness), while redo reproduces the
  recorded result exactly.
- Given a Patch operation on a 32-bpc document, the tool reports "unavailable"
  rather than converting.
- Given any committed content-aware operation, undo restores every touched pixel
  bit-exactly and exactly one history state is added.
- Given a cancelled long-running solve, the document is byte-identical to before.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  Adobe Photoshop CS6 Help reference (downloaded and text-extracted). Established:
  Content-Aware Patch and Content-Aware Move are grouped with the Spot Healing
  Brush; Content-Aware Patch options (Patch = Content-aware, Adaptation, Sample
  All Layers) and workflow; Content-Aware Move options (Mode = Move/Extend,
  Adaptation, Sample All Layers), that Move suits similar backgrounds and Extend
  suits hair/trees/buildings on a parallel plane; `Edit > Fill > Content-Aware`
  behavior, the "extend the selection slightly" advice, and randomized re-roll;
  Patch tool's 8/16-bpc limit, Source/Destination, Transparent, Use Pattern;
  `Shift+F5` Fill shortcut and Content-Aware default on `Delete` for Background
  layers. Primary source for CS6 behavior.
- `http://www.photoshopforphotographers.com/pscs6/downloads/patch-tool.pdf` —
  Martin Evening, *Adobe Photoshop CS6 for Photographers*, free chapter.
  Established: CS6 adds a Content-Aware mode to the Patch tool; the five
  Adaptation methods `Very Strict / Strict / Medium / Loose / Very Loose` (Figure
  2); **default Medium**; "Strict uses a rigid sampling from the surrounding area,
  while with the Loose method it tends to jumble things up more"; Content-Aware
  Fill has no adaptation control; expanding the selection and repeated fills can
  improve results; the layer-mask "training" trick; Patch uses the same algorithm
  as the Healing Brush. Secondary, CS6.
- `http://www.photoshopforphotographers.com/3101-1901/Help_guide/tp/contentaware_movetool.html`
  — same series. Established: Content-Aware Move works like Patch in Destination
  mode but moves/extends; offers "the same adaptation methods as provided for the
  patch tool in the content-aware fill mode (in CS6)"; Move and Extend workflows
  (leaf relocation; cloud extension using `Very Loose`). Secondary, CS6.
- `https://gfx.cs.princeton.edu/pubs/Barnes_2009_PAR/index.php` — Barnes et al.,
  "PatchMatch: A Randomized Correspondence Algorithm for Structural Image
  Editing" (SIGGRAPH 2009). Established: the randomized approximate nearest-
  neighbor patch-matching algorithm (random init, propagation, random search,
  voting synthesis) and its use for image completion. Community/academic source
  for the candidate algorithm.
- `https://gfx.cs.princeton.edu/pubs/_2011_PAF/index.php` — Barnes et al.,
  "PatchMatch: A Fast Randomized Matching Algorithm with Application to Image
  and Video" (2011). Established: the multi-scale/refined extension that postdates
  CS5/CS6; clarifies that the CS5-era feature is the earlier single-scale variant.
  Community/academic source.

Additional evidence backing the PatchMatch attribution (author's own statement)
appeared in search-result snippets from `http://www.connellybarnes.com/work` and
`https://stackoverflow.com/questions/2530449/how-does-content-aware-fill-work`;
these pages were **not** fetched in full and are cited as unverified context only.

Not parsed in this pass: `https://helpx.adobe.com/photoshop/using/content-aware-patch-move.html`
(helpx.adobe.com returns HTTP 403); `https://prodesigntools.com/photoshop-cs6-sneaks-content-aware-move-extend-patch.html`
(HTTP 403).

## Open questions

- **Exact shipping algorithm.** That CS6 Content-Aware Fill is PatchMatch-based is
  strongly evidenced by the author's own statement but not confirmed by an Adobe
  CS6 document; the production patch size, metric, and finishing blend are
  closed. Resolve with a comparison harness or accept behavioral parity.
- **Adaptation parameterization.** The precise effect of each of the five levels
  is described only qualitatively. Resolve by controlled CS6 reference renders.
- **Structure/Color/Skin scope.** CS6 has no Structure/Color/Skin controls; those
  belong to later CC releases. Whether Kooka Pictura should expose them as optional
  non-CS6 controls is a product/parity-policy decision. Resolve with
  `00-overview/feasibility-and-non-goals.md`.
- **Seam finishing.** Whether Content-Aware Fill in CS5/CS6 ends with a Poisson/
  biharmonic gradient-domain blend or pure patch voting is unknown. Resolve by
  experiment or document it as an implementation choice.
- **Default Adaptation.** Evening states the default is `Medium`; the CS6 Help PDF
  does not state a default. Confirm on a CS6 install.
- **Random-seed reproducibility.** Whether redo in CS6 reproduces a content-aware
  fill exactly is unverified; our design stores a seed to guarantee it.
