# Lasso Selection Tools

- **Spec ID:** `TOOL-003`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `No` — the Lasso, Polygonal Lasso, and Magnetic Lasso tools and their options are unchanged from CS5.
- **Depends on:** `ARCH-002` document-model, `ARCH-007` undo-history, `TOOL-002`/`TOOL-004` (sibling selection tools), `08-selection/selection-model.md`, `08-selection/refine-edge.md`.

> All crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not.

## CS6 behavior

Three freehand-style selection tools share the `L` shortcut:

- **Lasso** — drag to draw an arbitrary freehand selection border. `Alt`(`Option`)-clicking switches between freehand and straight-edged segments while drawing; `Delete` erases recently drawn straight segments. Releasing the mouse closes the border.
- **Polygonal Lasso** — click to set straight segments. `Shift` constrains a segment to a multiple of 45°; `Alt`-drag draws a freehand segment; `Delete` erases recently drawn straight segments. Close by clicking the start point (a closed circle appears), or double-click / `Ctrl`(`Cmd`)-click when not over the start.
- **Magnetic Lasso** — the border snaps to defined edges as the pointer moves. `Alt`-drag temporarily switches to the Lasso, `Alt`-click to the Polygonal Lasso, `Delete` erases recently drawn segments and fastening points, `Enter`/`Esc` (`Ctrl+.` on Windows) apply/cancel. Double-click or `Enter` closes with a magnetic segment; `Alt`-double-click closes with a straight segment. The Magnetic Lasso is **not available on 32-bits-per-channel images**.

All three expose the four selection modes (**New**, **Add To**, **Subtract From**, **Intersect With**) and **Feather** (0–250 px); the Lasso and Polygonal Lasso also expose **Anti-aliased** (the CS6 Help lists anti-aliasing as available for all three lasso tools). After a selection is made, `Refine Edge` (`Ctrl+Alt+R`) refines the boundary. The Lasso and Polygonal Lasso are also used inside the Magnetic Lasso's temporary-switch workflow.

Magnetic Lasso-specific options:
- **Width** — detection distance from the pointer, in pixels; the lasso seeks edges only within this band. `]` / `[` increase/decrease the width by 1 px while the tool is selected but idle; `Caps Lock` switches the pointer to a precision cross showing the width.
- **Contrast** (Edge Contrast) — sensitivity to edges, 1%–100%. Higher values detect only sharply contrasting edges; lower values detect lower-contrast edges.
- **Frequency** — rate at which the lasso sets fastening points, 0–100. Higher values anchor the border faster.
- **Stylus Pressure** — when on, greater pen pressure decreases the edge width (tablet only).

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Tools panel | Tool | `L` | Lasso is the default `L` tool |
| Tools panel | Hidden tools | `Shift+L` | Cycles Lasso / Polygonal Lasso / Magnetic Lasso |
| Options bar | Buttons | — | New / Add / Subtract / Intersect (Magic Wand/docs; shared with all selection tools) |
| Options bar | Spin box | — | Feather (0–250 px) |
| Options bar | Checkbox | — | Anti-aliased |
| Options bar | Spin box | — | Magnetic only: Width (px) |
| Options bar | Spin box | — | Magnetic only: Contrast (%) |
| Options bar | Spin box | — | Magnetic only: Frequency (0–100) |
| Options bar | Checkbox | — | Magnetic only: Stylus Pressure |
| Options bar | Button | `Ctrl+Alt+R` | Refine Edge |
| Canvas | Modifier | `Alt`-drag | Magnetic → Lasso temporarily |
| Canvas | Modifier | `Alt`-click | Magnetic → Polygonal Lasso temporarily |
| Canvas | Modifier | `Alt`-click | Lasso: toggle straight/freehand segments |
| Canvas | Modifier | `Shift` | Polygonal: constrain segment to 45° |
| Canvas | Key | `Delete` | Erase recently drawn segments/fastening points |
| Canvas | Key | `Enter` / `Esc` / `Ctrl+.` | Magnetic: apply / cancel |
| Canvas | Gesture | Double-click | Close magnetic border |
| Canvas | Modifier | `Alt`-double-click | Close magnetic border with a straight segment |
| Canvas | Key | `[` / `]` | Magnetic: decrement/increment detection width by 1 px |
| Canvas | Key | `Caps Lock` | Magnetic: show detection-width pointer |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Selection mode | enum | New | New / Add / Subtract / Intersect | Shared with all selection tools |
| Feather | int (px) | 0 | 0–250 | Set before selecting |
| Anti-aliased | bool | on | on / off | Set before selecting |
| Width (Magnetic) | int (px) | 10 | 1–256 | Default community-reported (see Open questions); `[`/`]` step 1 |
| Contrast (Magnetic) | int (%) | 10% | 1–100 | Edge-contrast threshold |
| Frequency (Magnetic) | int | 57 | 0–100 | Fastening-point rate; default community-reported |
| Stylus Pressure (Magnetic) | bool | off | on / off | Tablet only; more pressure = narrower edge width |
| Refine Edge radius | int (px) | 0 | 0–~1000 (dialog) | `08-selection/refine-edge.md` |
| Refine Edge smooth | int | 0 | 0–100 | `08-selection/refine-edge.md` |
| Refine Edge contrast | int (%) | 0 | 0–100 | `08-selection/refine-edge.md` |
| Refine Edge shift edge | int (%) | 0 | −100–100 | `08-selection/refine-edge.md` |

