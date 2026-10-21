# Selection Tools Overview & Shared Options

- **Spec ID:** `SEL-002`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `No` — the cross-tool selection options bar and the Select-menu operations carry over from CS5. The CS6 selection *additions* (Color Range Skin Tones / Detect Faces, and mask threshold/invert for layer masks) belong to `SEL-005` and `LAY-004`, not to this shared surface.
- **Depends on:** `SEL-001` selection-model, `SEL-003` refine-edge, `SEL-004` quick-mask, `SEL-005` color-range, `TOOL-002` marquee, `TOOL-003` lasso, `TOOL-004` quick-selection/magic-wand, `TOOL-044` quick-mask-tool, `03-tools/eyedropper-color-sampler-ruler.md`, `05-layers/layer-masks.md` (`LAY-004`), `01-architecture/qt6-ui-design.md`, `02-ui-ux/keyboard-shortcuts.md`, `02-ui-ux/toolbox-and-options-bar.md`.

> All module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Behavior is taken from the fetched CS6 Help corpus;
> community-reported values are marked.

## CS6 behavior

Every selection tool shares one options bar: the four **selection modes** (New / Add To / Subtract From / Intersect With), a **Feather** value, and — for the Elliptical Marquee, Lasso, Polygonal Lasso, Magnetic Lasso, and Magic Wand — an **Anti-aliased** checkbox. A **Refine Edge** button (`Ctrl+Alt+R`) opens the Refine Edge dialog (`SEL-003`) against the current selection. The CS6 Help's rule is that **feather and anti-alias are set before the tool is used**; anti-aliasing cannot be added afterwards, while feather can also be added to an existing selection via `Select > Modify > Feather`. Before adding to or subtracting from an existing selection, Help recommends matching the options-bar feather and anti-aliasing values to those of the original selection.

Selection-producing tools and their shortcuts (from the CS6 tool-key table):

- **Move** `V` (moves selected pixels when a selection is active).
- **Marquee group** `M`: Rectangular Marquee (default), Elliptical Marquee, Single Row, Single Column (`Shift+M` cycles; hidden tools via `Alt`-click).
- **Lasso group** `L`: Lasso, Polygonal Lasso, Magnetic Lasso.
- **Magic Wand** `W`; **Quick Selection** shares `W`.
- **Quick Mask Mode** `Q` (a mode, not a drawing tool; `SEL-004`).
- **Pen/shape/path** tools produce paths that convert to selections (`08-selection/paths-and-vector-selection.md`).

The Select menu is the shared command surface. In CS6 it contains (pixel-selection commands):

- `Select > All` — select all pixels on the active layer within the canvas.
- `Select > Deselect` — clear the active selection.
- `Select > Reselect` — restore the most recent selection.
- `Select > Inverse` — select the previously unselected part.
- `Select > Color Range…` — color/tonal range selection (`SEL-005`).
- `Select > Refine Edge…` — edge refinement (`SEL-003`).
- `Select > Modify >` submenu — **Border** (1–200 px), **Smooth** (sample radius 1–100), **Expand** (1–100 px), **Contract** (1–100 px), **Feather**.
- `Select > Grow` / `Select > Similar` — extend by Magic Wand tolerance (adjacent vs whole image); unavailable on Bitmap and 32-bpc documents.
- `Select > Transform Selection` — geometric transform of the selection border.
- `Select > Save Selection…` / `Select > Load Selection…` — alpha-channel round trip with the four combine operations (`SEL-001`).

`Select > Grow` and `Grow`/`Similar` reuse the Magic Wand's **Tolerance**; running either repeatedly increases the selection in increments. The CS6 Help warns Grow/Similar do not work on Bitmap-mode or 32-bits-per-channel images.

Layer-level entries also live under the Select menu but are **not pixel selections**: `Select > All Layers` (`Ctrl+Alt+A`), `Deselect Layers`, `Similar Layers`. Keep them distinct in the menu model.

### Cross-tool modifier behavior

While a selection tool is active on the canvas:

