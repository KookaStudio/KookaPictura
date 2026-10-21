# Vanishing Point

- **Spec ID:** `FILT-102`
- **Status:** `Draft`
- **Parity tier:** `Core` for perspective planes and perspective-correct clone/repair/paint/transform; `Extended-only` for the `Measure` tool, `Render Measurements To Photoshop`, and DXF/3DS export.
- **New in CS6:** `No` — unchanged from CS5. The `Measure` tool and DXF/3DS export remain **Photoshop Extended** only; the CS6 Help documents the same dialog and toolset.
- **Depends on:** `01-architecture/document-model.md`, `01-architecture/undo-history.md`, `01-architecture/gpu-rendering-pipeline.md`, `03-tools/clone-stamp-and-pattern-stamp.md`, `03-tools/healing-brushes.md`, `05-layers/layer-masks.md`, `01-architecture/file-formats.md`, `04-image-ops/image-modes.md`

> All crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Behavior is taken from the fetched CS6 Help PDF unless
> marked *(inferred)*. Adobe's plane solver and retouch heuristics are closed;
> those parts are **behavioral parity only, algorithm TBD**.

## CS6 behavior

`Filter > Vanishing Point` simplifies perspective-correct editing. The user defines rectangular **perspective planes** aligned to surfaces in the image (walls, floors, building sides); subsequent painting, cloning, copying/pasting, and transforming obey the plane's perspective, so retouching stays correctly oriented and scaled.

**Dialog layout:** the **Vanishing Point menu** (Show Edges, Render Grids To Photoshop, Render Measurements To Photoshop, Export to DXF / 3DS), tool **Options**, a tool **Toolbox**, the session **preview**, and **zoom** options.

**Tools** (behave like their main-toolbox counterparts; selecting a tool changes the options):

| Tool | Behavior |
|---|---|
| `Edit Plane` | Selects, edits, moves, and resizes planes; drag corner nodes to reshape, drag inside to move, drag an edge node to scale; adjust `Grid Size`. |
| `Create Plane` | Defines the four corner nodes of a plane (click in the preview); adjusts size/shape and tears off new planes. |
| `Marquee` | Square/rectangular selections; moves or clones selections. Double-click inside a plane selects the entire plane. |
| `Stamp` | Paints with a sample of the image. **Unlike the main Clone Stamp, it cannot clone elements from another image.** |
| `Brush` | Paints a selected color in a plane. |
| `Transform` | Scales, rotates, and moves a floating selection via bounding-box handles; `Flip`/`Flop` options. |
| `Eyedropper` | Selects a paint color from the preview. |
| `Measure` *(Extended)* | Measures distances and angles in a plane; can `Link Measurements To Grid`. |
| `Zoom` / `Hand` | Preview navigation; hold `x` to temporarily zoom. |

**Planes and grid:**
- Define four corner nodes; the active plane shows a bounding box + grid. `Grid Size` can align the grid to image texture/pattern and help count items. *(Extended)* `Link Measurements To Grid` ties grid spacing to a `Length` value.
- Bounding-box/grid color encodes validity: **blue** = valid; **red** = invalid (aspect ratio cannot be calculated); **yellow** = invalid (some vanishing points cannot be resolved). Editing an invalid plane yields improperly oriented results.
- `Show Edges` toggles the grid, active selections, and plane boundaries (selections still flash while resized/repositioned).
- **Related planes:** `Ctrl`/`Cmd`-drag an **edge** node to tear off a new plane at **90°** to the existing one; planes remain related so edits stay scaled/oriented. Change the new plane's angle via `Alt`/`Option`-drag of the opposite mid-edge node, the `Angle` box, or the `Angle` slider. Once a child plane exists, the parent's angle can no longer be changed. Overlapping planes: `Ctrl`/`Cmd`-click to cycle.