## Algorithms & pipeline

### Common representation

All three tools produce a closed polygon (freehand points or clicked vertices) that is rasterised into the same coverage mask as `TOOL-002`, then combined with the existing selection via `New`/`Add`/`Subtract`/`Intersect`. Anti-aliasing computes partial coverage along the polygon boundary; feather blurs the resulting mask before combining. Undo is one command per closed border.

### Lasso and Polygonal Lasso

- **Lasso:** sample the pointer path into a polyline (optionally simplified with a distance/angle tolerance) while dragging; `Alt`-click inserts a vertex and switches to straight segments; `Delete` pops the last straight segment. On release, close the polyline and rasterise.
- **Polygonal Lasso:** vertices come from clicks; a rubber-band segment previews the next edge; `Shift` snaps the next vertex direction to 45° increments; `Alt`-drag freehand-inserts points. Closing rules are as described above.

Polygon rasterisation with anti-aliasing is a standard scanline-with-coverage or supersampled-fill problem.

### Magnetic Lasso (edge-snapping)

Magnetic Lasso is an interactive edge follower. The CS6 Help describes the observable behaviour: within the **Width** band the active segment snaps to the strongest edge, **Contrast** sets the edge threshold, and **Frequency** controls how often **fastening points** are committed.

An implementation-agnostic model that matches the description:

```text
state: current_point, pending_points[], last_fastening_point
for each pointer move:
    candidates = sample an arc of radius Width around current_point
    score(p)  = edge_strength(p)            # gradient magnitude of the document
    edge      = argmax score(p) subject to score >= contrast_threshold(Contrast)
    extend the active segment through edge
    if arc_angle_since(last_fastening_point) >= 360 / Frequency_scale:
        commit fastening point; last_fastening_point = edge
on click:          add a manual fastening point at pointer
on close:          rasterise the path through all fastening points
```

The publicly studied relative is the **Intelligent Scissors / "live wire"** boundary, which finds a globally minimal-cost path where the cost combines gradient magnitude, gradient direction, and Laplacian zero-crossings (Mortensen & Barrett). Photoshop's Magnetic Lasso is a lighter, greedy/local variant; the exact cost function and the mapping from `Contrast`/`Frequency` to thresholds are closed. Mark as *behavioral parity only, algorithm TBD*.

Recommended proposal: a local greedy snap for interactive feel, with a bounded Dijkstra/live-wire refinement between fastening points to avoid drifting across weak edges. Use a precomputed gradient-magnitude pyramid so the Width arc is cheap at any zoom.

