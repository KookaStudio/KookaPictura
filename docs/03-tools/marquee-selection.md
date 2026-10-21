# Marquee Selection Tools

- **Spec ID:** `TOOL-002`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `No` — the marquee tools and their options carry over from CS5 unchanged.
- **Depends on:** `ARCH-002` document-model, `ARCH-007` undo-history, `TOOL-003`/`TOOL-004` (sibling selection tools), `08-selection/selection-model.md`, `08-selection/refine-edge.md`, `01-architecture/performance-targets.md`.

> All crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not.

## CS6 behavior

The marquee tools create geometric selections:

- **Rectangular Marquee** (default `M`) — drag a rectangle; `Shift`-drag constrains to a square.
- **Elliptical Marquee** (`Shift+M` to cycle, or Alt-click the tool) — drag an ellipse; `Shift` constrains to a circle.
- **Single Row** and **Single Column** — define a 1-pixel-wide row or column; no drag extent, just a click to place.

All four share the selection-mode buttons — **New**, **Add To**, **Subtract From**, **Intersect With** — plus a **Feather** field and, for the Elliptical Marquee, an **Anti-alias** checkbox. The Rectangle and Ellipse additionally have a **Style** menu: `Normal` (proportions follow the drag), `Fixed Ratio` (height-to-width ratio, decimal values accepted), and `Fixed Size` (whole-number pixel values, with units beyond px such as inches or centimetres). Selection behavior modifiers, confirmed in the CS6 Help shortcut tables: `Shift`-drag constrains a square/circle when no other selection is active; `Alt`-drag draws from the centre; `Shift+Alt`-drag does both; `Space`-drag repositions the marquee while still dragging; `Shift`/`Alt`/`Shift+Alt`-drag adds/subtracts/intersects with any selection tool.

The marquee can snap to guides, grids, slices, and document bounds when `View > Snap` (or `View > Snap To`) is enabled. The marching-ants edge is toggled by `View > Show > Selection Edges` (per-selection) or `View > Extras` (globally). The `Refine Edge` button in the options bar (or `Select > Refine Edge`, `Ctrl+Alt+R`) refines the working edge, and `Select > Transform Selection` transforms the selection border geometrically.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Tools panel | Tool | `M` | Rectangular Marquee is the default `M` tool |
| Tools panel | Hidden tool | `Shift+M` | Cycles Rectangular ↔ Elliptical (when "Use Shift Key for Tool Switch" is on) |
| Tools panel | Hidden tools | — | Single Row and Single Column are unassigned by default; reach via Alt-click |
| Options bar | Buttons | — | New / Add To / Subtract From / Intersect With |
| Options bar | Spin box | — | Feather (0–250 px) |
| Options bar | Checkbox | — | Anti-aliased (Elliptical Marquee only) |
| Options bar | Combo + fields | — | Style: Normal / Fixed Ratio / Fixed Size (+ Width/Height) |
| Options bar | Button | `Ctrl+Alt+R` | Refine Edge |
| Canvas | Gesture | `Shift`-drag | Constrain to square/circle |
| Canvas | Gesture | `Alt`-drag | Draw from centre |
| Canvas | Gesture | `Shift+Alt`-drag | Constrain + from centre |
| Canvas | Gesture | `Space`-drag | Reposition marquee mid-drag |
| Canvas | Modifier | `Shift`/`Alt`/`Shift+Alt` | Add / subtract / intersect (any selection tool) |
| `View > Show > Selection Edges` | Menu | — | Toggle marching ants for current selection |
| `View > Snap` / `Snap To` | Menu | — | Snap to guides/grid/slices/document bounds |
| `Select > Transform Selection` | Menu | — | Transform the border |
| `Select > Modify > Feather` | Menu | — | Feather an existing selection |
| `Select > All` / `Deselect` / `Reselect` | Menu | `Ctrl+A` / `Ctrl+D` / `Ctrl+Shift+D` | Standard selection commands |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Selection mode | enum | New | New / Add / Subtract / Intersect | If nothing selected, New; some tools auto-switch to Add |
| Feather | int (px) | 0 | 0–250 | Defined at tool time; see `Select > Modify > Feather` for existing selections |
| Anti-aliased | bool | on | on / off | Elliptical Marquee only; must be set before selecting |
| Style | enum | Normal | Normal / Fixed Ratio / Fixed Size | Rectangle and Ellipse only |
| Ratio Width | float | — | decimals allowed | e.g. `2 : 1` for twice as wide as high |
| Ratio Height | float | — | decimals allowed | Appears when Style = Fixed Ratio |
| Size Width | int + unit | — | whole px, in, cm, … | Appears when Style = Fixed Size |
| Size Height | int + unit | — | whole px, in, cm, … | Appears when Style = Fixed Size |
| Snap | bool | on | on / off | `View > Snap` |

