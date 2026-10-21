# Paths Panel

- **Spec ID:** `PAN-003`
- **Status:** `Draft`
- **Parity tier:** `Core`.
- **New in CS6:** `No` — the Paths panel itself is CS5-era. CS6 continues the CS5 model (saved paths, work path, vector-mask path) and adds no panel chrome; path features new to CS6 elsewhere (e.g. `3D > Make Work Path from 3D Layer`, path-aware shape-layer behavior) are referenced, not owned here.
- **Depends on:** `SEL-014` paths-and-vector-selection, `TOOL-052` pen-and-path-tools, `TOOL-053` path-drawing behavior, `TOOL-054`/`TOOL-055` path-selection-tools, `TOOL-044` quick-mask-tool, `LAY-005` vector-masks-and-clipping-masks, `CLR-003` swatches-and-libraries, `ARCH-008` document-model, `ARCH-009` undo-history, `10-workflow-io/export-formats.md`, `10-workflow-io/printing.md`.

> Crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Path construction/editing semantics are owned by
> `TOOL-052`/`TOOL-053` and `SEL-014`; this file specifies the **panel widget**
> and the operations invoked from it.

## CS6 behavior

The **Paths panel** (`Window > Paths`) lists each saved path by name with a thumbnail, together with the current work path and the current vector mask. Panel anatomy: **A** saved path, **B** temporary work path, **C** vector-mask path (only listed while the owning shape layer is selected). Thumbnails can be turned off (`Panel Options > None`) to improve performance.

**Default placement (CS6 Essentials workspace).** The panel is tabbed with **Layers** and **Channels** in the bottom-right group; Layers is the default-active tab. (Source: Photoshop Essentials, *Managing Panels In Photoshop CS6*.)

A path is a resolution-independent mathematical outline that can be converted to a selection, filled, or stroked with colour; it can also be designated a **clipping path** used to make part of an image transparent when exporting to a page-layout or vector-editing application (EPS/TIFF; see `10-workflow-io/`). A **work path** is temporary: it is replaced when a new work path is started unless saved, and it is lost on deselect if unsaved.

### Selecting and managing

Only **one path can be selected at a time**; clicking a path name selects it and a path must be selected before it is shown in the image. Clicking a blank area or pressing `Esc` deselects. Path stacking order is changed by dragging the row; the heavy black line marks the drop point. Vector masks and work paths cannot be reordered. Saved paths and work paths can be renamed by double-clicking the name.

Operations available from the panel bottom buttons and the panel menu:

- **Create New Path** — creates and selects a new empty path. Without a name, the button creates it directly; to name it, ensure no work path is selected, then `Alt`/`Option`-click the button or choose `New Path` from the menu.
- **New work path** — drawn with a shape or pen tool in Path mode. Path area options decide how overlapping components combine: `Add To Path Area`, `Subtract From Path Area`, `Intersect Path Areas`, `Exclude Overlapping Path Areas` (during drawing, `Shift` = add, `Alt`/`Option` = subtract).
- **Save a work path** — drag the work path name onto the `New Path` button (saves without renaming) or choose `Save Path` from the menu (names/renames).
- **Duplicate Path** — drag a path to the `New Path` button (copy, no rename) or `Alt`/`Option`-drag (copy and rename), or `Duplicate Path` from the menu.
- **Delete Path** — drag to the Delete icon, `Delete Path` from the menu, or the Delete button (with confirmation). `Alt`/`Option`-click the Delete icon deletes without confirmation.
- **Copy/paste paths** — `Edit > Copy` in the source document then paste in the destination (`SEL-014`); paths can also be exported/imported via file formats.

### Paths ↔ selections

