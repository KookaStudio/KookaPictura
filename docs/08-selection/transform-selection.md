# Transform Selection

- **Spec ID:** `SEL-011`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `Changed` — the transform gesture set is unchanged from CS5, but CS6 added an **Interpolation** pop-up to the Free Transform options bar (see `03-tools/move-and-transform.md`), and the `Snap Vector Tools And Transforms To Pixel Grid` preference now lets a 90° rotation of an even-×-odd-pixel item avoid landing on a half-pixel position. The Free Transform options bar (shared with Transform Selection) is otherwise the CS5 bar.
- **Depends on:** `08-selection/selection-model.md`, `03-tools/move-and-transform.md` (`TOOL-001`), `ARCH-002` document-model, `ARCH-007` undo-history.

> All crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not.

## CS6 behavior

`Select > Transform Selection` puts a transform bounding box around the **selection border itself**, not around the pixels the selection encloses. Dragging handles reshapes the marching-ants outline; the underlying image is untouched. This is the command to use when the user wants to move or reshape a selection without moving its content.

The CS6 Help lists the transformable items as: a selection, an entire layer, multiple layers, a layer mask, a path, a vector shape, a vector mask, a selection border, or an alpha channel. Transforming a **selection border** is a distinct branch: make or load a selection, then choose `Select > Transform Selection`.

Because Transform Selection reuses the transform engine, it supports the same operations and modifiers as Free Transform:

- `Scale`, `Rotate`, `Skew`, `Distort`, `Perspective` and `Warp` gestures on the bounding box.
- A **reference point** (the fixed point around which the transform is performed), centred by default, movable via the options-bar reference-point locator or by dragging it in the canvas; it may lie outside the selection.
- Numeric `X`/`Y` position (with a relative-positioning toggle), `W`/`H` scale percentages (with an aspect-ratio link), a rotation angle, and `H`/`V` skew angles.
- Commit with `Enter`/`Return`, the options-bar commit button, or double-clicking inside the transform marquee; cancel with `Esc` or the cancel button.
- `Edit > Free Transform` may also start a transform of the selection border when the target has been selected with the Move tool and **Show Transform Controls** is enabled: choose the Move tool, then select **Show Transform Controls** in the options bar.

**Interaction with Free Transform.** `Edit > Free Transform` (`Ctrl+T`) on a document with an active selection transforms the **selected pixels** — Photoshop lifts the selected content into the transform and moves/resamples it, leaving a hole (or not, depending on the layer) behind — whereas `Select > Transform Selection` transforms only the **border**. The two commands must not be conflated: the Option bar is visually identical, but the target differs. The Help groups both under the transform keys table, "Keys for transforming selections, selection borders, and paths".

Transforming the border is a **raster** operation on the selection mask: the user-visible selection is a grayscale coverage mask (see `08-selection/selection-model.md`), so a rotation or perspective produces a resampled mask with partial coverage at the new edges, not a stored vector transform that can be re-edited. There is no "re-open last Transform Selection" command; each invocation starts fresh from the current mask. `Edit > Transform > Again` repeats the last *transform*, which can apply to a selection border as well.

