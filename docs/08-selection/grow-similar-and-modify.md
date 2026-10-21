# Grow, Similar & Modify

- **Spec ID:** `SEL-010`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `Changed` — the Modify commands themselves are unchanged from CS5, but CS6 added **decimal Feather values** for the Marquee and Lasso tools and the mask panels, matching a precision the Feather dialog already had; the Make Selection-from-path dialog also recalls its Feather radius (see `paths-and-vector-selection.md`).
- **Depends on:** `08-selection/selection-model.md`, `08-selection/selection-tools-overview.md`, `03-tools/quick-selection-and-magic-wand.md` (Magic Wand `Tolerance`), `08-selection/refine-edge.md`, `ARCH-002` document-model, `ARCH-007` undo-history.

> All crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not.

## CS6 behavior

The **Select** menu bundles three groups of selection-shaping operations. They act on the current selection and produce a new selection mask; none of them touch pixel content except Refine Edge with `Decontaminate Colors`.

**Grow / Similar** (`Select > Grow`, `Select > Similar`) expand the current selection to include additional pixels based on color similarity. Both read the **`Tolerance` value from the currently configured Magic Wand options** — not from a dialog of their own:

- `Grow` adds **adjacent** pixels that fall within the Magic Wand tolerance range.
- `Similar` adds pixels **throughout the whole image**, not only those adjacent to the selection.
- Issuing either command repeatedly grows the selection in increments.
- Neither command is available on **Bitmap-mode** images or **32-bits-per-channel** images.

**Select > Modify** contains the geometric edge edits:

- `Border` — replaces the selection with a band of pixels of a chosen width, **centred on the original selection border** (half inside, half outside). A width of 20 px yields a band extending 10 px inward and 10 px outward. The result is described as a soft-edged selection. Useful for selecting a halo/band around an area rather than the area itself.
- `Expand` — enlarges the selection border by a specified number of pixels. Any part of the selection border running along the **canvas edge is unaffected**.
- `Contract` — shrinks the selection border by a specified number of pixels.
- `Feather` — blurs the transition between selected and unselected areas by building a transition boundary of the given radius. The effect becomes visible only after the selection is moved, cut, copied, or filled.
- `Smooth` — cleans up stray pixels in a color-based selection by a majority vote over a neighbourhood: for each pixel, the surrounding pixels within `Sample Radius` are examined; if **more than half** of them are selected the pixel stays selected and unselected neighbours are added, otherwise the pixel is removed. The overall effect reduces patchiness and smooths sharp corners and jagged lines.

**Anti-aliasing** is a related edge softener but is not under Modify: it is a per-tool option (Lasso, Polygonal Lasso, Magnetic Lasso, Elliptical Marquee, Magic Wand) chosen **before** a selection is made; it cannot be added to an existing selection.

