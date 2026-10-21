# Paths & Vector Selection

- **Spec ID:** `SEL-014`
- **Status:** `Draft`
- **Parity tier:** `Core` (path→selection, selection→path, clipping paths; 3D extrusion and type-on-path excluded as Extended/other-domain)
- **New in CS6:** `Changed` — CS6 recalls the **Feather radius** in the Make Selection-from-path dialog (CS6 What's New > Selections), continues the CS5 move of vector-mask controls into the **Properties** panel, and adds vector-curve dragging/90° pixel-grid behaviour (see `TOOL-001`). Path→selection and Make Work Path themselves are unchanged.
- **Depends on:** `03-tools/pen-and-path-tools.md`, `03-tools/path-selection-tools.md`, `08-selection/selection-model.md`, `05-layers/vector-masks-and-clipping-masks.md`, `05-layers/layer-masks.md`, `01-architecture/file-formats.md`, `10-workflow-io/printing.md`.

> All crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not.

## CS6 behavior

### Paths are vector outlines

Paths are resolution-independent Bezier outlines drawn with the Pen or Shape tools into the **Paths** panel. They can be turned into selections, filled/stroked with colour, edited by moving anchor points and direction handles, and — once saved and named — designated as a **clipping path** for export. Photoshop keeps paths in the document (the Work Path is temporary until named).

### Path → selection

Any **closed** path can be converted to a selection border, and can be **added to, subtracted from, or combined with** the current selection.

- Using current settings: select the path, then click the **Load Path as a Selection** button at the bottom of the Paths panel, or `Ctrl`/`Cmd`-click the path thumbnail.
- With settings: `Alt`/`Option`-click the `Load Path As A Selection` button, `Alt`/`Option`-drag the path onto it, or choose **`Make Selection`** from the Paths panel menu. The dialog offers:
  - `Feather Radius` — how far inside and outside the selection border the feather edge extends.
  - `Anti-aliased` — finer transition; when checked the Help requires `Feather Radius = 0`.
  - `Operation` — `New Selection`, `Add To Selection`, `Subtract From Selection`, `Intersect With Selection`. If the path and selection do not overlap under Intersect, nothing is selected.

CS6 detail: the Make Selection dialog **recalls the last Feather radius** (CS6 What's New).

### Selection → path

`Make Work Path` turns any selection into a path:

- Click the **Make Work Path** button in the Paths panel (uses the current tolerance without a dialog), `Alt`/`Option`-click it, or choose **`Make Work Path`** from the panel menu.
- `Tolerance` values range **0.5–10 px**. Higher tolerance = fewer anchor points and a smoother path. The Help advises a higher tolerance if a resulting clipping path prints poorly.
- `Make Work Path` **eliminates any feathering** applied to the selection and can alter the selection shape depending on tolerance.

### Clipping paths

A saved path can be designated as a **clipping path** so that part of the image becomes transparent when the file is placed in a page-layout or vector-editing application. The Help's Paths overview lists this as one of the uses of paths: a clipping path makes part of an image transparent when the image is exported to a page-layout or vector-editing application.

Format interaction (fetched Help):

- **EPS supports clipping paths but does not support alpha channels.**
- DCS is an EPS variant; DCS 2.0 retains spot channels.
- The Help's forward references ("Create transparency using image clipping paths", "Printing image clipping paths") point to sections **not present in the extracted PDF text**, so the exact dialog (path chooser, flatness) is not sourced here — see Open questions.

### Path vs raster mask

- A **vector mask** is a resolution-independent path that clips out the contents of the layer. It is created with the pen/shape tools, edited as a path, and stored as a path in the document. It can be disabled/enabled, its density/feathering adjusted, and removed; rasterising it (`Layer > Rasterize > Vector Mask`) converts it to a layer mask **one-way**.
- A **layer mask** is a raster alpha channel. Loading its boundary as a selection uses the same modifier-click model as channels (`SEL-012`).
- Selection↔path conversion is the bridge between the raster selection world and the vector mask world: a selection → work path → vector mask gives a resolution-independent clip; a vector mask/path → selection gives editable raster coverage. Cross-ref `05-layers/vector-masks-and-clipping-masks.md`, `05-layers/layer-masks.md`.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Window > Paths` | Panel | — | Paths list, fill/stroke, clipping-path designation |
| Paths panel | Buttons | — | `Fill Path`, `Stroke Path`, `Load Path As A Selection`, `Make Work Path From Selection`, `New Path`, `Delete` |
| Paths panel menu | Menu | — | `Make Selection`, `Make Work Path`, `Clipping Path`, `Fill Path`, `Stroke Path`, `Save Path`, `Duplicate Path`, `Delete Path` |
| Paths panel | Modifier-click | `Ctrl`/`Cmd`-click thumbnail | Load path as selection (current settings) |
| Paths panel | Modifier-click | `Ctrl+Shift` / `Ctrl+Alt` / `Ctrl+Shift+Alt`-click name | Add / subtract / intersect with path (Help shortcut table) |
| `Make Selection` dialog | Dialog | — | Feather Radius, Anti-aliased, Operation |
| `Make Work Path` dialog | Dialog | — | Tolerance 0.5–10 px |
| `Layer > Vector Mask > …` | Menu | — | `Reveal All`, `Hide All`, `Current Path`, `Disable`, `Enable` |
| `Layer > Rasterize > Vector Mask` | Menu | — | One-way conversion to layer mask |
| Pen / Shape tools | Tools | `P` / `U` | Draw/edit paths |
| Path Selection tool | Tool | `A` | Select entire path/components |
| Direct Selection tool | Tool | `A` (cycle) | Select anchors/segments |
| `File > Save As > Photoshop EPS` | Menu | — | `Include Vector Data`, clipping paths |
| `File > Export > Paths to Illustrator` | Menu | — | Exports paths as AI (cross-ref `10-workflow-io/export-formats.md`) |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Make Work Path Tolerance | float (px) | current setting (community ~2.0) | 0.5–10 | Higher = fewer anchors, smoother; default not stated in Help |
| Make Selection Feather Radius | float (px) | recalls last value in CS6 | ≥ 0 | CS6 What's New |
| Make Selection Anti-aliased | bool | not stated | on / off | Requires Feather Radius = 0 when on |
| Make Selection Operation | enum | New Selection | New / Add / Subtract / Intersect | |
| Path operation modifiers | key + click | — | Add / Subtract / Intersect | `Ctrl+Shift` / `Ctrl+Alt` / `Ctrl+Shift+Alt`-click |
| Vector mask density | percent | 100 % | 0–100 % | CS6 Properties panel |
| Vector mask feather | float (px) | 0 | ≥ 0 | CS6 Properties panel |
| Clipping path selection | enum | none | any saved path | Saved path required |
| Clipping path flatness | int | not stated | not stated | Not in fetched PDF — see Open questions |
| Include Vector Data (EPS) | bool | off | on / off | Keeps shapes/type vector; rasterised on reopen in Photoshop |

## Algorithms & pipeline

### Path representation

A path is a sequence of subpaths, each a closed or open sequence of cubic Bezier segments with anchor points and direction handles. The document stores named paths (the transient Work Path has no name until saved). Selection conversion only applies to **closed** subpaths; open subpaths are implicitly closed for the purpose of making a selection.

### Path → selection (anti-aliased coverage)

Rasterising the path to selection coverage is a scanline polygon/Bezier fill with anti-aliasing coverage (analytic or supersampled) yielding a `CoverageMask` in `[0,1]`. `Feather Radius` then blurs that coverage (same feather pipeline as `SEL-010`); `Anti-aliased` selects the finer edge transition. Mark the precise coverage/AF algorithm as *behavioral parity only*. The resulting mask composes with the existing selection via the standard `Replace/Add/Subtract/Intersect` (shared with `SEL-012`).

### Selection → path (curve fitting)

`Make Work Path` extracts the selection's **50 % coverage contour** and fits cubic Bezier curves to it. `Tolerance` is the fit-error budget in pixels: larger tolerance → fewer anchor points and smoother curves. Feathering is discarded first (the Help states this explicitly), implying the contour is taken from a hard threshold of the current selection. Proposed implementation: contour extraction (marching squares), Ramer–Douglas–Peucker or Schoenberg–Whitney simplification seeded by tolerance, then Schneider-style cubic fitting. Mark inferred; Adobe's exact fitter is closed.

### Vector mask rendering

A vector mask is rasterised into coverage per render pass at the current zoom (resolution-independent), optionally blurred by the mask `Feather` and scaled by `Density`, then multiplied with the layer's alpha. Because it is rasterised at draw time, it stays sharp at any zoom; rasterising converts it permanently to a layer mask. See `01-architecture/gpu-rendering-pipeline.md`.

### Clipping path on export

When writing EPS, the designated path is emitted as the PostScript clipping path around the image data, with `Include Vector Data` optionally preserving shapes/type. The image's background outside the clipping path is transparent to the page-layout application. Exact EPS emission belongs to `01-architecture/file-formats.md` / `10-workflow-io/printing.md`.

## Rust module mapping

- `pictura_geom::Path` — `Vec<SubPath>`, `SubPath { segments: Vec<CubicBezier>, closed: bool }`, anchor/handle model.
- `pictura_selection::rasterize_path` — `path_to_mask(path: &Path, aa: bool, feather: f32) -> CoverageMask`; scanline Bezier fill with coverage.
- `pictura_geom::fit` — `mask_to_path(mask: &CoverageMask, tolerance: f32) -> Path`; contour + cubic fit.
- `pictura_geom::contour` — marching-squares contour extraction at 0.5 coverage (shared with `SEL-011`).
- `pictura_paths::PathsModel` — named paths, Work Path, clipping-path designation.
- `pictura_layers::vector_mask` — `VectorMask { path: PathId, density, feather }`; render-time rasterisation.
- `pictura_psd` — path resources (PSD path records) and clipping-path name resource; EPS/DCS clipping-path emission lives in `pictura_export`.
- Data crossing the boundary: `PathId`, `Path` as anchor/handle arrays, `CoverageMask`, `Combine`. Path geometry may cross to Qt for the overlay; masks do not.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `PathsPanel` / `PathsModel` | `QDockWidget` / `QAbstractItemModel` | Path list, fill/stroke actions, clipping-path flag, modifier-click load |
| `MakeSelectionDialog` | `QDialog` | Feather Radius, Anti-aliased, Operation; recalls prior feather (CS6) |
| `MakeWorkPathDialog` | `QDialog` | Tolerance 0.5–10 spin box |
| `PathOverlay` | `QGraphicsItem` | Anchors/handles/direction lines, direct-selection editing |
| `VectorMaskProperties` | `QWidget` (Properties panel) | Density/Feather/Delete/Disable for vector masks (CS6) |
| `PathThumbnailDelegate` | `QStyledItemDelegate` | Renders the path thumbnail with the clipping-path marker |

Widgets over QML: the Paths panel and property controls are dense and keyboard-first; widgets fit `ARCH-003`. The path editing overlay is a vector-layer `QGraphicsItem`.

## Data-model impact

- **New node type:** `Path { id, name?, subpaths, is_work_path, is_clipping_path }`. Paths live in the document, independent of layers; a vector mask references a `PathId`.
- **PSD/PSB:** paths serialise in the document path records; the clipping-path name is a document resource. Exact keys belong in `01-architecture/file-formats.md`.
- **Vector mask node:** `{ path: PathId, density: f32, feather: f32, enabled: bool }` attached to a layer/group; rendering reads it, rasterisation converts it to a layer mask (one-way, Help-sourced).
- **Selection state:** path→selection writes the document `CoverageMask`; selection→path writes a Work Path. Both are history states.
- **Undo granularity:** path edits (anchor moves, Make Selection, Make Work Path, clipping-path flag) are individual history states; path geometry is cheap to snapshot so undo is lossless.
- **No XMP:** paths are document geometry, not metadata. Clipping-path designation is part of the path record.

## Edge cases

- **Open paths:** cannot directly become a selection; implicitly closed. Behaviour of subpaths with holes uses even-odd/nonzero winding — Adobe's fill rule is not stated in the fetched text (open question).
- **Empty selection → Make Work Path:** no contour; the command must be disabled.
- **Feathered selection → Make Work Path:** feathering is discarded; the path follows the hard contour.
- **High tolerance:** can collapse small/1-px selections into a degenerate path; guard against empty results.
- **Path partly off-canvas:** allowed; selection coverage clips to the canvas.
- **Anti-aliased + non-zero Feather:** Help says set Feather Radius to 0 when Anti-aliased is on — the UI should enforce or warn.
- **Vector mask on Background layer:** Background cannot take a vector mask until converted to a normal layer.
- **Rasterize > Vector Mask is one-way:** the UI must not imply reversibility.
- **Bitmap / Indexed / Multichannel:** path→selection behaviour must be checked per colour mode; Indexed may lack the usual anti-aliased coverage model.
- **16/32-bit:** path rasterisation produces coverage independent of bit depth; feather in float must not clamp.
- **PSB / huge docs:** contour extraction and curve fitting over 300,000-px masks must be tiled/streamed.
- **GPU unavailable:** path fill/stroke and overlays must fall back to CPU vector rendering.
- **Clipping path in EPS only:** saving to formats that ignore clipping paths (e.g. JPEG/PNG) must warn or apply the path as transparency where supported.
- **Undo/redo:** separate the document selection, the Work Path, and any named path so undo never conflates them.

## Parity acceptance criteria

- Given a closed path, clicking `Load Path As A Selection` produces a selection whose 50 % contour matches the path within 1 px, anti-aliased by default.
- Given the `Make Selection` dialog with `Anti-aliased` checked, `Feather Radius` is forced to 0 (or the operation is rejected), matching the Help.
- Given `Make Selection` with `Operation = Intersect` and a path not overlapping the selection, the result is an empty selection.
- Given a selection, `Make Work Path` with a low tolerance yields more anchor points than with a high tolerance on the same selection.
- Given a feathered selection, `Make Work Path` produces a path following the hard (non-feathered) boundary.
- Given a CS6 session, `Make Selection` reopens with the previously used Feather radius (CS6 recall).
- Given a vector mask, zooming in keeps the mask edge sharp (resolution-independent), while rasterising it (`Layer > Rasterize > Vector Mask`) produces a pixel-resolution layer mask.
- Given a designated clipping path, saving to EPS embeds it; the same document saved to a format without clipping-path support warns or drops it.
- Given a path saved and reloaded in PSD, its geometry, name, and clipping-path designation round-trip.
- Given `Ctrl+Shift` / `Ctrl+Alt` / `Ctrl+Shift+Alt`-clicking a path name, the path adds / subtracts / intersects with the selection identically to the dialog operations.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — primary CS6 Help corpus (curl → `/tmp`, `pdftotext`). Established: "Converting between paths and selection borders" (path→selection with Feather Radius, Anti-aliased, and Operation; selection→path via Make Work Path with Tolerance 0.5–10 and feather elimination); Paths-panel load/make-work-path buttons and shortcut table (Load path, add/subtract/intersect); path uses including clipping path; vector mask definition, add/edit/enable/disable, density/feather in CS5 Masks or CS6 Properties panel, and one-way `Rasterize > Vector Mask`; "Load selections from a layer or layer mask's boundaries"; EPS supports clipping paths but not alpha channels; DCS variants; CS6 What's New > Selections ("Feather radius recalled in Make Selection from path dialog") and > Transform notes; `Include Vector Data` EPS option.
- `https://search.brave.com/search?q=Photoshop+CS6+%22Transform+Selection%22+Select+menu` — search results page (context for the selection-border/vector distinction).
- `https://html.duckduckgo.com/html/?q=Photoshop+CS6+%22Save+Selection%22+default+channel+name+Alpha+1` — search results page (context for alpha-channel/selection storage cross-reference).

Not used in this pass:

- `helpx.adobe.com` (HTTP 403).
- Adobe PSD file-format spec (needed to pin path-record and clipping-path resource keys).

## Open questions

- **Clipping-path dialog details.** The Help references "Create transparency using image clipping paths" / "Printing image clipping paths" but those sections are absent from the extracted PDF text. The path chooser, flatness, and export behaviour must be sourced elsewhere (Adobe file-format spec or a CS6 UI capture).
- **Make Work Path direction.** Whether Photoshop fits the contour clockwise/counter-clockwise and in what order subpaths appear is undocumented; relevant to round-trip path identity. Resolve with CS6 comparison.
- **Path fill rule.** Even-odd vs nonzero winding for self-intersecting paths/holes is not stated. Resolve from a CS6 reference path.
- **Tolerance→anchor-count mapping.** The exact fitting algorithm and its error metric are closed; the proposed RDP/Schneider pipeline is inferred. Validate against CS6 output.
- **Anti-aliased + Feather enforcement.** The Help says to set Feather Radius to 0 when Anti-aliased is on; whether the dialog enforces this or merely advises is unclear. Resolve from a CS6 UI capture.
- **Vector mask on groups and Smart Objects.** Availability and interaction with smart filters need confirmation in `05-layers/vector-masks-and-clipping-masks.md`.
- **Default Make Work Path tolerance.** The Help gives the range but not the shipped default (community reports ~2.0). Resolve from a CS6 UI capture.
- **PSD path-record encoding.** The precise resource IDs and coordinate representation must come from the Adobe file-format spec, not the Help PDF.