A selection can be moved without transforming by dragging it with a selection tool (not the Move tool) or with the arrow keys — 1 px per press, 10 px with `Shift`. `Select > Transform Selection` is the way to scale/rotate/skew a border. A read-only convenience: right-clicking inside an active selection surfaces `Transform Selection` in the canvas context menu (community-reported).

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Select > Transform Selection` | Menu | — | Transforms the selection border only |
| Canvas context menu (inside selection) | Context menu | Right-click | Community-reported convenience path |
| Transform / Free Transform options bar | Widgets | — | Reference-point locator, X/Y, W/H, angle, H/V skew, interpolation pop-up, commit/cancel |
| `Edit > Free Transform` | Menu | `Ctrl+T` / `Cmd+T` | With an active selection transforms the **pixels**, not the border |
| `Edit > Transform > …` | Menu | — | Submenu; also transformable target selection border |
| `Edit > Transform > Again` | Menu | `Ctrl+Shift+T` | Repeats the last transform |
| Move tool options bar | Checkbox | — | `Show Transform Controls` enables border/selection transforms |
| `Select > Deselect` | Menu | `Ctrl+D` | Ends a selection (not a transform commit) |
| Commit transform | Key / button | `Enter`/`Return`, double-click inside | |
| Cancel transform | Key / button | `Esc`, `Ctrl+.` | Restores the item pixel-exact for pixel transforms; mask-exact for borders |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Reference point | enum (3×3) | centre | 9 locators + free in-canvas drag | May sit outside the item |
| X / Y position | float + unit | current | unbounded (document coords) | Relative-positioning toggle |
| W / H scale | percent | 100 % | > 0 % | Link-lock maintains aspect ratio |
| Rotation angle | degrees | 0 | unbounded | `Shift` constrains drag to 15° steps |
| H / V skew | degrees | 0 | unbounded | Free Transform `Ctrl+Shift`-drag |
| Interpolation | enum | Bicubic | Nearest Neighbor / Bilinear / Bicubic / Bicubic Smoother / Bicubic Sharper / Bicubic Automatic | CS6 options-bar control; irrelevant to a colourless selection mask but present on the shared bar |
| Warp Style / Bend / X,Y | enum + floats | None / 0 / 0 | preset-dependent | Shared with Free Transform |
| Transform value entry | numerics | — | — | Same widgets as `TOOL-001` |

## Algorithms & pipeline

### Target and coverage model

A selection border is stored as a `CoverageMask` (float coverage in `[0,1]` per pixel). Transform Selection applies a transform matrix to that mask and resamples it. Unlike a pixel-layer transform, there is no colour to interpolate, so the interpolation menu has no effect on the *values* — only the geometric mapping and the anti-aliasing of the resulting coverage matter.

### Matrix

Scale/rotate/skew map to a 3×3 affine matrix; `Distort` and `Perspective` require the projective (homography) form. As in `TOOL-001`, successive gestures accumulate into one matrix and are applied once on commit:

```text
state := identity (or homography)
on gesture: state := gesture_matrix * state
on commit:  resample the coverage mask once through state
```

The reference point is the fixed point of the mapping; moving it changes only the translation component.

### Resampling the mask

The coverage mask is resampled with the same kernel family used for pixels (`pictura_imageops::resample`) but single-channel. The marching-ants contour is re-extracted from the resampled mask at the 50 % coverage level. Whether Photoshop binarises before or after resampling is not documented; the Help's transform illustrations show a rotated selection border with a smooth curve, consistent with resampling a soft coverage mask and then contouring. Mark inferred.

### Inverse for undo (behavioral parity only)

For affine transforms the inverse matrix can restore the previous mask, but resampling round-trip error is possible at fractionally-aligned edges. To be lossless, the undo record should retain the pre-transform coverage mask (or its tile snapshot) rather than re-deriving it from the inverse. Mark as a design decision, not an Adobe behaviour.

## Rust module mapping

- `pictura_selection::transform` — `transform_selection(mask: &CoverageMask, m: &Matrix, interpolation: Interpolation) -> CoverageMask`.
- `pictura_selection::contour` — `contour(mask, threshold = 0.5) -> Vec<Polyline>` for marching-ants display.
- `pictura_transform::Matrix` — shared affine/homography type from `TOOL-001`; no separate selection matrix type.
- `pictura_transform::ReferencePoint` and `Interpolation` — reused from `TOOL-001` so the options bar binds to one widget model.
- `pictura_tools::transform_session` — `TransformSession { target: SelectionBorder | Layer | Mask | Path, matrix, reference, interpolation }` shared by Free Transform and Transform Selection.

Data crossing the boundary: the live matrix is marshalled to Qt as `[f64; 6]` (affine) or `[f64; 9]` (homography); the contour is marshalled as polylines for the vector overlay. The coverage mask itself stays in the core.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `SelectionTransformController` | `QObject` | Owns the transform session when target is a selection border; start/commit/cancel |
| `TransformOptionsBar` | `QWidget` | Reused verbatim from `TOOL-001` (reference point, X/Y, W/H, angle, skew, interpolation, commit/cancel) |
| `SelectionTransformOverlay` | `QGraphicsItem` | Bounding box, 8 handles, rotation zone, reference point, live matrix preview |
| `MarchingAntsItem` | `QGraphicsItem` | Re-contoured selection border, re-drawn from the live matrix |
| `TransformSessionModel` | `QAbstractItemModel` / value type | Exposes target kind so the menu/entry points can enable/disable correctly |

Widgets over QML: consistent with `ARCH-003`; the overlay is a vector-layer `QGraphicsItem` above the GPU-composited canvas.

## Data-model impact

- The selection is document state; Transform Selection replaces the current `CoverageMask` with the resampled one. Underlying layer pixels are untouched.
- **Undo granularity:** one history state per committed transform. Proposal: store the pre-transform mask (or tiled snapshot) for lossless undo; store the matrix only for vector targets (paths/vector masks), which are re-derivable.
- **Serialization:** a transformed selection is not stored as a transform; to persist it the user runs `Select > Save Selection` (see `save-and-load-selections.md`), which writes the mask to an alpha channel. PSD has no "selection transform" record.
- **Tool state** (reference point, interpolation) persists in preferences/tool presets, never in the document.
- The shared transform session means `TOOL-001` and this spec must agree on the commit/cancel contract; a session cannot be active for two targets at once.

## Edge cases

- **No selection:** `Select > Transform Selection` is unavailable/disabled.
- **Empty selection after transform:** scaling a 1-px selection down or rotating it can produce an all-zero mask; guard against a zero-area result and keep a recoverable previous state.
- **Reference point outside bounds:** allowed, and must not be clamped.
- **Soft / feathered selection:** transforming resamples the soft coverage; the 50 % contour can shift more than for a hard mask — tolerance must account for this.
- **Selection partly off-canvas:** dragging the border beyond the canvas is explicitly allowed for a move; transform operates in document space and may move coverage off-canvas.
- **Bitmap / Indexed mode:** no selection coverage mask in the usual sense; behaviour must be checked against CS6.
- **32-bit / CMYK / Lab:** geometric only; colour space is irrelevant, but the mask is still per-document.
- **PSB / huge docs:** resampling a 300,000-px coverage mask must be tiled; undo snapshot must be tile-backed.
- **GPU unavailable:** overlay and marching ants are cheap vector/CPU work and must not require the GPU.
- **Undo/redo:** must restore the exact prior coverage without cumulative resample loss.
- **Marching-ants fidelity:** at high zoom, contouring a resampled soft mask must place the ants where CS6 does (sub-pixel) — open question (shared with `TOOL-001`).

## Parity acceptance criteria

- Given an active selection and `Select > Transform Selection`, dragging a corner handle with `Shift` scales the border proportionally around the reference point; the committed border's bounding box matches the drag within 1 px.
- Given `Select > Transform Selection`, the underlying layer pixels are bit-identical before and after the command (only the border changes).
- Given `Edit > Free Transform` with an active selection, the selected **pixels** are transformed, not the border; the two commands produce observably different results.
- Given a reference point moved to the top-left locator, a 90° rotation maps the border consistently with CS6 (same corner fixed) within 1 px.
- Given `Esc` during Transform Selection, the selection border is restored to its pre-command shape.
- Given a rectangular selection, numeric `W = 200 %` doubles the border's width and height within 1 px.
- Given `Edit > Transform > Again`, the last selection-border transform is repeated identically.
- On a document with no selection, `Select > Transform Selection` is disabled.
- Marching ants extracted from the transformed mask sit on the 50 % coverage contour within 0.5 px.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — primary CS6 Help corpus (curl → `/tmp`, `pdftotext`). Established: transforming a selection border by making or loading a selection and choosing `Select > Transform Selection`; transformable-items list including selection border and alpha channel; reference-point behaviour (default centre, movable, may lie outside); the full `Edit > Transform` submenu semantics; Free Transform gesture/modifier matrix; Move tool `Show Transform Controls` for selection/selection-border transforms; commit/cancel and duplicate/again shortcuts; "Keys for transforming selections, selection borders, and paths"; Free Transform interpolation preference behaviour; CS6 What's New > Transform notes (Smart Object icon, vector-curve dragging, 90° even-×-odd pixel behaviour, Ignore Rotation Metadata preference).
- `https://search.brave.com/search?q=Photoshop+CS6+%22Transform+Selection%22+Select+menu` — search results page; confirms `Select > Transform Selection` as the border transform and the distinction from `Edit > Transform`.

Consulted as search-result snippets only (not individually fetched; community-reported):

- `https://photoshoptrainingchannel.com/tips/transform-selections/` — bounding box around the selection after `Select > Transform Selection`.
- `https://tuyettac.org/menu-select-trong-photoshop-cs6.html` — CS6 context menu offers Transform Selection inside a selection.

Not used in this pass:

- `helpx.adobe.com` (HTTP 403).

## Open questions

- **Coverage binarisation ordering.** Does Photoshop threshold the selection before resampling (hard edges) or resample the soft coverage and contour afterwards? Resolve with a rotated-soft-selection reference from CS6.
- **Marching-ants placement after transform.** Sub-pixel contour rules are unspecified, matching the open question recorded in `TOOL-001`. Resolve by comparing transformed selections against CS6.
- **`Transform Selection` availability by colour mode.** Behaviour on Bitmap/Indexed/Multichannel is not in the fetched text. Resolve from a CS6 UI capture.
- **Whether the transformed border can be re-edited.** Photoshop appears to discard the transform on commit (raster mask); retaining it would be a non-parity enhancement. Decide in `08-selection/selection-model.md`.
- **Right-click context-menu path.** Only community-reported, not in the CS6 Help PDF. Confirm against a CS6 build before shipping the context-menu item.