## Algorithms & pipeline

### Selection representation

A selection is a document-sized **coverage mask** with per-pixel values in `[0, 255]` for 8-bit channels (or 0..1 float conceptually). Geometric marquees rasterise a hard-edged region and then combine it with the existing mask using the mode:

```text
combine(existing, new, mode):
    New        -> new
    Add        -> max(existing, new)
    Subtract   -> min(existing, 255 - new)     # i.e. existing * (1 - new)
    Intersect  -> existing * new / 255
```

Anti-aliasing and feathering are applied to the *new* primitive before combining. This matches the CS6 rule that feather/anti-alias are set before the tool is used and cannot be added afterwards; later feathering of a finished mask is a separate `Select > Modify > Feather` operation.

### Rasterising the primitives

- **Rectangle / Single Row / Single Column:** clip the rectangle to canvas bounds and set coverage 255 inside. Single Row/Column is a 1-px band; Style does not apply.
- **Ellipse:** evaluate the normalised ellipse equation per pixel; anti-aliased mode computes approximate fractional coverage at boundary pixels. Source-over / coverage methods (e.g. analytic circle coverage or 4×4 supersampling) are implementation choices; Photoshop's exact AA filter is unspecified — mark *behavioral parity only*.
- **Fixed Ratio / Fixed Size:** constrain the drag geometry before rasterising. Fixed Ratio keeps `W/H` at the entered ratio; Fixed Size ignores the drag extent and places the entered rectangle/ellipse centred on the mousedown (with `Alt`-draw-from-centre semantics).
- **Feather:** blur the hard mask with a kernel whose support scales with the feather radius. Photoshop's feather is not a plain Gaussian; the exact profile is unspecified. A Gaussian with `σ ≈ feather / 2` is the conventional approximation — mark inferred. Feather is a scalar distance (px), applied isotropically.

### Marching ants

The visible edge is the marching-ants outline traced from the mask's 50%-coverage contour (or the analytic primitive for an uncommitted marquee). It is drawn in the vector overlay layer, not into pixel data, and is view-state only.

## Rust module mapping

- `pictura_selection::SelectionMask` — document-sized coverage buffer (`u8` or `f32`), with `combine(mode)`.
- `pictura_selection::mode::SelectionMode` — `{ New, Add, Subtract, Intersect }`.
- `pictura_selection::marquee` — `RectMarquee`, `EllipseMarquee`, `SingleRowMarquee`, `SingleColumnMarquee`; `MarqueeStyle { Normal, FixedRatio(w, h), FixedSize(w, h) }`.
- `pictura_selection::rasterize` — `rasterize_rect`, `rasterize_ellipse`, `rasterize_band`; anti-alias coverage routines.
- `pictura_selection::feather` — `feather(mask, radius_px)`.
- `pictura_selection::contour` — 50%-threshold marching-squares contour for the overlay.
- `pictura_tools::marquee` — `MarqueeTool { kind, mode, feather, anti_alias, style }` driving interaction.

Data crossing the boundary: `NodeId`/selection id, `Rect2`, `MarqueeStyle`, `SelectionMode`, and mask tiles. Mask tiles cross as borrowed buffers or via shared memory; the overlay receives a contour polyline (`Vec<Point2>`) rather than the full mask.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `MarqueeOptionsBar` | `QWidget` | Mode buttons, feather spin box, anti-alias check, style combo, Refine Edge button |
| `EllipticalMarqueeOptionsBar` | `MarqueeOptionsBar` | Adds the anti-alias check (only for the ellipse) |
| `FixedRatioFields` / `FixedSizeFields` | `QDoubleSpinBox` / `QSpinBox` + unit combo | Shown conditionally by Style |
| `SelectionModeButtonGroup` | `QButtonGroup` | Four exclusive mode buttons, shared with other selection tools |
| `MarqueeOverlayItem` | `QGraphicsItem` | Live rectangle/ellipse rubber band during drag |
| `MarchingAntsItem` | `QGraphicsPathItem` | Animated dashed contour over the committed mask |
| `RefineEdgeDialog` | `QDialog` | Shared with `08-selection/refine-edge.md` |

Widgets over QML for the same reasons as `ARCH-003`: tool options are dense, keyboard-centric, and live in the docked options bar. The live rubber band and ants are `QGraphicsView` items in the vector overlay layer.

## Data-model impact

- The selection is a first-class document object; the tool writes a new mask and pushes one undo **command** per completed marquee (drag release), not per mouse-move. Intermediate drag states are preview-only.
- Undo record shape: `SelectionChange { before: MaskSnapshot, after: MaskSnapshot, region: Rect2 }` — a bounding-box tile snapshot keeps memory bounded.
- Saved selections serialise to alpha channels (PSD) per `08-selection/save-and-load-selections.md`; marquee state (tool, mode, feather) is preference/tool-preset state, not document state.
- The 50%-contour is derived, never stored.