- **Fill Path** — fills with pixels on the active layer. Click the `Fill Path` button to use current settings; `Alt`/`Option`-click the button, `Alt`/`Option`-drag the path to the button, or `Fill Path` from the menu to open the dialog (Use, Opacity, Mode — including a `Clear` mode to erase to transparency, `Preserve Transparency`, `Feather Radius`, `Anti-aliased`). Filling requires an active standard/background layer; it is refused when a mask, text, fill, adjustment, or Smart Object layer is active. For a path component the menu command reads `Fill Subpath`.
- **Stroke Path** — paints the path border using the current painting-tool settings/brush. Click the `Stroke Path` button (each click builds up opacity); `Alt`/`Option`-click/drag or the menu opens the dialog (choose tool, `Simulate Pressure`). Same active-layer restriction as fill. Menu reads `Stroke Subpath` for a component.
- **Make Selection** — `Ctrl`/`Cmd`-click the path thumbnail, click the `Load Path as a Selection` button, or `Alt`/`Option`-click/drag the button or choose `Make Selection` from the menu to open the dialog (`Feather Radius`, `Anti-aliased`, Operations: `New Selection`, `Add To Selection`, `Subtract From Selection`, `Intersect With Selection`). Combine modifiers on the path thumbnail mirror the Layers/Channels convention.
- **Make Work Path** — converts the current selection to a path. Click the `Make Work Path` button to use the current tolerance; `Alt`/`Option`-click or the menu opens the dialog with a `Tolerance` of **0.5–10 px** (higher tolerance = fewer anchors, smoother path). Feathering on the selection is discarded. The resulting work path appears at the bottom of the panel.

### Vector masks

A shape layer's path appears as a **vector mask** in the Paths panel, but only while the parent layer is selected in the Layers panel. Vector masks are linked to their parent layer and cannot be reordered; the panel is a second entry point for selecting the vector-mask path and editing it, alongside the Layers panel thumbnail (`LAY-005`).

### Clipping path

A saved path can be designated a **clipping path** so that part of the image is transparent when exported to a page-layout/vector application. The `Clipping Path` dialog (Paths panel menu) exposes two fields:

- **Path** — choose which saved path to designate.
- **Flatness** — leave blank to use the printer's default; otherwise `0.2–100` (device pixels). Lower values approximate the curve with more, shorter line segments (more accurate). Adobe recommends `8–10` for high-resolution printing (1200–2400 dpi) and `1–3` for low-resolution printing (300–600 dpi).

Because paths are vector outlines with hard edges, a clipping path cannot preserve the softness of a feathered edge (a shadow, for example). The file must be saved as Photoshop EPS, DCS or PDF for a PostScript printer, or TIFF for a non-PostScript workflow. The exact PSD/TIFF serialisation of the designation is documented in Adobe's file-format spec (`ARCH-008`).

## UI surface

| Location | Type | Shortcut (Win / Mac) | Notes |
|---|---|---|---|
| `Window > Paths` | Menu → panel | — | Display the panel. |
| Panel row | Select path | click name | One path at a time; required before the path shows. |
| Panel blank area / `Esc` | Deselect | — | Clears the active path. |
| Panel menu | Menu | — | `New Path`, `Duplicate Path`, `Delete Path`, `Save Path`, `Make Work Path`, `Make Selection`, `Fill Path`, `Stroke Path`, `Clipping Path`, `Panel Options`. |
| New Path button | Button | `Alt`/`Option`-click | Name on create. |
| Fill Path button | Button | `Alt`/`Option`-click or drag | Options dialog. |
| Stroke Path button | Button | `Alt`/`Option`-click or drag | Tool + `Simulate Pressure`. |
| Load Path as Selection button | Button | `Alt`/`Option`-click/drag | Make Selection dialog. |
| Make Work Path button | Button | `Alt`/`Option`-click | Tolerance dialog. |
| Path thumbnail | Load selection | `Ctrl`/`Cmd`-click | `+Shift` add, `+Alt`/`+Option` subtract, `+Shift+Alt` intersect. |
| `Ctrl+Shift+H` / `Cmd+Shift+H` | Hide path | — | Hides the path overlay. |
| Delete icon | Button | `Alt`/`Option`-click | Delete without confirmation. |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Path thumbnail | enum | medium | None / small / medium / large | `Panel Options`. |
| Make Work Path tolerance | px | 2.0 | 0.5–10 | Higher = smoother, fewer anchors. Default 2 px is community/CS6-era (Help says "use the default"). |
| Fill Use | enum | Foreground Color | foreground / background / colour / pattern / history etc. | Mirrors `Edit > Fill`. |
| Fill Opacity | percent | 100 | 0–100 | 100 = opaque. |
| Fill Mode | enum | Normal | blend modes + `Clear` | `Clear` erases to transparency; needs a non-background layer. |
| Preserve Transparency | bool | off | on / off | Limits fill to existing pixels. |
| Fill/Selection Feather Radius | px | 0 | ≥ 0 | Applied before anti-aliasing. |
| Anti-aliased | bool | on | on / off | Selection conversion edge softening. |
| Make Selection Operation | enum | New Selection | New / Add / Subtract / Intersect | With an existing selection. |
| Stroke tool | enum | current painting tool | any painting/editing tool + brush | `Simulate Pressure` optional. |
| Clipping path | bool | off | — | Designates the selected saved path. |
| Clipping Path flatness | px | blank (printer default) | 0.2–100 | 8–10 for 1200–2400 dpi; 1–3 for 300–600 dpi. |
| Path area option | enum | Add | Add / Subtract / Intersect / Exclude | While drawing components. |