**Refine Edge** (`Select > Refine Edge`, or `Refine Edge…` in a selection tool's options bar) improves complex edges such as hair and fur and can also refine a layer mask. It is documented in the CS6 Help as the recommended replacement for the old Extract plug-in because it produces an editable mask instead of permanently erasing pixels. Its controls are:

- `View Mode` — display mode for previewing the selection (including `Show Original` and `Show Radius`); tool-tip descriptions are shown on hover.
- `Refine Radius` / `Erase Refinements` brushes — paint to add or remove the refinement zone; `Shift+E` toggles between them, bracket keys change brush size.
- `Smart Radius` — automatically varies the refinement radius between hard and soft edge regions; deselect it to control `Radius` manually.
- `Radius` — size of the border zone in which refinement occurs (small for sharp edges, large for soft edges).
- `Smooth` — reduces irregular "hills and valleys" in the border.
- `Feather` — blurs the selection/surround transition.
- `Contrast` — makes soft-edged transitions sharper when increased.
- `Shift Edge` — moves soft-edged borders inward (negative) or outward (positive); inward shifting removes background colour from edges.
- `Decontaminate Colors` — replaces colour fringes with the colour of nearby fully selected pixels, proportionally to edge softness. Because it changes pixel colour it **requires output to a new layer or document**; `Amount` controls the strength.
- `Output To` — selection, mask on layer, or new layer/document.

A dedicated treatment lives in `08-selection/refine-edge.md`; this spec only covers its role in the Modify workflow.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Select > Grow` | Menu | — | Uses Magic Wand `Tolerance`; repeatable |
| `Select > Similar` | Menu | — | Uses Magic Wand `Tolerance`; whole image |
| `Select > Modify > Border` | Menu | — | Width dialog (1–200 px) |
| `Select > Modify > Expand` | Menu | — | Expand By dialog (1–100 px) |
| `Select > Modify > Contract` | Menu | — | Contract By dialog (1–100 px) |
| `Select > Modify > Feather` | Menu | `Shift+F6` | Feather Radius dialog |
| `Select > Modify > Smooth` | Menu | — | Sample Radius dialog (1–100 px) |
| `Select > Refine Edge…` | Dialog | `Ctrl+Alt+R` | Selection-edge refinement; also a button in selection options bar |
| Selection-tool options bar | Button | — | `Refine Edge…` |
| Selection-tool options bar | Checkbox | — | `Anti-aliased` (Lasso/Marquee/Wand; set before selecting) |
| Magic Wand options bar | Numeric | — | `Tolerance` 0–255 feeds Grow/Similar |
| Toolbox | Button | `Q` | Quick Mask (related mask editing, see `quick-mask.md`) |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Magic Wand Tolerance | int | 32 (community-reported) | 0–255 | Read by Grow/Similar; PDF states range, not default |
| Grow adjacency | behaviour | adjacent | adjacent only | `Select > Grow` |
| Similar adjacency | behaviour | whole image | whole image | `Select > Similar` |
| Expand By | int (px) | not stated | 1–100 | Border along canvas edge unaffected |
| Contract By | int (px) | not stated | 1–100 | |
| Border width | int (px) | not stated | 1–200 | Centred on original border; soft-edged |
| Feather Radius (dialog) | float (px) | not stated | PDF gives 0–250 for tool option; dialog accepts decimals in CS6 | Exact dialog min/max not stated — see Open questions |
| Feather (tool option) | int (px) | 0 | 0–250 | Marquee/Lasso tools; CS6 also accepts decimals |
| Smooth Sample Radius | int (px) | not stated | 1–100 | Majority vote over neighbourhood |
| Anti-aliased | bool | not stated | on / off | Must be set before selection |
| Refine Edge Radius | float (px) | not stated | ≥ 0 | Enabled/disabled by Smart Radius |
| Refine Edge Smooth | int | 0 | not stated | |
| Refine Edge Feather | float (px) | 0 | not stated | |
| Refine Edge Contrast | int (percent) | 0 | not stated | |
| Refine Edge Shift Edge | int (percent) | 0 | negative inward / positive outward | |
| Refine Edge Decontaminate Colors | bool | off | on / off | Requires new layer/document |
| Refine Edge Amount | int (percent) | not stated | 0–100 nominal | Decontamination strength |

## Algorithms & pipeline

### Grow / Similar (behavioral parity only, algorithm TBD)

The Help text defines the observable rule: grow to adjacent pixels whose colour falls within the Magic Wand tolerance; similar to pixels anywhere in the image within tolerance. The underlying colour-distance metric (per-channel box, Euclidean RGB, or luminance-weighted) and the treatment of anti-aliased edge pixels are **not published**. Proposed implementation: seed a flood fill (Grow) or a global threshold pass (Similar) against the Magic Wand reference colour sampled at selection time, using the same distance function the Magic Wand uses so the two commands agree. Mark inferred.

### Expand / Contract

Straightforward morphology on the binary (or coverage-thresholded) selection mask: `Expand` is a dilation by a radius-`n` structuring element, `Contract` an erosion. The Help does **not** state the structuring-element shape (square vs. Euclidean disk) or whether the operation is performed on the soft coverage mask or its 50 % threshold. Proposed implementation: threshold-independent distance-transform approach (`distance = +expand / −contract`) so soft edges are preserved rather than binarised. Mark inferred.

### Border

A band centred on the original contour. Proposed: `band = dilate(mask, w/2) XOR erode(mask, w/2)`, then feather proportionally so the band reads as soft-edged, matching the Help's "soft-edged selection" wording. Exact Adobe softness is closed. Mark inferred.

### Feather

Feathering is a blur of the selection coverage mask. The Help describes a transition boundary but does not name the kernel. Photoshop's feather is widely treated as Gaussian; the exact radius-to-sigma relationship is closed. Proposed: separable Gaussian blur on the float coverage mask, with the marching-ants contour drawn at the 50 % level. The Help's note that a feathered selection can become so faint that its edges are invisible, and trigger *"No pixels are more than 50% selected"*, is consistent with a coverage mask whose maximum falls below 0.5. Mark inferred.

### Smooth

The Help states the algorithm explicitly as a majority rule over a neighbourhood of the given `Sample Radius`: if more than half the surrounding pixels are selected the centre stays selected (and its unselected neighbours are added); otherwise it is removed. This is a binary median / majority filter. The Help does not state the neighbourhood shape; a square window of side `2·radius+1` is the natural reading, a disk is also plausible. Mark shape as inferred.

### Refine Edge (behavioral parity only, algorithm TBD)

Refine Edge is an alpha-matting operation: given a trimap implied by the selection and the refinement radius, estimate fractional foreground coverage and, for `Decontaminate Colors`, a foreground colour estimate. Adobe's implementation is closed and the community has publicly documented it in the direction of closed-form matting / colour-line models. Smart Radius adapts the trimap width to local edge softness. Mark all internals as *behavioral parity only*. Full detail belongs in `08-selection/refine-edge.md`.

## Rust module mapping

- `pictura_selection::ops` — dispatch enum `ModifyOp { Border(u32), Expand(u32), Contract(u32), Feather(f32), Smooth(u32) }`.
- `pictura_selection::grow` — `grow(selection, reference_color, tolerance, mode: Adjacency)`; `Adjacency::{Contiguous, Global}` covers Grow and Similar with one routine.
- `pictura_selection::morph` — `dilate`, `erode`, `distance_transform`, `border_band` on a `CoverageMask`.
- `pictura_selection::feather` — `feather(mask: &CoverageMask, radius_px: f32) -> CoverageMask`; separable Gaussian.
- `pictura_selection::smooth` — `majority_filter(mask: &CoverageMask, radius: u32) -> CoverageMask`.
- `pictura_selection::refine` — `RefineParams` + `refine(mask, params) -> RefinedOutput` (mask / new layer / new document).
- `pictura_mask::CoverageMask` — single-channel `f32` coverage `[0,1]` at document resolution, the shared currency of every operation here.

Data crossing the boundary: `CoverageMask` is retained in the Rust core; Qt receives only a downsampled preview or a vector contour for the marching ants, never a full mask per frame.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `ModifyDialogs` | `QDialog` subclasses | Expand/Contract/Border/Smooth/Feather numeric entry, validation against ranges |
| `FeatherSpinBox` | `QDoubleSpinBox` | Decimal feather entry (CS6 precision); `Shift+F6` target |
| `RefineEdgeDialog` | `QDialog` | All Refine Edge controls; embeds `RefineBrushOverlay` |
| `RefineBrushOverlay` | `QGraphicsItem` | Refine Radius / Erase Refinements brush cursor, `Shift+E` toggle |
| `SelectionMenu` | `QMenu` | Grow/Similar/Modify submenu state (disabled on Bitmap/32-bit) |

Widgets over QML: these are dense, keyboard-first modals consistent with `ARCH-003`. The Refine Edge brush overlay lives in the `QGraphicsView` vector layer above the GPU canvas.

## Data-model impact

- Every operation consumes and produces a `CoverageMask`; the selection is document state (see `08-selection/selection-model.md`), not a layer.
- **Undo granularity:** one history state per command invocation. Grow/Similar and the Modify geometry ops are cheap to re-derive; the pre-operation mask (or its bounding box) should be stored for lossless undo rather than a full-document snapshot.
- **Serialization:** none of these commands write to PSD directly; a subsequent `Select > Save Selection` is what materialises the mask as an alpha channel (see `save-and-load-selections.md`).
- **Refine Edge with `Decontaminate Colors`** is a pixel-changing operation and must obey the same new-layer/master-document constraints; its undo record must include affected source tiles.
- `Tolerance`, `Anti-aliased`, and tool feather are tool state, not document state.

## Edge cases

- **Bitmap mode / 32-bpc:** `Grow` and `Similar` are explicitly unavailable.
- **Canvas edge:** `Expand` does not move a border lying on the canvas edge.
- **Empty / 1-px selection:** Border with a large width can consume the whole selection; Expand/Contract by more than the selection's inradius can empty it — define empty-selection handling (`Select > Reselect` should still restore the last selection per Help).
- **Tiny selection + large Feather:** can produce the *"No pixels are more than 50% selected"* condition; the UI must offer both "decrease radius / enlarge selection" and "accept as-is" exactly as documented.
- **CMYK / Lab:** tolerance distance is computed in the document's working space; naive per-channel distance matches Photoshop's per-channel semantics but is not proven.
- **16-bit / 32-bit:** coverage masks are float internally; 32-bit Refine Edge must not clamp colour.
- **PSB / huge documents:** dilate/erode/feather over 300,000-px dimensions require tiled/distance-transform implementations and a memory budget.
- **GPU unavailable:** all Modify ops are CPU; previews defer.
- **Non-contiguous / soft masks:** Grow over a feathered mask must decide whether to expand the soft edge or the 50 % region — unspecified by Adobe.
- **Undo/redo:** undo must restore the exact prior coverage (thresholded or soft).

## Parity acceptance criteria

- Given a Magic Wand `Tolerance` of `t` and a selection, `Select > Grow` adds exactly the adjacent pixels within `t` and no pixel outside it, verified against a reference image.
- Given the same selection and tolerance, `Select > Similar` adds all in-tolerance pixels image-wide, including disconnected regions.
- `Select > Grow` and `Select > Similar` are both greyed out / rejected on Bitmap and 32-bpc documents.
- Given a rectangular selection, `Modify > Border` with width `w` produces a band whose inner and outer edges each sit `w/2` px from the original border, within 1 px.
- Given a selection touching the canvas edge, `Modify > Expand` leaves the canvas-edge portion unchanged.
- Given a hard rectangular selection, `Modify > Feather` with radius `r` moves the 50 % coverage contour no more than 1 px and produces a monotonic coverage ramp over roughly `2r`.
- Given a patchy magic-wand selection, `Modify > Smooth` with radius `r` removes isolated selected pixels smaller than the majority threshold in their neighbourhood.
- Given a selection, `Modify > Feather` followed by `Select > Reselect` returns the pre-feather selection.
- Given `Refine Edge` with `Decontaminate Colors` enabled, the command forces output to a new layer/document (no in-place destructive edit on the active layer).
- All commands produce a single undo step each.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — primary CS6 Help corpus (curl → `/tmp`, `pdftotext`). Established: Grow/Similar use the Magic Wand tolerance and are unavailable on Bitmap/32-bpc; Expand/Contract 1–100 px and canvas-edge exemption; Border 1–200 px centred half-in/half-out; Smooth majority rule 1–100 px; Feather dialog and 0–250 px tool range; anti-aliasing availability and ordering; the full Refine Edge control list (View Mode, refine brushes, Smart Radius, Radius, Smooth, Feather, Contrast, Shift Edge, Decontaminate Colors + Amount, Output To); Magic Wand option definitions (Tolerance 0–255, Anti-aliased, Contiguous, Sample All Layers); "Support for decimal feather values…" and "Feather radius recalled in Make Selection from path dialog" in the CS6 What's New > Selections list.
- `https://html.duckduckgo.com/html/?q=Photoshop+CS6+%22Save+Selection%22+default+channel+name+Alpha+1` — search results page; surfaced the community claim of the `Alpha 1` default channel name (used in SEL-012).
- `https://search.brave.com/search?q=Photoshop+CS6+%22Transform+Selection%22+Select+menu` — search results page; confirms `Select > Transform Selection` (used in SEL-011).
- `https://search.brave.com/search?q=Photoshop+%22Grow%22+%22Similar%22+Magic+Wand+tolerance+default+32` — search results page; community snippets describing the Magic Wand default tolerance of 32.

Consulted as search-result snippets only (not individually fetched; community-reported):

- `http://www.sketchpad.net/channels2.htm` — states that Photoshop gives a new channel the default name `Alpha 1`.
- `https://brighthub.com/multimedia/photography/articles/25654.aspx` and `https://jkost.com/blog/2021/07/25-shortcuts-and-tips-for-creating-better-selections-in-photoshop.html` — Magic Wand default `Tolerance` 32.

Not used in this pass:

- `helpx.adobe.com` (HTTP 403).
- `https://www.photoshopessentials.com` — not fetched.

## Open questions

- **Colour-distance metric for Grow/Similar.** Adobe publishes the tolerance range but not the distance formula. Resolve by comparing Grow/Similar output against a CS6 reference at several tolerances on a colour ramp.
- **Feather kernel and radius→sigma.** The Help says "blur" only. Resolve with a CS6 feather reference and a kernel fit.
- **Modify structuring element shape.** Square vs. disk for Expand/Contract/Border/Smooth is unstated. Resolve against CS6 edge pixel counts.
- **Whether Modify ops threshold the selection first.** Behaviour on soft/anti-aliased selections is undocumented. Resolve with a feathered-then-expanded reference.
- **Feather dialog numeric range and decimal step.** The CS6 What's New note implies decimals; the exact min/max/step are not in the fetched text. Resolve from a CS6 UI capture.
- **Refine Edge is described in the task as a "CS6 addition".** The CS6 Help presents it as an established command and the What's New list does not introduce it; it predates CS6 (CS5). Resolve the version provenance before labelling it `New in CS6`; treat as `Changed` pending evidence.
- **Grow/Similar reference colour persistence.** Whether the similarity is measured against the original clicked colour or the current selection's boundary colours is not stated. Resolve with repeated-Grow tests against CS6.