- `Shift`-drag → add; `Alt`/`Option`-drag → subtract; `Alt+Shift`-drag → intersect (a `+`, `−`, or `x` appears next to the pointer).
- `Space`-drag reposition a marquee mid-drag (marquee/lasso tools).
- With a selection present and the Move tool (or a selection tool), dragging inside the border moves the *border* (marquee/lasso set to New), while the Move tool moves the selected *pixels*.
- Move a selection border: drag it; constrain direction to multiples of 45° with `Shift`; nudge 1 px with arrow keys, 10 px with `Shift`+arrows.
- A selection border can be dragged partly beyond the canvas and back without loss, and can be dragged between open Photoshop documents of the same pixel dimensions.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Options bar | Button group | — | New / Add To / Subtract From / Intersect With (all selection tools) |
| Options bar | Spin box | — | Feather (0–250 px); default 0 |
| Options bar | Checkbox | — | Anti-aliased (Elliptical Marquee, Lasso, Polygonal Lasso, Magnetic Lasso, Magic Wand) |
| Options bar | Button | `Ctrl+Alt+R` / `Cmd+Opt+R` | Refine Edge |
| Tools panel | Tools | `M`, `L`, `W` | Selection-tool groups |
| Tools panel | Mode button | `Q` | Quick Mask mode toggle (`SEL-004`) |
| `Select > All` | Menu | `Ctrl+A` / `Cmd+A` | Standard CS6 shortcut |
| `Select > Deselect` | Menu | `Ctrl+D` / `Cmd+D` | Confirmed in CS6 key tables |
| `Select > Reselect` | Menu | `Ctrl+Shift+D` / `Cmd+Shift+D` | Standard CS6 shortcut |
| `Select > Inverse` | Menu | `Ctrl+Shift+I` / `Cmd+Shift+I`; `Shift+F7` | Function-key table lists `Shift+F7` for Inverse Selection |
| `Select > Color Range…` | Menu | — | `SEL-005` |
| `Select > Refine Edge…` | Menu | `Ctrl+Alt+R` / `Cmd+Opt+R` | `SEL-003` |
| `Select > Modify > Border` | Menu | — | 1–200 px, centered on edge |
| `Select > Modify > Smooth` | Menu | — | Sample radius 1–100 |
| `Select > Modify > Expand` | Menu | — | 1–100 px |
| `Select > Modify > Contract` | Menu | — | 1–100 px |
| `Select > Modify > Feather` | Menu | `Shift+F6` | Confirmed in CS6 function-key table |
| `Select > Grow` / `Similar` | Menu | — | Uses Magic Wand tolerance |
| `Select > Transform Selection` | Menu | — | Transform border, not pixels |
| `Select > Save Selection…` | Menu | — | To channel or layer mask |
| `Select > Load Selection…` | Menu | — | From channel, with `Invert` |
| `Select > All Layers` | Menu | `Ctrl+Alt+A` / `Cmd+Opt+A` | Layer selection, not pixel selection |
| `View > Extras` | Menu | — | Toggles marching ants with other extras |
| `View > Show > Selection Edges` | Menu | — | Toggles ants for current selection |
| Canvas | Modifier | `Shift` / `Alt` / `Alt+Shift` | Add / subtract / intersect while drawing |
| Canvas | Nudge | arrows / `Shift`+arrows | 1 px / 10 px selection movement |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Selection mode | enum | New | New / Add / Subtract / Intersect | Shared; some tools auto-switch to Add when a selection exists |
| Feather | int px | 0 | 0–250 | Tool-time; `Select > Modify > Feather` for existing selections |
| Anti-aliased | bool | on | on / off | Tool-time only; not on rectangular/single-row-column marquees |
| Refine Edge radius | int px | 0 | dialog; CS6 max unverified (community: up to 1000) | `SEL-003` |
| Refine Edge smooth | int | 0 | 0–100 | `SEL-003` |
| Refine Edge feather | int px | 0 | dialog | `SEL-003` |
| Refine Edge contrast | percent | 0 | 0–100% | `SEL-003` |
| Refine Edge shift edge | percent | 0 | −100..+100% | `SEL-003` |
| Border width | int px | — | 1–200 | `Select > Modify > Border` |
| Smooth sample radius | int px | — | 1–100 | `Select > Modify > Smooth` |
| Expand / Contract | int px | — | 1–100 | `Select > Modify > Expand/Contract` |
| Magic Wand tolerance | int | 32 *(community-reported)* | 0–255 | Drives Grow/Similar |

## Algorithms & pipeline

Shared semantics are owned by `SEL-001`; this spec owns the *dispatch* layer.

### Options → primitive pipeline

```text
tool_drag / click:
    primitive = rasterize(gesture)          # coverage mask
    primitive = antialias(primitive) if AA else primitive
    primitive = feather(primitive, Feather) if Feather else primitive
    selection = combine(existing_selection, primitive, mode)
    commit(SelectionChange)
```

Each tool maps option state to the same `combine` truth table (`max` / `E·(1−N)` / `E·N`) stated in `SEL-001` and `TOOL-002`.

### Grow / Similar (modify pipeline)

`Grow` = flood-fill from the current selection boundary into adjacent pixels within Magic Wand **Tolerance** (a color-distance threshold). `Similar` = the same test applied globally, not just adjacent pixels. Both are implemented on the composite/active-layer color data; Adobe's exact distance metric (RGB Euclidean vs perceptual) is unspecified — *behavioral parity only*.

