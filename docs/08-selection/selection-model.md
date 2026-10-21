# Selection Model

- **Spec ID:** `SEL-001`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `No` — the per-pixel coverage-mask model, the four selection modes, anti-aliasing, feathering, marching ants, and alpha-channel round-tripping are all CS5-era and earlier. CS6 adds selection *features* (Color Range Skin Tones/Detect Faces, SEL-005) but does not change the underlying model.
- **Depends on:** `01-architecture/document-model.md` (`ARCH-008`), `01-architecture/undo-history.md` (`ARCH-009`), `01-architecture/file-formats.md` (`ARCH-011`), `01-architecture/gpu-rendering-pipeline.md` (`ARCH-006`), `03-tools/marquee-selection.md` (`TOOL-002`), `03-tools/lasso-selection.md` (`TOOL-003`), `03-tools/quick-selection-and-magic-wand.md` (`TOOL-004`), `SEL-002`..`SEL-005`, `05-layers/layer-masks.md` (`LAY-004`), `10-workflow-io/document-lifecycle.md`.

> All crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Behavior is taken from the fetched CS6 Help corpus;
> anything inferred or drawn from community sources is marked.

## CS6 behavior

A Photoshop **selection** isolates one or more parts of an image so that edits, filters, and fills affect only those pixels. The CS6 Help defines it behaviorally: selecting particular areas lets you edit and apply effects and filters to those parts while leaving the unselected areas untouched.

Internally a selection is a **document-sized coverage mask**: one value per pixel expressing how much of that pixel belongs to the selection. This is why the CS6 Help calls saved selections *masks*: selections can be copied, moved, and pasted, or saved into an alpha channel, which stores them as grayscale images called masks. The Help frames the mask as the inverse-facing object — it covers the unselected part of the image and protects it from editing — and notes the round trip: a stored mask can be converted back into a selection by loading the alpha channel into an image.

User-visible model:

- **Hard selections** come from geometric tools (rectangular/elliptical marquee, single row/column, lasso, polygonal, magnetic) and from hard-edged auto tools (Magic Wand with anti-aliasing off). Coverage is 0 or "fully selected".
- **Soft selections** come from anti-aliasing, feathering, Color Range, Quick Selection, Focus/Refine operations, and channel loads. Coverage is a continuous transition; a pixel can be partially selected.
- **The 50% line is the selection.** When a feathered/anti-aliased mask is converted to a selection, CS6 states that the boundary line runs halfway between the black and white pixels of the mask gradient, marking the transition between pixels that are less than 50% selected and those that are more than 50% selected. The same 50% coverage contour is what the marching ants trace (SEL-002).
- **Selected ↔ masked.** Mask convention: white = fully selected/editable, black = unselected/protected, gray = partially selected. The **Quick Mask** (SEL-004) and the **layer mask** (LAY-004) use this convention; Quick Mask and layer masks display in rubylith/grayscale.
- **Selection vs mask vs layer mask.** The *selection* is a single document-level active region (there is at most one active selection per document at a time). A *mask* is a stored grayscale image — an alpha channel, a Quick Mask temporary channel, or a per-layer mask. A *layer mask* is a mask attached to a layer that hides (black) or reveals (white) that layer; a selection is loaded into it or painted into it. The CS6 Help separates these: a selection becomes a layer mask by loading it to make it active and then adding a new layer mask, and paths can be converted to selections just as selections can be converted to paths.
- **Selection has no compositing effect by itself.** It only constrains the *next* operation; CS6 warns that a stale hidden selection can make a tool misbehave, so if a tool is not working as expected a hidden selection may be the cause — run Deselect and try the tool again.
- **Size limit.** A selection spans the document; there is no fixed element count either way, so an empty selection (no pixels) and an all-pixels selection are both valid, as is a selection extending partly beyond the canvas (marquee borders can be dragged beyond the canvas and reappear intact).

### Selection operations

Every selection-producing tool exposes the four modes **New**, **Add To**, **Subtract From**, **Intersect With**; modifiers `Shift`/`Alt`/`Alt+Shift` map to Add/Subtract/Intersect while drawing. The CS6 Help gives the observable result:

- **New** replaces the active selection.
- **Add** unions the new primitive with the existing mask (pointer shows `+`).
- **Subtract** removes the new primitive from the existing mask (pointer shows `−`).
- **Intersect** keeps only the overlap (pointer shows `x`).

Later committed primitives combine with whatever set the current mask; a fresh selection begins from New. The same four semantics appear elsewhere as *Replace/Add/Subtract/Intersect* — `Select > Save Selection`/`Load Selection`, channel loads, and `Select > Color Range` all reuse them (e.g. `Ctrl+Shift`-click an alpha channel adds it).

### Anti-aliasing and feathering

CS6 distinguishes two edge softeners:

- **Anti-aliasing** smooths the jagged edges of a selection by softening the color transition between edge pixels and background pixels; because only the edge pixels change, no detail is lost. Available on the Lasso, Polygonal Lasso, Magnetic Lasso, Elliptical Marquee, and Magic Wand tools only. **Set before the selection is made:** the option must be specified before using these tools, because anti-aliasing cannot be added after a selection is made.
- **Feathering** blurs edges by building a transition boundary between the selection and its surrounding pixels; that blurring can cause some loss of detail at the selection edge. Feather is set on the tool (0–250 px) or applied after the fact via `Select > Modify > Feather`. **Feathering effects become apparent only after you move, cut, copy, or fill the selection** — the ants do not move just because feather was applied.

Both are edge/coverage operations on the *new* primitive before it is combined into the mask.

### Marching ants

The selected region is drawn as an animated dashed outline (marching ants) traced from the mask's 50%-coverage contour. It is **view state, not pixel data**:

- `View > Extras` toggles selection edges along with grids, guides, target paths, slices, annotations, layer borders, count, and smart guides.
- `View > Show > Selection Edges` toggles the edges for the *current* selection only; the edges reappear when a different selection is made.
- `Ctrl+H` (Windows) / `Cmd+H` (macOS) is the shortcut for "Hide selection and planes."

The ants are the only default visible evidence of a selection; a feathered selection can be so faint that its ants are effectively invisible (CS6 warns "No pixels are more than 50% selected"), yet the selection still exists and still constrains edits.

### Selection vs alpha channel

A selection can be saved to and loaded from an alpha channel. CS6:

- Save: `Select > Save Selection` (or the Channels panel's Save Selection button), choosing Document, Channel, Name, and an operation (**Replace Channel**, **Add to Channel**, **Subtract From Channel**, **Intersect With Channel**).
- Load: `Select > Load Selection` (or `Ctrl`-click the channel), choosing Document, Channel, **Invert**, and operation (**New Selection**, **Add To Selection**, **Subtract From Selection**, **Intersect With Selection**).
- Drag a channel's thumbnail to load it; modifier-clicks combine (add/subtract/intersect).
- `Select > Save Selection` can also write directly to a **layer mask** if the image contains layers.

Up to 56 channels total exist per document including alpha, so the number of savable selections is bounded by the document's channel budget (`ARCH-008`).

### Bit-depth and color-mode behavior

- **8/16-bit:** the selection/mask model is identical; a selection is bit-depth-independent.
- **32-bit (32 bpc):** selections still work, but several *selection-producing* commands are unavailable — `Select > Grow`/`Similar` (documented as unavailable on Bitmap-mode and 32-bits-per-channel images), `Select > Color Range` (SEL-005), and the Magnetic Lasso (`TOOL-003`). Refine Edge behavior at 32 bpc is not addressed in the fetched Help.
- **Color modes:** selection is color-space independent; CMYK/Lab/Grayscale/Bitmap documents all hold a coverage mask without color conversion. Color-based tools sample in the working space and are unavailable on Bitmap mode for some commands.

## UI surface

The selection model itself has no single UI surface; it is exposed through every selection tool, the Select menu, the Channels panel, and the overlays. Cross-tool surface is tabulated in `SEL-002`; per-tool surface in `TOOL-002`/`TOOL-003`/`TOOL-004`.

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Canvas | Overlay | — | Marching-ants 50% contour |
| `View > Extras` | Menu | — | Toggle selection edges with other extras |
| `View > Show > Selection Edges` | Menu | — | Toggle edges for current selection only |
| `View > Show > Selection Edges` | Menu | `Ctrl+H` / `Cmd+H` | "Hide selection and planes" shortcut table |
| `Select` menu | Menu | — | All/Deselect/Reselect/Inverse/modify/save/load (see `SEL-002`) |
| Channels panel | Dock | — | Alpha-channel storage of saved selections; load/combine |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Coverage value | u8 per pixel | 0 | 0–255 | 0 = unselected, 255 = fully selected; float equivalent 0..1 *(inferred representation)* |
| Selection-set enumeration | enum | New | New / Add / Subtract / Intersect | Shared by all selection tools and channel ops |
| Anti-aliased | bool | on | on / off | Tool-time only; tools listed above |
| Feather (tool) | int px | 0 | 0–250 | Tool-time |
| Feather (existing selection) | int px | 0 | dialog; CS6 help gives no max | `Select > Modify > Feather`, `Shift+F6` |
| Marching-ants threshold | u8 | 128 (50%) | fixed | Boundary between <50% and ≥50% selected |

## Algorithms & pipeline

### Representation

Document-sized, tiled, single-channel coverage buffer. The classic Photoshop representation is **8-bit grayscale** (selections saved as grayscale images called masks); the coverage range is 0–255, treated as 0..1. A selection is therefore independent of the document's pixel bit depth and channel count. Whether CS6 stores an alpha channel at the document bit depth (8/16/32) rather than 8-bit is not stated in the fetched Help — mark as an open question; the *coverage semantics* are identical either way.

### Combination (selection operations)

Given existing coverage `E` and incoming primitive coverage `N`, both in `[0, 255]`:

```text
combine(E, N, mode):
    New        -> N
    Add        -> max(E, N)
    Subtract   -> E * (255 - N) / 255      # equivalently min(E, 255 - N)
    Intersect  -> E * N / 255
```

This is the same formula stated in `TOOL-002` and is the standard alpha-compositing truth table for selection masks. Adobe's exact integer rounding for Subtract/Intersect is unspecified — behavioral parity only.

### Thresholding and conversion

- The visible boundary (ants) is the set of pixels whose coverage crosses 128 (50%). A marching-squares contour of the binarized mask at 50% yields the polyline (`TOOL-002` proposes this).
- Converting a mask back to a "hard" selection never quantizes the stored mask; partial coverage persists and is only interpreted at edit time.
- Converting a layer mask / alpha channel to a selection loads coverage as-is (with optional `Invert`: `255 - E`).

### Anti-aliasing

Anti-aliasing computes fractional coverage at boundary pixels of the *new* primitive (analytic coverage or supersampling; Adobe's exact filter is closed — *behavioral parity only, algorithm TBD*). Straight-edged tools (Rectangle Marquee, Single Row/Column) have no AA option; only the tools listed by CS6 expose it.

### Feathering

Feather blurs the hard coverage map with a kernel whose support scales with the radius (px). Adobe does not document the profile; a Gaussian with `σ ≈ radius/2` is the conventional approximation and is *inferred*. Feather is isotropic and distance-based. `Select > Modify > Feather` applies to an existing mask; tool feather applies to the primitive before combination.

```text
soften(mask, radius):          # shared by AA (edge-only) and feather (global blur)
    if radius <= 0: return mask
    return blur(mask, kernel(radius))
```

### Marching ants

Pure function of the mask: `contour(mask, level=128) -> Vec<Path>`; animated dashes are a paint-time effect. Never written into pixel data, never serialized. If the selection is empty, no contour is produced.

## Rust module mapping

- `pictura_selection::Selection` — the document-level active selection; wraps a `Mask`.
- `pictura_selection::Mask` — tiled coverage buffer (`u8` per pixel; `f32` variant if a float mask is ever needed), with `combine(&self, other, SelectionOp)`, `invert`, `threshold`, `is_empty`.
- `pictura_selection::op::SelectionOp` — `{ New, Add, Subtract, Intersect }`; the shared enum for tools, channel ops, and dialogs.
- `pictura_selection::rasterize` — primitive → coverage (rect/ellipse/band/polygon), anti-alias coverage.
- `pictura_selection::feather::feather(&mut Mask, radius_px)` — shared edge softener; also used by Refine Edge (`SEL-003`).
- `pictura_selection::contour::ants_contour(&Mask, level) -> Vec<Polyline>` — marching-squares at 50%.
- `pictura_selection::alpha` — `save_to_channel` / `load_from_channel` with `SelectionOp` and `invert`.
- `pictura_core::mask::RasterMask` — the storage type shared with layer masks (`LAY-004`); a `Selection` holds one.

Data crossing the boundary: `Mask` tiles (borrowed buffers / shared memory), `SelectionOp`, `Rect2`, and to the UI a `Vec<Polyline>` contour plus an `is_empty` flag — never the raw mask when only the outline is needed.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `SelectionModel` | `QObject` | Active selection id, empty/soft flags, revision counter for invalidation (`ARCH-006`) |
| `SelectionOverlay` | `QGraphicsItem` | Marching-ants contour, visibility bound to `View > Extras` / `Show > Selection Edges` |
| `SelectionActionGroup` | `QActionGroup` | Select menu enable/disable from `is_empty` (Deselect/Inverse/Modify inactive when empty; Reselect when history permits) |
| `ChannelsModel` | `QAbstractListModel` | Alpha-channel list; load-as-selection, save-selection actions (`ARCH-008`, `LAY-004`) |

The model is GUI-thread-only; mask arithmetic and contour extraction run in the Rust core and return tiles/contours to the GUI thread (`ARCH-003`). Widgets over QML because selection state is dense, keyboard-centric, and drives docked menus and panels.

## Data-model impact

- One **active selection** per document, not serialized by itself. A *saved* selection lives as an alpha channel (`ChannelKind::Alpha`) and is serialized per the PSD/PSB channel rules in `ARCH-011`.
- Undo granularity: **one command per committed selection change**, not per mouse-move. Record shape (matching `TOOL-002`): `SelectionChange { before: Option<MaskTileSnapshot>, after: Option<MaskTileSnapshot>, region: Rect2 }`, where `None` means "no selection". Only the changed bounding-box tiles are snapshotted; the contour is derived, never stored.
- Reselect (`Ctrl+Shift+D`) restores the most recent selection and must survive later edits per `ARCH-009`.
- Marching-ants visibility is **view state** (per-session), not document state; the `Show > Selection Edges` per-selection toggle is the only CS6 nuance and is transient.
- Tool/mode/feather/AA defaults are preferences/tool-preset state, not document state.

## Edge cases

- **Empty selection:** valid; ants show nothing; Deselect/Inverse/Modify are typically disabled. `Select > Deselect` clears the active selection without touching saved channels.
- **All-pixels selection:** from `Select > All` (or a marquee over the whole canvas); behaves as "no constraint" for most edits but is still an active selection.
- **No pixels > 50% selected:** feather larger than the primitive, or a Color Range preset with no matching colors, yields a selection whose ants are invisible yet which still exists and still constrains edits. CS6 accepts this with a warning; replicate.
- **Selection larger than canvas:** borders may leave and re-enter the canvas; coverage outside the document is clipped for storage but the border persists while dragging.
- **1-px documents:** a Single Row/Column selection covers the whole document; division/rounding guards needed.
- **Dense/empty alpha channels:** loading an all-black or all-white channel yields an empty or full selection; `Invert` flips it.
- **16-bit masks / 32-bit documents:** coverage stays logically 0..1; Grow/Similar, Color Range, and Magnetic Lasso are unavailable at 32 bpc and on Bitmap mode where noted.
- **CMYK/Lab:** no color conversion at selection time; Color Range and Magic Wand sample the working space.
- **PSB/huge documents:** tiled masks; invert and combine are per-tile; undo snapshots only dirty tiles. Inverting a 300k×300k mask must not allocate one flat buffer.
- **GPU unavailable:** masks are CPU data; the ants overlay falls back to `QPainter` (`ARCH-006`).
- **Undo/redo:** deselect/reselect and save/load selection each form one history state; an in-progress marquee drag is preview-only.

## Parity acceptance criteria

- Given no selection, applying `Add` from a tool produces exactly the primitive; applying `Subtract`/`Intersect` to no selection leaves nothing selected.
- Given existing mask `E` and primitive `N`, `Add` equals `max(E, N)` per pixel; `Subtract` equals `E·(1−N)`; `Intersect` equals `E·N` (each within integer rounding tolerance of ±1).
- Given `Select > Inverse`, the result equals `255 − E` per pixel.
- Given `Select > Save Selection` to a new channel then `Select > Load Selection`, the reloaded selection equals the saved mask exactly.
- Given `Select > All` then `Deselect` then `Reselect`, the active selection is byte-identical to the pre-deselect selection.
- Given `Feather = 250` on a 1000×1000 rectangle, no boundary pixel is 0 or 255 and the transition is monotone across the border within the feather distance.
- Given an anti-aliased Elliptical Marquee selection, boundary coverage includes values strictly between 0 and 255; with AA off, every value is 0 or 255.
- Given a feathered mask converted to/from Quick Mask, the marching-ants boundary sits at the 50% coverage contour (±1 px).
- Given `View > Show > Selection Edges` off, the ants vanish but every subsequent edit is identical to the ants-on case.
- Given `Ctrl+H` ("Hide selection and planes") then the same toggle, edge visibility returns to its prior state.
- Given a 1-px document, `Select > All` succeeds and `Select > Deselect` leaves no error.
- Given a 32-bpc document, Grow, Similar, Color Range, and the Magnetic Lasso are unavailable while the coverage model and other tools work.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — primary CS6 Help corpus (Feb 2013 revision; contains some "Creative Cloud only" notes). Established: definition and mask framing of a selection (isolating parts of an image, alpha channels storing selections as grayscale masks, and a mask as the inverse of a selection); selection vs layer mask and selection↔path conversion; hidden-selection warning; the four selection modes and their modifier shortcuts; anti-aliasing tool list and the rule that it must be set before the selection; feather 0–250, `Select > Modify > Feather`, `Shift+F6`, and the "No pixels are more than 50% selected" behavior; the 50% boundary rule when a feathered mask becomes a selection; `View > Extras`, `View > Show > Selection Edges`, `Ctrl+H`; `Select > All`/`Deselect`/`Reselect`/`Inverse`; `Select > Grow`/`Similar` unavailability on Bitmap and 32-bpc; Save/Load Selection dialogs, channel combination operations, and `Invert`; Move/Border/Expand/Contract/Smooth semantics and their 1–100 / 1–200 / radius ranges; the `Shift`/`Alt`/`Space` movement modifiers (45° constraint, 1-px/10-px nudge).

Canonical example of the combine formulas and shared selection operations is `03-tools/marquee-selection.md` (`TOOL-002`), which cites the same CS6 PDF and the same `max`/`min`/product rules; `03-tools/lasso-selection.md` (`TOOL-003`) and `03-tools/quick-selection-and-magic-wand.md` (`TOOL-004`) share the representation.

Not used in this pass:

- `helpx.adobe.com` — HTTP 403.
- Direct `html.duckduckgo.com` and `search.brave.com` result pages returned JS/anti-bot shells without usable results; discovery was done through SearXNG instead (see `SEL-003`/`SEL-005`).

## Open questions

- **Alpha/selection storage bit depth.** Whether CS6 stores a saved selection channel at 8-bit or at the document bit depth (8/16/32) is not stated in the fetched Help. Resolve by inspecting a PSD with a 16/32-bit document and a saved selection, or the Adobe File Formats Specification.
- **Integer rounding in Subtract/Intersect.** The exact per-pixel rounding (truncation vs round-half) is unspecified. Resolve by comparing masks saved from CS6.
- **Anti-alias and feather kernels.** Exact coverage filter and feather falloff are closed (shared with `TOOL-002`). Resolve with pixel comparisons against a CS6 reference.
- **"No pixels are more than 50% selected" mechanics.** Whether CS6 creates a truly empty selection or stores the sub-50% coverage is unstated. Resolve by saving the result to a channel.
- **Selection memory accounting.** How a document-sized active selection counts against scratch-disk/history budgets is not documented. Resolve in `10-workflow-io/scratch-disks-and-memory.md`.
- **Contour/ants at 32 bpc on GPU.** Whether the ants path is GPU-composited in CS6's Mercury engine or drawn as a CPU overlay is unknown; decide in `ARCH-006`/`02-ui-ux`.