**Selections and cloning:**
- `Marquee` options before drawing: `Feather` (blur the selection edge), `Opacity` (how much moved pixels obscure what's below), and a `Heal` mode — `Off` (no blending with color/shadow/texture), `Luminance` (blend with lighting only), or `On` (blend color, lighting, and shading). A selection may span multiple planes and wraps to each plane's perspective; `Shift` constrains to a square in perspective.
- Move a selection with `Move Mode`: `Destination` (select the area you move the marquee to) or `Source` (fill the selection from where you drag). `Shift` constrains movement to the grid.
- A selection with pixels is a **floating selection** (looks like a hovering layer but is not on a separate layer); it can be moved, rotated, scaled, flipped/flopped. Clicking outside deselects and pastes it, replacing pixels below; cloning a copy deselects the original. `Alt`/`Option`-drag the marquee creates a perspective copy.
- **Paste** from the clipboard makes a floating selection that conforms to whatever plane it is moved into; perspective is preserved when pasting between documents. Type must be rasterized before copying.

**Rendering / export:**
- `Render Grids To Photoshop` writes the session grid as **raster** (not vector) into the document; must be chosen per session; a separate layer is recommended.
- *(Extended)* `Render Measurements To Photoshop` writes measurements.
- *(Extended)* `Export to DXF` writes 3D plane information plus measurements; `Export To 3DS` writes rendered textures plus geometry. Both use a Save-file dialog.
- To preserve plane information in the document, save as **PSD, TIFF, or JPEG**.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Filter > Vanishing Point` | Menu | `Alt+Ctrl+V`* | Opens the dialog |
| Dialog — Vanishing Point menu | Menu | — | Show Edges, Render Grids, Render Measurements, Export DXF/3DS |
| Dialog — Toolbox | Toolbar | — | Edit Plane, Create Plane, Marquee, Stamp, Brush, Transform, Eyedropper, Measure*, Zoom, Hand |
| Dialog — Options | Panel | — | Changes with selected tool |
| Dialog — preview | Canvas | — | Edit surface |
| Dialog — zoom box / +/− | Control | `x` (temp) | Zoom in/out |
| Dialog — `OK` / `Cancel` | Buttons | — | Commit/discard the session |
| Document save as PSD/TIFF/JPEG | Save | — | Preserves plane info |

*Not confirmed in the CS6 PDF; listed as the conventional command.

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Grid Size | numeric/slider | — | preview grid units | Edit/Create Plane options |
| Angle | slider/numeric | 90° for tear-off | degrees | Child plane angle |
| Selection Feather | numeric (px) | 0 | ≥ 0 *(unverified)* | Marquee |
| Selection Opacity | numeric (%) | 100 *(unverified)* | 0…100 | Marquee; effect on moved pixels |
| Heal mode | enum | Off *(unverified)* | Off / Luminance / On | Marquee; blending of moved content |
| Move Mode | enum | Destination *(unverified)* | Destination / Source | Selection move behavior |
| Brush color | color | foreground *(unverified)* | any color | Brush/Eyedropper |
| Stamp/Brush size | numeric | — | tool options | Not tabulated in the PDF |
| Zoom | numeric | fit *(unverified)* | preset zoom levels | Preview |
| Measurement Length | numeric *(Extended)* | — | plane units | `Link Measurements To Grid` |

Exact ranges/defaults are not tabulated in the CS6 Help PDF; values marked *(unverified)* are listed under `## Open questions`.

## Algorithms & pipeline

*(inferred / behavioral parity only, algorithm TBD unless noted.)*

1. **Plane definition** — four image-space corner nodes define a quadrilateral. A homography maps it to a canonical (rectangular) plane space. Two vanishing points are derived from the quadrilateral's edge directions; the plane is **valid** only when the aspect ratio (red failure) and all vanishing points (yellow failure) resolve — matching the CS6 grid-color semantics (sourced: the color meanings; the solver is closed).
2. **Related planes** — tearing off an edge node constructs an adjacent homography sharing the parent's vanishing lines, initially at 90°, then rotatable. Child planes constrain parent geometry (sourced behavior).
3. **Selection / clone in perspective** — the marquee is transformed into each plane's canonical space; content is sampled and warped through the homography back to image space. `Heal` blends the moved pixels with the target using `Off`/`Luminance`/`On` (a clone/heal variant; core algorithm closed).
4. **Floating selection** — pixels are held off-surface and rendered through the current plane's homography; transform (move/rotate/scale/flip/flop) updates the quad; commit composites the warped pixels.
5. **Stamp / Brush** — painting operates in canonical plane space so strokes foreshorten correctly; `Stamp` samples only the same image.
6. **Grid rendering** — the canonical grid is projected back and rasterized into the layer.
7. **Export** *(Extended)* — plane geometry (+ measurements/textures) to DXF/3DS; the writer format is standard CAD/3D interchange, but the exact emitted entity types are TBD.

Persistence: plane data is preserved when saving **PSD/TIFF/JPEG** (sourced). The PSD container key/block that stores it is **not confirmed** here — see `## Open questions`.

## Rust module mapping

- `pictura_vp::Plane` — four image-space nodes + canonical mapping; `validity() -> Valid | InvalidAspect | InvalidVanishing`.
- `pictura_vp::PlaneSet` — related/tear-off planes, angle constraints, overlap cycling.
- `pictura_vp::solve::Homography` — derive canonical↔image homographies and vanishing points.
- `pictura_vp::Selection` / `FloatingSelection` — perspective marquee, transform (move/rotate/scale/flip/flop), copy/paste.
- `pictura_vp::retouch::{Stamp, HealMode}` — perspective clone and heal blending.
- `pictura_vp::grid` — grid model, `GridSize`, `LinkMeasurements`, raster rendering.
- `pictura_vp::measure` — *(Extended)* measurement model + linkage.
- `pictura_vp::export::{Dxf, ThreeDs}` — *(Extended)* plane/measurement/texture exporters.
- `pictura_filter::vanishing_point` — the filter entry point + `supported(mode, depth)`; owns the session and commits one history step.

Crossing types: `PlaneId`, `Plane`, `Homography`, `MoveMode`, `HealMode`, `TileStore`, `CanvasRect`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `VanishingPointDialog` | `QDialog` | Session shell: menu, toolbox, options panel, preview, OK/Cancel |
| `PlaneCanvas` | `QQuickWidget`/`QGraphicsView` | Preview, plane nodes, grid, selection marquee, floating-selection handles |
| `PlaneToolbar` | `QToolBar` | Edit/Create Plane, Marquee, Stamp, Brush, Transform, Eyedropper, Measure, Zoom, Hand |
| `PlaneOptionsPanel` | `QStackedWidget` | Per-tool options (Grid Size, Angle, Feather/Opacity/Heal, Move Mode, …) |
| `VpMenu` | `QMenu` | Show Edges, Render Grids/Measurements, Export DXF/3DS |
| `GridOverlay` | `QQuickItem` | Draw/size/color grid; link-to-measurements |
| `ExportDialog` | `QFileDialog` | DXF/3DS save (Extended) |

Widgets for menu/toolbar/options; QML/QtQuick for the interactive plane canvas and overlays (smooth GPU transforms, hit-testing handles).

## Data-model impact

- A Vanishing Point session is a single history step in Photoshop (the dialog edits a working copy). Integrated model: open a session command, apply on OK, discard on Cancel (`undo-history.md`).
- Rendered grids/measurements land as raster pixels on the active (or a new) layer (sourced: raster, not vector).
- Plane data is preserved in **PSD/TIFF/JPEG**; whether it is a layer additional-info block, a document block, or proprietary metadata is unconfirmed. If the project stores it, prefer a documented/owner-scoped block and preserve unknown Adobe data byte-for-byte (`document-model.md`, `file-formats.md`).
- No new document node type; planes attach to the image/layer being edited.
- *(Extended)* measurements and DXF/3DS export are output-only and do not modify the document.

## Edge cases

- **Standard vs Extended** — Measure, Render Measurements, and DXF/3DS export are absent in Standard; the UI must hide/gate them.
- **Invalid planes** — red/yellow grid must block or warn before edits that would be mis-oriented.
- **Parent-angle lock** — once a child plane is torn off, the parent angle control must be disabled.
- **Canvas bounds** — cloning/pasting beyond the canvas requires enlarging the canvas first (sourced); content outside bounds must not be silently lost.
- **Paste of type** — type layers must be rasterized before copy; non-raster clipboard content must be rejected clearly.
- **Selection spanning planes** — content must wrap per-plane exactly at plane boundaries.
- **Heal blending** — `Luminance`/`On` must not bleed color across plane seams.
- **JPEG save of plane data** — JPEG is lossy; plane metadata must survive, but repeated re-encode is a risk.
- **Mode/depth** — the PDF does not state supported modes/depths; define `supported()` and gate like other filters (unconfirmed).
- **GPU unavailable** — CPU transform/preview fallback required; cache keys include backend.
- **Very large documents / PSB** — floating selections and preview must be tiled; avoid full-document float copies.
- **Undo/redo** — Cancel must restore exactly; OK must be one reversible step.
- **Memory** — floating selections and multi-plane previews are memory-heavy; bound the working set.

## Parity acceptance criteria

- Given an image with a rectangular surface, `Create Plane` on its four corners yields a **blue** (valid) grid that aligns with the surface; deliberately crossing nodes yields **red** or **yellow**.
- Given a valid plane, a rectangular marquee drawn in it, when moved within the plane, stays perspective-warped (its projected edges follow the plane's vanishing points).
- Given two related planes torn off at 90°, an edit crossing the seam is continuous in perspective.
- Given `Heal = On`, moved content blends with the target's color, lighting, and shading; `Off` does not blend; `Luminance` blends only lighting.
- Given `Move Mode = Source`, dragging from the selection fills it from the source region; `Destination` selects the moved-to area instead.
- Given `Grid Size` alignment to a known texture, grid lines coincide with the texture within a pixel tolerance.
- Given `Render Grids To Photoshop`, the grid appears as raster pixels in the document window after `OK`.
- *(Extended)* Given `Export To 3DS`, the exported file contains the plane geometry and rendered texture; DXF contains geometry and measurements.
- Given a document saved as PSD/TIFF/JPEG, reopening Vanishing Point restores the planes.
- Given `Cancel`, the document is unchanged and no history step is added.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — CS6 Help corpus (downloaded, `pdftotext -layout`). Established: `Filter > Vanishing Point`; dialog layout; the full tool list and per-tool behavior (Edit/Create Plane, Marquee, Stamp, Brush, Transform, Eyedropper, Measure, Zoom, Hand); plane definition and grid; plane color semantics (blue valid / red aspect-ratio / yellow vanishing-point); tear-off related planes at 90°, angle adjustment, parent-angle lock, overlap cycling; `Show Edges`; marquee `Feather`/`Opacity`/`Heal` (Off/Luminance/On); Move Mode Destination/Source; floating selections and their transform/copy/paste behavior; paste-between-documents perspective preservation; raster `Render Grids To Photoshop`; Extended-only `Measure`, `Link Measurements To Grid`, `Render Measurements To Photoshop`, `Export to DXF`, `Export To 3DS`; PSD/TIFF/JPEG preservation of plane information.
- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` (same fetch) — established that the Smart Filters exclusion list includes Vanishing Point (used in `LAY-021`).

Marked inferred (not primary): the homography/vanishing-point solver, clone/heal core, persistence block, mode/depth gate, and export entity details.

## Open questions

- **PSD/TIFF persistence key.** Which additional-info block or document key carries Vanishing Point planes (and measurements)? Resolve by parsing CS6-authored PSD/TIFF files.
- **Plane solver parity.** The quadrilateral→vanishing-point solve and validity tests are closed. Define parity tolerance; resolve with test images.
- **Clone/heal algorithm.** `Stamp` and `Heal` internals are closed. Resolve with tolerance-based comparison or accept behavioral parity only.
- **Supported modes/depths.** The PDF gives no gate; determine whether Vanishing Point is 8/16/32-bit and RGB/CMYK/Lab limited. Resolve with a CS6 build test.
- **DXF/3DS fidelity.** Which entities/layers are emitted, and how textures are embedded in 3DS. Resolve by inspecting exported files.
- **Grid render multiplicity.** Confirm grids are per-session and raster-only (PDF says so) and define behavior when rendering to a masked/clipped layer.
- **Selection Feather/Opacity units and defaults.** Not tabulated; verify against the UI.
- **Performance model.** Whether the session works at full resolution or a proxy affects parity and memory; resolve with timing/profiling.
- **Post-CS6 changes.** Any CC-era Vanishing Point changes must be excluded from CS6 parity explicitly.