### Select > Modify

- **Border** keeps pixels within `w/2` inside and `w/2` outside the existing border, producing a soft-edged band; e.g. 20 px → 10 px in and 10 px out.
- **Smooth** inspects a `radius`-pixel neighborhood; keeps a pixel if more than half its neighbors are selected, otherwise removes it — a majority filter that reduces patchiness and rounds jagged corners.
- **Expand/Contract** dilate/erode the coverage by N px; portions running along the canvas edge are unaffected by Expand.
- **Feather** blurs coverage (see `SEL-001`).

### Refine Edge dispatch

The options-bar Refine Edge button is enabled for selection-tool types and opens `SEL-003`; the dialog edits the current selection mask (and, when invoked from a mask's Properties panel, the layer mask instead — CS6 moved mask editing into the Properties panel, `LAY-004`).

## Rust module mapping

- `pictura_selection::options::SelectionOptions` — `{ mode: SelectionOp, feather_px: u16, anti_alias: bool }`; shared by all tool state machines.
- `pictura_selection::pipeline::apply_primitive(&mut Selection, primitive: &Mask, &SelectionOptions)` — rasterize→AA→feather→combine in one place so every tool shares it.
- `pictura_selection::modify` — `border(mask, w)`, `smooth(mask, radius)`, `expand(mask, n)`, `contract(mask, n)`, `feather(mask, r)`.
- `pictura_selection::grow::grow_similar(&Mask, color_source, tolerance)` — Magic-Wand distance flood/global test.
- `pictura_selection::commands::SelectionCommand` — undo command wrapper (`SEL-001`).
- `pictura_tools::*` — per-tool interaction, all consuming `SelectionOptions` + `pipeline::apply_primitive`.

Boundary types: `SelectionOptions`, `SelectionOp`, `Tolerance`, `Mask` tiles, `Rect2`, and overlay contours. The Qt layer never performs mask arithmetic.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `SelectionOptionsBar` | `QWidget` | Shared mode button group, feather spin box, AA check; per-tool subclasses add tool fields |
| `RefineEdgeButton` | `QToolButton` | Emits `openRefineEdge`; enabled per tool; bound to `Ctrl+Alt+R` |
| `SelectionModeButtonGroup` | `QButtonGroup` | Four exclusive mode buttons, keyboard-accessible |
| `AntiAliasCheck` | `QCheckBox` | Shown only for AA-capable tools |
| `FeatherSpinBox` | `QSpinBox` | 0–250, suffix "px" |
| `SelectionMenu` | `QMenu` | Populates the Select menu; enable/disable from `SelectionModel` emptiness and document mode |
| `ModifySubmenu` | `QMenu` | Border / Smooth / Expand / Contract / Feather, each a small dialog |
| `GrowSimilarActions` | `QAction` | Disabled on Bitmap/32-bpc documents |
| `SelectionOverlay` | `QGraphicsItem` | Marching ants (shared with `SEL-001`) |

Widgets over QML: the options bar is dense, keyboard-first, docked, and shares the same structure as every other tool options bar (`02-ui-ux/toolbox-and-options-bar.md`). The Select menu is a native `QMenu` populated from a Rust-provided descriptor so menu enablement stays consistent with document state.

## Data-model impact

- No new document fields: this spec is the shared command/options layer over the model in `SEL-001`.
- Each Modify/Transform command is one undo state with the `SelectionChange { before, after, region }` record.
- `Grow`/`Similar` read the active layer's composited pixels (or the current selection's source) and never mutate pixels.
- The Select-menu enablement matrix (mode availability, 32-bpc/Bitmap restrictions) belongs to the command dispatcher, not persisted.
- `Select > All Layers` / `Deselect Layers` / `Similar Layers` operate on the layers selection, a separate document field from the pixel selection.

## Edge cases

- **Hidden/stale selection:** a tool seeming broken is the CS6-documented symptom; expose Deselect prominently and never silently drop the selection.
- **Bitmap mode:** Grow/Similar and some color-based tools are unavailable; menu items disabled, not error dialogs.
- **32-bpc:** Grow/Similar, Color Range, and Magnetic Lasso unavailable (`SEL-001`, `TOOL-003`); the rest of the Select menu works.
- **Anti-alias/feather mismatch when adding:** CS6 advises matching the original selection's feather/AA; do not auto-reset the user's current values.
- **Modify on an empty selection:** Border/Smooth/Expand/Contract/Feather must be no-ops or disabled; avoid creating a selection from nothing.
- **Border vs canvas edge:** `Expand` leaves edge-hugging borders unchanged; `Border` centered on the edge produces partial coverage at the canvas boundary.
- **Transform Selection with no selection:** disabled (there is nothing to transform).
- **Multiple documents:** selection borders can be dragged between same-size documents; moving a border between documents must carry the mask, not re-derive it.
- **Undo granularity:** one state per committed command; drag previews are not undo states.
- **Huge/PSB:** Grow/Similar and Modify operate tiled with a dirty-rect undo snapshot.
- **GPU unavailable:** no behavioral change; overlays use the CPU path.