## Algorithms & pipeline

The panel is a **view + controller** over the document's path list; rasterisation and selection conversion reuse the shared engines.

1. **Projection.** Emit rows for saved paths, the single work path, and the current vector-mask path (only when its parent layer is selected), each with `{ id, name, kind, thumbnail, selected }`.
2. **Path model.** Paths are Bézier outlines (anchor points + control handles) with fill rules; the panel never edits geometry directly — it delegates to `TOOL-052`/`TOOL-053`.
3. **Thumbnail.** Rasterise a small stroke of the path; cache and invalidate on geometry change; `None` skips.
4. **Selection conversion.** `Make Selection` rasterises the path under the current tolerance/feather/anti-alias settings into the shared selection bitmap (`SEL-001`, `SEL-012`); the reverse (`Make Work Path`) fits a path to a selection, discarding feather.
5. **Fill / Stroke.** Fill rasterises the path with the fill engine (`Clear` maps to erase-to-transparent); stroke replays the current brush along the path. Both write to the active layer only and are refused on incompatible layer kinds.
6. **Work-path lifecycle.** Drawing a new work path replaces an unsaved one; saving promotes it to a named saved path; deselect does not auto-save.
7. **Commands.** Create/save/duplicate/delete/rename/reorder, fill, stroke, make-selection, make-work-path and clipping-path designation are commands with undo records (`ARCH-009`).

## Rust module mapping

Design proposal; paths already belong to `ARCH-008` and the tool specs.

- `pictura_core::document::path::{Path, SubPath, Anchor}` — Bézier data, fill rule, kind: `Saved | Work | VectorMask { layer: NodeId }`.
- `pictura_core::document::PathList` — ordered paths, one-work-path invariant, clipping-path designation flag.
- `pictura_core::document::path_ops` — `NewPath`, `SaveWorkPath`, `DuplicatePath`, `DeletePath`, `RenamePath`, `ReorderPath`, `SetClippingPath`, `FillPath`, `StrokePath`.
- `pictura_core::vector` — fill/stroke rasterisation and path↔selection conversion (shared with `TOOL-052`, `SEL-014`).
- `pictura_ui_bridge::PathsViewModel` — projected rows + thumbnail handles; selection target for the image view.
- `pictura_ui_bridge::PathEditTarget` — the active path handle painted/pointed by the pen/path tools.

Crossing types: `PathId`, `NodeId`, `FillSpec`, `StrokeSpec`, `SelectionId`, `Tolerance`. No Qt types in `pictura_core`.

## Qt6 component mapping

Widgets (consistent with `ARCH-003`).