Edge direction matters: an edge oriented perpendicular to the tracing direction should score higher than one parallel to it, matching the live-wire cost's direction term.

## Rust module mapping

- `pictura_selection::lasso` — `LassoPath { points: Vec<Point2>, closed: bool }`; `simplify(tolerance)`.
- `pictura_selection::polygon` — `PolygonMarquee`, vertex insertion/removal, 45° constraint.
- `pictura_selection::magnetic` — `MagneticLassoSettings { width, contrast, frequency, stylus_pressure }`, `trace_edge(doc_edges: &EdgeMap, start: Point2, settings) -> LassoPath`.
- `pictura_selection::edges` — `EdgeMap` (gradient magnitude + orientation), cached per viewport and invalidated on document change.
- `pictura_selection::rasterize` — shared polygon coverage rasteriser.
- `pictura_selection::feather` — shared feather.
- `pictura_tools::lasso` — `LassoTool { kind: Lasso | Polygonal | Magnetic, settings }` interaction state machine.

Data crossing the boundary: `Point2`, `MagneticLassoSettings`, `SelectionMode`, contour polylines for the overlay. The interactive path stays in the core; Qt receives the overlay polyline and cursor state.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `LassoOptionsBar` | `QWidget` | Mode buttons, feather, anti-alias |
| `MagneticLassoOptionsBar` | `LassoOptionsBar` | Adds Width/Contrast/Frequency spin boxes and Stylus Pressure check |
| `SelectModeButtonGroup` | `QButtonGroup` | Shared four-mode control |
| `LassoPathItem` | `QGraphicsPathItem` | Live path preview (freehand, polygonal, magnetic) |
| `FasteningPointItem` | `QGraphicsItem` | Magnetic fastening-point markers |
| `MagneticCursorItem` | `QGraphicsItem` | Detection-width ring / precision cross when `Caps Lock` |
| `RefineEdgeDialog` | `QDialog` | Shared with `08-selection/refine-edge.md` |

The detection-width ring is a cheap vector overlay; the per-move edge search runs in the Rust core and must not block the GUI thread beyond one frame. Follow the threading rule in `01-architecture/qt6-ui-design.md`: the model/overlay is GUI-thread-only, background edge work queues results back.

## Data-model impact

- One undo command per closed border, shape `SelectionChange { before, after, region }` (bounding-box snapshot), same as `TOOL-002`.
- Fastening points, in-progress vertices, and the current path are **interaction state**, not document state; they are never serialised.
- Magnetic settings (Width/Contrast/Frequency/Stylus) are tool/preset state, persisted in the preferences/tool-preset store, not in PSD.
- The edge map is a derived cache keyed by layer visibility/contents; invalidate on any pixel change.

## Edge cases

- **32-bpc images:** Magnetic Lasso is unavailable (CS6 states this explicitly). The Lasso and Polygonal Lasso still work. Disable the tool/options with the same affordance CS6 uses.
- **Weak/no edges:** the magnetic follower must degrade gracefully to a straight/loose segment rather than jumping to a distant strong edge; clamp search radius to Width.
- **Self-intersecting paths:** define a fill rule (non-zero vs even-odd) and apply it consistently; CS6's exact rule is unspecified — see Open questions.
- **Open path auto-close:** releasing/closing adds a straight segment from the last point to the first; the preview must show this.
- **Very long freehand paths:** simplify/subsample to bound memory and rasterisation cost.
- **Stylus pressure:** when enabled, pressure scales Width inversely; handle pressure-less devices by ignoring the option.
- **Spot colours / CMYK / Lab:** edge detection runs on a luminance-like channel; choose the channel consistently (proposal: composite luminance) and document it.
- **Huge PSB:** edge map and mask are tiled; only the dirty rect is recomputed and snapshotted for undo.
- **GPU unavailable:** edge map is CPU data; overlays run through the CPU `QPainter` fallback path.
- **Undo/redo:** cancelling a magnetic trace (`Esc`) must leave the prior selection untouched.