## Parity acceptance criteria

- Given any selection tool, switching from New to Add and drawing unions the primitive; Subtract removes it; Intersect keeps overlap — matching the `SEL-001` formulas.
- Given `Feather = 10` set before drawing, the committed mask is feathered; setting Feather after a hard selection and drawing again feathers only the new primitive.
- Given the Refine Edge button, it opens the `SEL-003` dialog for an existing selection and is a no-op/disabled when the selection is empty.
- Given `Select > Modify > Border` with 20 px, the new band extends 10 px inside and 10 px outside the original border.
- Given `Select > Modify > Smooth` with radius 1, isolated 1-px specks are removed and holes filled where the majority neighborhood test requires.
- Given `Select > Modify > Expand` with 5 px next to the canvas edge, the edge-adjacent border does not move.
- Given `Select > Grow` twice, the selection grows at least as fast as once and never shrinks.
- Given `Select > Similar` on a flat-color region, non-adjacent matching-color pixels anywhere in the image join the selection.
- Given a Bitmap-mode or 32-bpc document, Grow and Similar are disabled.
- Given `Ctrl+Alt+R` with a selection-tool active, the Refine Edge dialog opens; given `Ctrl+Shift+I`, the selection inverts.
- Given `Select > Deselect` then `Select > Reselect`, the selection returns exactly (`SEL-001`).
- Given `Select > Save Selection` to a channel, then `Select > Load Selection` with Add/Subtract/Intersect, the combine matches `SEL-001`'s truth table.
- Given `Select > All Layers` (`Ctrl+Alt+A`), no pixel selection is created or altered.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — primary CS6 Help corpus. Established: the shared selection-mode buttons and their `Shift`/`Alt`/`Alt+Shift` pointer feedback; the recommendation to match the original selection's feather and anti-aliasing values before adding to or subtracting from it; anti-aliasing tool list and set-before rule; feather 0–250 and `Select > Modify > Feather`; `Select > All`/`Deselect`/`Reselect`; `Select > Inverse`; `Select > Modify` Border 1–200, Smooth sample radius 1–100, Expand/Contract 1–100, and their effects; `Select > Grow`/`Similar` and the Bitmap/32-bpc restriction; `Select > Transform Selection`; `Select > Save Selection`/`Load Selection` and the four channel operations; `Select > All Layers` `Ctrl+Alt+A`, `Deselect Layers`; Move/constrain/nudge rules for selection borders; `Ctrl+D` deselect; `Shift+F6` Feather Selection; `Shift+F7` Inverse Selection; `Ctrl+H` hide selection; `Ctrl+Alt+R` Refine Edge; the `M`/`L`/`W`/`Q` tool-slot shortcuts; `View > Extras` / `View > Show > Selection Edges`.

Not used in this pass:

- `helpx.adobe.com` — HTTP 403.
- Direct `html.duckduckgo.com` / `search.brave.com` result pages — JS/anti-bot shells.

## Open questions

- **Modify ranges on 16/32-bpc.** Whether Border/Smooth/Expand/Contract have the same numeric ranges at 16/32-bit is unstated. Resolve with a CS6 UI capture.
- **Grow/Similar color-distance metric.** The exact threshold metric and relationship to the Magic Wand Tolerance value are closed. Resolve with a color-ramp comparison against CS6.
- **Transform Selection interpolation.** The transform engine, interpolation, and whether AA is applied to the transformed border are unspecified. Resolve in `08-selection/transform-selection.md` with a reference comparison.
- **Menu enablement matrix.** The precise enable/disable rules (e.g. whether Inverse is disabled for an empty selection, whether Reselect survives save/load) are not fully documented. Resolve with a CS6 UI capture.
- **Standard shortcut confirmation.** `Ctrl+A`, `Ctrl+Shift+D`, and `Ctrl+Shift+I` are standard Photoshop shortcuts but were not located verbatim in the fetched corpus; the corpus confirms `Ctrl+D`, `Shift+F6`, `Shift+F7`, `Ctrl+Alt+R`, and `Ctrl+H`. Confirm against a CS6 shortcut map before asserting.