| Proposal | Base | Responsibility |
|---|---|---|
| `PathsPanel` | `QDockWidget` | Host; list, bottom button strip, panel menu. |
| `PathsModel` | `QAbstractItemModel` | Path rows; roles for kind, name, thumbnail, clipping designation, vector-mask link. |
| `PathRowDelegate` | `QStyledItemDelegate` | Thumbnail, name, work-path marker, vector-mask marker, clipping indicator. |
| `PathThumbnailCache` | `QPixmapCache`-backed helper | Stroke-preview generation/invalidation. |
| `FillPathDialog` / `StrokePathDialog` / `MakeSelectionDialog` / `MakeWorkPathDialog` | `QDialog` | Option dialogs described above. |
| `ClippingPathDialog` | `QDialog` | Path selector + `Flatness` (blank / 0.2–100). |
| `NewPathDialog` / `DuplicatePathDialog` | `QDialog` | Name entries. |
| `PathsPanelMenu` | `QMenu` | Panel-menu actions, enabled per state (e.g. no fill on a vector-layer target). |

## Data-model impact

- **Paths already exist** in `ARCH-008` as vector data; the panel adds the `clipping_path` designation and the single work-path invariant.
- **Vector masks** are paths linked to a `Node`; the same `Path` object is presented in the Layers panel, the Paths panel and the image view — one source of truth (`LAY-005`).
- **Persistent** in PSD/TIFF; the Help notes platform/format support for path persistence lives in `01-architecture/file-formats.md`. Clipping path designation must survive PSD round-trip (exact keys unresolved).
- **Undo:** geometry edits record anchor diffs (`TOOL-053`); create/save/delete/reorder and clipping designation are structural commands; fill/stroke record pixel tiles on the target layer.
- **Clipping path** is document metadata attached to a path, not a new document node.

## Edge cases

- **Unsaved work path:** starting a new path replaces it without warning (Help); the panel must make this visible (work-path styling) rather than silently lose data this spec can prevent by prompting.
- **No work path:** `Save Path` / drag-to-New-Path must be disabled; `New Path` with a work path selected must be refused (Help requires no work path selected to name a new path).
- **Vector-mask path visibility:** listing depends on the parent layer's selection; the panel must update when the Layers-panel selection changes.
- **Multiple paths:** only one active at a time; the image overlay must follow the active path.
- **Fill/Stroke on incompatible layers:** refused on mask, text, fill, adjustment and Smart Object layers; `Clear` fill refused on the Background.
- **Make Work Path on a feathered/complex selection:** feather is discarded and the path simplifies per tolerance; both are expected losses, not bugs.
- **Empty/1-px documents:** zero-area paths and thumbnails must not crash.
- **CMYK/Lab/32-bit:** fill/stroke run in the document colour space; `Clear` and `Preserve Transparency` follow layer rules.
- **Huge PSB:** thumbnail and fill rasterisation respect the tile-cache cap (`ARCH-006`).
- **GPU unavailable:** CPU fill/stroke and thumbnails.
- **Undo/redo:** deleting the active path and undoing restores geometry, order and clipping designation.

## Parity acceptance criteria