## Edge cases

- **Empty / 1-px documents:** a 1-px doc can hold only a 1-px selection; Single Row/Column on it yields the whole image. Divide/rounding guards required.
- **Feather larger than the selection:** CS6 emits "No pixels are more than 50% selected" and may create an invisible selection. Replicate the warning and the accepted-but-invisible outcome.
- **Anti-aliasing is not available on the Rectangular Marquee** (hard straight edges); only the Elliptical Marquee exposes it. Do not show the checkbox otherwise.
- **Single Row/Column ignore Style** and produce a 1-px band regardless.
- **32-bit / 16-bit images:** selection coverage is independent of bit depth; keep the mask 8-bit while pixel edits run at native depth.
- **CMYK/Lab:** selection is colour-space independent; no conversion needed at selection time.
- **PSB/huge docs:** mask is document-sized; use tiled masks and only snapshot the changed bounding box for undo.
- **GPU unavailable:** overlay falls back to a CPU `QPainter` path; masks are CPU data regardless.
- **Undo/redo:** reselect (`Ctrl+Shift+D`) must survive undo of later commands per `ARCH-007`.
- **Snapping:** snap targets can change mid-drag; re-evaluate on each move rather than caching.

## Parity acceptance criteria

- Given a 1000×1000 px document, dragging a `200×100` rectangle then releasing produces a selection whose 50%-coverage bounding box is `200×100` within 1 px.
- Given `Shift`-drag, the resulting width and height differ by at most 1 px (square).
- Given `Alt`-drag from point `P`, the selection is centred on `P` within 1 px.
- Given an existing selection and `Add To Selection`, the combined mask equals `max(existing, new)` per pixel.
- Given `Subtract From Selection`, pixels in the new marquee lose coverage exactly.
- Given `Intersect With Selection`, only the overlap remains.
- Given Style = `Fixed Ratio` with `2 : 1`, every dragged rectangle has `W/H = 2 ± 0.01`.
- Given Style = `Fixed Size`, the selection is exactly the entered pixel size regardless of drag length.
- Given Feather = 250, the mask's outer transition spans approximately the feather distance and no pixel outside the primitive remains at 255.
- Given the Elliptical Marquee with Anti-aliased on, boundary coverage values include values strictly between 0 and 255; with it off, all values are 0 or 255.
- Given `View > Show > Selection Edges` off, the marching ants disappear but the mask and subsequent edits are unchanged.
- Given a 1-px document, a Single Row selection covers the whole document and does not error.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — primary CS6 Help corpus (Feb 2013). Established: marquee tool descriptions and single row/column behaviour; the four selection-mode buttons; feather 0–250; anti-aliasing availability (Elliptical Marquee, Lasso, Polygonal, Magnetic, Magic Wand) and the rule that it must be set before the selection; Style Normal/Fixed Ratio/Fixed Size and their value rules including non-pixel units; snapping to guides/grid/slices/document bounds; `View > Show > Selection Edges` vs `View > Extras`; selection modifier shortcut table (`Shift`/`Alt`/`Space`, 45° constraint for moving a selection, 1-px/10-px nudge); `Select > Transform Selection`; `Refine Edge` shortcut `Ctrl+Alt+R`; tool shortcut table (`M`, Shift-cycle, Alt-click hidden tools).

Not used in this pass (blocked or binary/undecodable):

- `helpx.adobe.com` (HTTP 403).
- `https://www.manuals.co.uk/adobe/photoshop-cs6/manual?p=235` and `https://www.photoshopessentials.com/...` — returned undecodable/non-English payloads or redirects to the reader; the same CS6 Help text was available directly in the archived PDF.

## Open questions

- **Anti-alias coverage filter.** The exact sub-pixel filter/averaging Photoshop uses for the Elliptical Marquee is not documented. Resolve with a pixel-level comparison against a CS6 reference selection.
- **Feather profile.** Photoshop's feather kernel is not a plain Gaussian in principle; exact falloff is unknown. Resolve with a profile measurement from a feathered CS6 selection.
- **Fixed Ratio / Fixed Size defaults.** The shipped default ratio and size values when the Style is first chosen are not in the fetched text. Resolve with a CS6 UI capture.
- **Snap target precedence.** Which of guides/grid/slices/document bounds wins when several are within snap distance is unspecified. Resolve from CS6 behaviour or a documented rule.
- **Marching-ants animation timing.** Dashed-line dash length, speed, and invert-on-dark behaviour are visual-detail open questions; resolve in `02-ui-ux/toolbox-and-options-bar.md`.
- **Feather edge for Subtract/Intersect.** Whether feather is applied to the primitive before or after masking to the canvas is unspecified; decide and record in `08-selection/selection-model.md`.