## Parity acceptance criteria

- Given a high-contrast rectangle on a uniform background, tracing roughly inside the Width band with the Magnetic Lasso produces an edge-following selection whose boundary stays within 2 px of the true edge along a straight run.
- Given Contrast = 100%, a low-contrast edge is not snapped; given Contrast = 1%, it is.
- Given Frequency = 0, very few fastening points are committed; given Frequency = 100, fastening points are visibly dense (count differs by at least 5× over the same trace).
- Given `]`, the detection width increases by exactly 1 px; given `[`, it decreases by exactly 1 px.
- Given `Alt`-drag with the Magnetic Lasso, the trace becomes freehand (Lasso) for the duration of the drag and reverts on release.
- Given `Alt`-click with the Magnetic Lasso, subsequent segments are straight (Polygonal) until the click sequence ends.
- Given `Delete`, the most recent fastening point/segment is removed without disturbing earlier ones.
- Given a Polygonal Lasso segment with `Shift` held, the segment angle is a multiple of 45° within 0.5°.
- Given `Esc` during any lasso trace, the document selection is bit-identical to its pre-trace state.
- Given a 32-bpc document, the Magnetic Lasso cannot be activated and the other two lasso tools work normally.
- Given a closed lasso path, the 50%-coverage contour of the produced mask matches the drawn polygon within 1 px (anti-aliased on).

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — primary CS6 Help corpus (Feb 2013). Established: Lasso/Polygonal/Magnetic workflows and modifiers; Magnetic Lasso unavailable on 32-bpc images; Width/Contrast/Frequency/Stylus Pressure descriptions and ranges (Contrast 1%–100%, Frequency 0–100); `[`/`]` width stepping; `Caps Lock` pointer; fastening points; temporary tool switches (`Alt`-drag → Lasso, `Alt`-click → Polygonal); `Delete` behaviour; `Enter`/`Esc`/`Ctrl+.`; closing rules; anti-aliasing availability for all lasso tools; feather 0–250; tool shortcut table (`L`, Shift-cycle); selection modifier shortcut table.
- `https://www.underwaterphotography.com/PhotoShop/PhotoShop/1_9_2_4.html` — mirrored older Adobe Help text for "Setting options for the lasso, polygonal lasso, and magnetic lasso tools"; independently confirms Width, Edge Contrast, Frequency ranges and the Stylus Pressure wording.

Consulted as search-result snippets only (not individually fetched; community-reported):

- Multiple lasso tutorials surfaced via SearXNG — Magnetic Lasso shipped defaults Width 10 px, Edge Contrast 10%, Frequency 57.

Not used in this pass:

- `helpx.adobe.com` (HTTP 403).
- The `glensmith.co.uk` and `expertphotography.com` articles returned undecodable/binary payloads and were not relied upon.

## Open questions

- **Magnetic Lasso default values.** Width 10 / Contrast 10% / Frequency 57 are community-reported and not stated in the fetched CS6 Help text. Resolve with a CS6 UI capture and record as the shipped defaults.
- **Magnetic Lasso cost function.** The gradient/direction/zero-crossing weighting, edge threshold mapping, and the exact relation of Frequency to arc distance are closed. Resolve with a behavioral study and a reference implementation.
- **Polygon fill rule.** Whether self-intersecting lasso paths use non-zero or even-odd filling is unspecified. Resolve with a CS6 comparison.
- **Edge channel.** Which channel(s) drive edge detection on colour documents is not documented. Resolve with a controlled multi-channel test.
- **Path simplification.** Whether (and how) Photoshop simplifies a freehand lasso polyline before rasterising is unknown; resolve only if the pixel comparison demands it.
- **Stylus Pressure curve.** The exact pressure-to-width mapping for the Magnetic Lasso is unspecified. Resolve with a tablet capture if tablet parity is required.
- **Feather profile** (shared with `TOOL-002`). Exact falloff unknown.