1. Given a shape layer selected, its vector mask appears in the Paths panel; selecting another layer removes it from the list.
2. Given a work path, drag-to-New-Path saves it under the current name; `Save Path` renames it.
3. Given a selection, `Make Work Path` with tolerance `T` in 0.5–10 produces a path that re-loads as a selection within the selection-engine tolerance, with feather discarded.
4. Given a closed path, `Make Selection` with each Operation (New/Add/Subtract/Intersect) yields the corresponding selection combination.
5. Given `Fill Path` with Mode `Clear`, pixels are erased to transparency; on the Background layer the option is refused.
6. Given `Stroke Path` with `Simulate Pressure`, the stroke varies with the current brush; each `Stroke Path` button click increases stroke opacity.
7. Given a saved path designated as a clipping path, exporting to TIFF/EPS carries the designation and produces the expected transparent region.
8. Given two paths, only one is active at a time and the image overlay follows it; dragging reorders saved paths but not vector masks or the work path.
9. Given `Panel Options > None`, thumbnails are hidden and all operations remain available.
10. Given an unsaved work path and a new path-draw, the previous work path is replaced (and the UI indicated it was unsaved).

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` (downloaded, `pdftotext -layout`) — official CS6 Help. Sections used: "Paths panel overview" (`Window > Paths`; saved path / work path / vector-mask path with anatomy A–C; select/deselect; `Panel Options` thumbnail size/`None`; stacking order and the vector/work reorder restriction); "Create a new path in the Paths panel" (New Path button, `Alt`/`Option`-click naming); "Create a new work path" (shape/pen Path mode; Add/Subtract/Intersect/Exclude path area options; `Shift`/`Alt` modifiers); "Manage paths" (work-path temporariness; saving/renaming; deleting; vector masks linked to the parent layer); "Fill paths with color" (button/`Alt`-click/menu; Use, Opacity, Mode incl. `Clear`, Preserve Transparency, Feather Radius, Anti-aliased; active-layer restriction; `Fill Subpath`); "Stroke paths with color" (button/`Alt`-click/menu; tool choice; `Simulate Pressure`; `Stroke Subpath`); "Convert paths to selection borders" (thumbnail `Ctrl`-click; button; `Make Selection` dialog with Feather/Anti-aliased/Operations); "Convert a selection to a path" (`Make Work Path` button/dialog; Tolerance 0.5–10; feather discarded); "About paths" (a path can define a selection, a fill, or a stroke, and saved paths can be designated as export clipping paths); "Keys for the Paths panel" (thumbnail combine modifiers; `Ctrl+Shift+H` hide; `Alt`-click fill/stroke/load/make-work-path/new-path buttons).
- `http://www.photoshopforphotographers.com/3101-1901/Help_guide/tp/Paths_palette.html` — CS6-era book companion: work path is temporary and promoted via double-click or drag to New Path; path can become a vector mask or be saved as a clipping path in EPS/TIFF; Fill/Stroke buttons; Make Selection / Make Work Path.
- `https://www.underwaterphotography.com/PhotoShop/PhotoShop/1_20_8_1.html` (fetched) — reproduces Adobe's "Using image clipping paths to create transparency": the `Clipping Path` dialog's `Path` and `Flatness` fields, flatness range 0.2–100, the 8–10 / 1–3 dpi guidance, the hard-edge/feather limitation, and the EPS/DCS/PDF/TIFF save requirements.
- `https://www.photoshopessentials.com/basics/managing-panels-in-photoshop-cs6` (fetched) — CS6 Essentials default workspace: Paths is tabbed with Layers and Channels.
- `https://frameandfocal.com/post-processing/how-to-use-the-pen-tool-in-photoshop` and `https://www.reddit.com/r/photoshop/comments/3168rn/make_work_path_tolerance_default` — CS6-era/community corroboration that the Make Work Path default tolerance is 2 px (the Help itself says only "use the default value").

Consulted as search-result snippets only (not individually fetched; community): SearXNG queries for "Photoshop CS6 Paths panel menu Clipping Path Save Path Panel Options" and "Photoshop make work path tolerance default" — confirmed the panel-menu `Clipping Path…` entry, the `Save Path` flow, and the 2 px default.

Not used in this pass:

- `helpx.adobe.com` (HTTP 403) — modern `paths` / `editing-paths` pages inaccessible; the archived CS6 Help PDF was used instead.
- `https://www.photoshopforphotographers.com/3101-1901/Help_guide/tp/propertiespanel.html` (retrieved successfully but belongs to `PAN-006`) and `https://graphicdesign.stackexchange.com/...` returned transport/403 errors.

## Open questions

- **Panel Options thumbnail default** on a fresh install is not stated. *Resolves with:* a preferences dump.
- **Work-path replacement warning.** Whether CS6 prompts before discarding an unsaved work path is not stated. *Resolves with:* a CS6 test.
- **Clipping-path PSD key/serialisation** is not asserted. *Resolves with:* `ARCH-008` / the Adobe file-format spec.
- **`Make Selection` anti-alias default** with nonzero feather (Help implies anti-aliasing requires feather 0) needs a CS6 confirmation. *Resolves with:* a CS6 test.
- **Make Work Path default tolerance** is now known to be 2 px from CS6-era/community sources but is not stated in the Help itself; a preference dump would make it normative.
