# Merging and Flattening Layers

- **Spec ID:** `LAY-031`
- **Status:** `Draft`
- **Parity tier:** `Core` (Merge To HDR Pro is `Core`; 3D-layer merge is out of core scope)
- **New in CS6:** `Changed` — CS6 renames the path "Combine" command to `Merge Shape Components` and moves it to the Path Operations drop-down; the layer merge/flatten commands and their shortcuts otherwise predate CS6. `Merge To HDR Pro` is the CS6 name of the HDR merge automation.
- **Depends on:** `05-layers/layers-overview.md`, `05-layers/layer-groups.md`, `05-layers/layer-masks.md`, `05-layers/vector-masks-and-clipping-masks.md`, `05-layers/smart-objects.md`, `05-layers/layer-styles.md`, `05-layers/blend-modes.md`, `05-layers/adjustment-layers.md`, `05-layers/fill-layers.md`, `05-layers/artboards.md`, `04-image-ops/32-bit-hdr.md`, `ARCH-008` document-model, `ARCH-009` undo-history, `06-filters/lens-correction.md`

> All module and type names below are **design proposals**. No code exists in
> this repository. Unverified behaviors (result naming, style/vector-mask
> handling, exact smart-object fate) are marked *(inferred)* or parked under
> `## Open questions`.

## CS6 behavior

Merging is **destructive**: it replaces several layer nodes with a single pixel
layer whose rendered result equals the composite of the inputs. The CS6 Help
warns that after saving a merged document you cannot revert to the unmerged
state.

### Merge commands

| Command | Path / shortcut | Input | Result |
|---|---|---|---|
| **Merge Down / Merge Layers** | `Layer > Merge Down / Merge Layers`, `Ctrl+E` / `Cmd+E` | One selected layer (Merge Down) or two or more selected layers/groups (Merge Layers) | One layer; the menu item's label follows the selection |
| **Merge Visible** | Layers panel menu / `Layer > Merge Visible`, `Ctrl+Shift+E` | All layers showing the eye icon (requires a visible layer to be selected) | One visible layer; hidden layers untouched |
| **Merge Clipping Mask** | `Layer > Merge Clipping Mask` | Raster base layer of a clipping mask | Clipping group collapsed into the base; base must be raster |
| **Stamp (selected)** | `Ctrl+Alt+E` | Multiple selected layers | New layer with the merged content; originals intact |
| **Stamp (all visible)** | `Ctrl+Shift+Alt+E` | All visible layers | New layer above the active layer; originals intact |
| **Merge Shape Components** | Path Selection options bar, Path Operations drop-down | Overlapping path components in one path | Single combined path component (CS6 name for CS5's `Combine`) |
| **Merge To HDR Pro** | `File > Automate > Merge To HDR Pro` | Multiple exposures of one scene | A new 32/16/8-bpc HDR document, not a layer merge |

### Merge semantics documented by the Help

- The data on the top layers replaces any data it overlaps on the lower layers.
- The intersection of all transparent areas in the merged layers remains
  transparent.
- An **adjustment or fill layer cannot be the target layer** of a merge.
- Two **3D layers** can be merged and share one scene; the top layer inherits
  the bottom layer's 3D properties; enabled only when camera views match
  (Extended).
- Linked layers can be merged after `Layer > Select Linked Layers`.
- Two adjacent layers/groups can be merged by selecting the top item and
  choosing the merge command; with one selected layer the item reads
  `Merge Down`, and with two or more it reads `Merge Layers`.

### Flatten Image

`Layer > Flatten Image` (or the Layers panel menu):

- Merges **all visible layers** into a single **Background** layer.
- **Discards hidden layers** (the Help says hidden layers are discarded; the
  community reports a confirmation prompt before discarding — see Open
  questions).
- Fills any remaining transparent areas with **white**.
- Permanently removes layer structure on save.
- Converting an image between some color modes flattens the file; save a copy
  if the layer stack is needed later.

### What is discarded

| Input | Fate on merge/flatten | Source |
|---|---|---|
| Type layer | Rasterized to pixels; text no longer editable | Help: vector data cannot be painted; merging rasterizes. Community-consistent *(inferred for exact step)* |
| Shape layer / vector mask / fill content | Rasterized; vector mask becomes pixels or a raster mask | Help (Rasterize options mirror this) |
| Adjustment layer | Applied into the composite pixels when not the target; parameters lost | Community/Adobe Help *(inferred)* |
| Fill layer | Applied into pixels | Help (cannot target a merge) |
| Clipping mask | Collapsed; clipping relationship lost | Help (Merge Clipping Mask) |
| Layer mask | Applied into alpha; the mask object is lost *(inferred)* | Community *(inferred)* |
| Layer style (fx) | Baked into pixels or dropped depending on recipe *(inferred)* | Community *(inferred)* |
| Smart Object | Rasterized unless merged as smart object; smart filters lost *(inferred)* | Community *(inferred)* |
| Group | Collapsed into a single layer | Help (groups are mergeable) |
| Hidden layers | Untouched by Merge Visible; discarded by Flatten | Help |

### Background handling

- A document has at most one `Background` layer, always bottommost, with a
  locked stacking order, blend mode, and opacity.
- A `Background` can be converted to a normal layer (`Layer > New > Layer From
  Background` / double-click), and a normal layer can become the Background
  (`Layer > New > Background From Layer`), which converts transparent pixels to
  the background color and drops it to the bottom.
- **Flatten** always produces a Background layer, filling transparency with
  white.
- `Merge Down` involving a Background is allowed for a normal layer above it
  *(inferred — exact result name/mode unverified)*.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Layer > Merge Down / Merge Layers` | Menu | `Ctrl+E` / `Cmd+E` | One context-sensitive item: the label follows the selection; also in the Layers panel menu |
| `Layer > Merge Visible` | Menu | `Ctrl+Shift+E` | All visible; one visible layer must be selected |
| `Layer > Merge Clipping Mask` | Menu | n/a | Base layer must be raster |
| `Layer > Flatten Image` | Menu | none listed | Also Layers panel menu |
| Layers panel menu | Menu | n/a | Same merge/flatten/stamp entries |
| `Ctrl+Alt+E` | Shortcut | yes | Stamp selected layers to a new layer |
| `Ctrl+Shift+Alt+E` | Shortcut | yes | Stamp all visible to a new layer |
| Path Selection options bar | Tool options | `A` | Path Operations drop-down → `Merge Shape Components` |
| `File > Automate > Merge To HDR Pro` | Dialog | n/a | HDR workflow, separate document |
| Layer context menu | Context menu | n/a | Merge Down / Merge Visible etc. *(not yet exposed by the row menu)* |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Merge command | enum | selection-dependent | Merge Down (one selected), Merge Layers (two or more), Merge Visible, Merge Clipping Mask, Flatten | Selection-dependent label and enablement |
| Merge To HDR Pro source files | file list | n/a | Browse / Add Open Files / Use Folder | At least 3 exposures recommended, minimum 3 |
| Attempt To Automatically Align Source Images | bool | off *(inferred)* | on / off | Equivalent to Auto-Align for hand-held shots |
| HDR bit depth | enum | 32 *(inferred)* | 32 / 16 / 8 bits/channel | Only 32-bit stores the full range |
| Tone mapping method (16/8-bit) | enum | Local Adaptation *(inferred)* | Local Adaptation, Equalize Histogram, Exposure And Gamma, Highlight Compression | For non-32-bit output |
| Remove Ghosts | bool | off | on / off | For moving objects across exposures |
| Response curve | curve / preset | auto-calculated | Save/Load Response Curve | Camera sensor response |
| Merge Shape Components | action | n/a | Path Operations drop-down | CS6 relabeling of `Combine` |
| Background fill color | color | white (Flatten) | n/a | Transparency in a flattened image becomes white |

## Algorithms & pipeline

### Merge Down / Merge Layers

1. Validate: every selected layer is visible enough / mergeable; the target
   (bottommost of a Merge Down, or the selected set) is not an adjustment or
   fill layer.
2. Determine the result's content rectangle as the union of the inputs' content
   rectangles.
3. Composite the inputs in stacking order into an RGBA buffer:
   - For Merge Down, the upper layer is composited over the lower layer using
     the upper layer's blend mode / opacity, while the **result layer inherits
     the lower layer's blend mode, opacity, and (for Merge Down) name**
     *(inferred)*.
   - For Merge Layers, the selected layers composite into one buffer; the
     result's blend mode/opacity is reset to Normal/100 or inherited from the
     bottom selected layer *(unverified)*.
4. Apply masks and clipping into the alpha channel.
5. Replace the input nodes with one `Pixel` node; adjust sibling order and any
   parent group references.

The compositing math is identical to normal layer compositing
(`05-layers/blend-modes.md`); parity is defined against the compositor, not a
separate code path. Resolution/bit depth follow the document.

### Merge Visible

Same as Merge Layers, but the input set is exactly the layers currently showing
the eye icon (ancestor visibility included); hidden layers are skipped and left
in place. Result placement is in the normal stacking position, not necessarily
bottommost *(inferred)*.

### Flatten Image

1. If hidden layers exist, discard them (CS6 Help) — a confirmation prompt is
   reported by community sources but not stated in the Help.
2. Composite all visible layers over a **white, opaque** backdrop the size of
   the document.
3. Replace the entire layer tree with a single `Background` layer.
4. Groups, artboards, masks, styles, and vector data are gone; the result is a
   full-document raster.

### Merge To HDR Pro (automation, not a layer op)

1. Load/accept N exposures.
2. Optionally auto-align (same engine as Auto-Align Layers, `LAY-030`).
3. Estimate a camera response curve (auto-calculated, or save/load a preset).
4. Reconstruct 32-bit radiance: weight each exposure by its reliability in the
   highlight/shadow range and merge.
5. Remove ghosts (moving objects) if requested; pick the best-exposed thumbnail
   as base.
6. Tone-map to 16/8-bit if the user chooses a lower output depth; for 32-bit,
   the white-point preview is stored in the file.
7. Produce a new document (PSD/PSB/HDR/PBM/OpenEXR/TIFF capable).

The exact merge weighting, response-curve solver, and tone-mapping operators
are closed. **Behavioral parity only, algorithm TBD**; see
`04-image-ops/32-bit-hdr.md` and `04-image-ops/adjustments/hdr-toning.md`.

### Merge Shape Components

Combine overlapping path components in one path into a single component using
the active path operation (Add/Subtract/Intersect/Exclude). Boolean path
arithmetic on Bézier contours; see `03-tools/path-selection-tools.md` and
`03-tools/pen-and-path-tools.md`.

## Rust module mapping

Proposals:

- `pictura_layers::merge` — `MergeScope { Down, Selected, Visible }`,
  `merge(&mut Document, MergeScope) -> Result<NodeId>`,
  `flatten(&mut Document) -> Result<NodeId>`, `stamp(&mut Document,
  StampScope) -> Result<NodeId>`.
- `pictura_layers::merge::validation` — `can_merge_target(&Node) -> bool`
  (rejects adjustment/fill targets), `is_visible_in_panel(&Node) -> bool`.
- `pictura_composite` — reused compositor; merge calls it into a scratch
  buffer, then bakes the buffer into one `Pixel` node. No second compositing
  implementation.
- `pictura_layers::path_merge` — boolean combine of `BezierPath` components
  (`pictura_geometry::path::boolean`).
- `pictura_automation::hdr_merge` — `HdrMergeOptions { align, bit_depth,
  tone_map, remove_ghosts, response_curve }`, `merge_to_hdr(...) -> Document`;
  backed by `pictura_imaging::hdr`.
- `pictura_core::pixel` — scratch `RasterBuffer` at the document bit depth
  (`u8`/`u16`/`f32`).

Crossing types: `NodeId(u64)`, `RectI`, `RasterBuffer`, `TileRef`. Merge is a
document mutation that must be issued through the command/undo layer, never
directly by the UI.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `LayerMergeActions` | `QActionGroup` | One selection-dependent Merge Down / Merge Layers item, plus Visible / Clipping Mask / Flatten / Stamp; enable state from selection and neighbor kind |
| `LayersPanelMenu` | `QMenu` | Mirrors the Layer menu merge entries |
| `FlattenConfirmDialog` | `QMessageBox` | Confirms discarding hidden layers on flatten (if CS6 prompts) |
| `HdrMergeDialog` | `QDialog` | File list, align/ghost options, bit depth, tone-mapping controls, histogram, response-curve preset |
| `HdrToneMapPane` | `QWidget` | Local Adaptation / Equal Histogram / Exposure-Gamma / Highlight Compression controls |
| `PathOperationsMenu` | `QToolButton` + menu | Path Operations drop-down including `Merge Shape Components` |

Merge and flatten run on a worker with a progress dialog; on commit the model
emits one structural change so the Layers view collapses the removed rows
atomically.

## Data-model impact

- Merge/flatten **replace node subtrees**. The undo record must retain the full
  pre-merge node descriptors (kind, name, transform, masks, styles, params) plus
  the pre-merge pixel tiles, because the operation is destructive. Undo record
  shape: `Command { label, affected: Vec<NodeId>, before: Vec<StateDiff>,
  after: Vec<StateDiff>, pixel_backups: Vec<TileRef> }` (`ARCH-009`);
  `before` includes the removed children list so the tree can be rebuilt.
- The result is a `Pixel` node (or a `Background` node after flatten). Any
  `Adjustment`/`Fill`/`Text`/`Shape`/`SmartObject` kind is lost, so those nodes
  cannot be reconstructed from the merged result — only from the undo record.
- **PSD/PSB:** the merged state serializes as ordinary layer records plus a
  merged composite. Flatten sets the filespec's "no layers" state (merged image
  only). Background is not a separate PSD concept; it is a normal bottom layer
  with the `B0` name convention *(inferred)*.
- **Smart objects:** merging a smart object with another layer rasterizes it
  *(inferred)*; `New Smart Object via Copy` / "merge as smart object" keeps it
  editable.
- **Clipping masks / groups / vector masks:** the clipping and group nesting
  are gone after merge, so their containers must be removed from the tree.

## Edge cases

- **Only one layer** — the merge item reads Merge Down and is disabled when
  there is nothing below; the Merge Layers case and Merge Visible are disabled
  when fewer than two eligible layers are selected or visible.
- **Target is adjustment or fill layer** — merge must be rejected.
- **Hidden active layer** — Merge Visible requires the selected layer to be
  visible; hidden layers are not merged into the result.
- **All layers hidden + Flatten** — result is an empty/transparent document
  filled with white; confirm behavior.
- **Empty layers / zero-area content** — contribute nothing; result bounds come
  from the non-empty inputs; zero-area is legal.
- **Clipping-mask base not raster** — `Merge Clipping Mask` disabled.
- **Groups** — merging a group collapses its children; group opacity/blend must
  be folded into the composite.
- **3D layers (Extended)** — Merge Layers shares a scene; out of core scope,
  preserve as opaque if unsupported.
- **Video/frame-animation layers** — out of core scope.
- **8/16/32-bit** — compositing runs at the document depth; 32-bit float uses
  `f32` buffers and 4× memory per tile.
- **CMYK / Lab / Multichannel / Duotone** — merge/flatten operates on the
  document channels; Bitmap/Indexed restrict most layer operations.
- **Very large / PSB documents** — the scratch buffer may be huge; tile it and
  stream; do not allocate one full-canvas buffer when avoidable.
- **Undo** — one state per merge/flatten; `Edit > Undo` must restore the exact
  prior tree and pixels.
- **Merge To HDR with <3 images** — allowed but discouraged; missing EXIF
  triggers the manual EV dialog.
- **Merge To HDR with motion** — Remove Ghosts needs a best-exposed base
  thumbnail; movement in very light/dark areas may need a different base.
- **No GPU** — CPU compositing fallback for merge/flatten and HDR reconstruction.

## Parity acceptance criteria

- Given a pixel layer over a pixel layer, `Layer > Merge Down` yields one layer
  whose pixels equal the two-layer composite (within the compositor tolerance),
  with alpha = union of nontransparent areas.
- Given a type/shape/adjustment/fill layer merged with a pixel layer, the result
  is a raster layer and the original vector/parametric kind is gone.
- Given an adjustment layer as the intended merge target, merge is rejected and
  no layer changes.
- Given layers A (hidden) and B (visible), `Merge Visible` merges only B and
  leaves A in place; `Flatten Image` removes A.
- Given any transparency, `Flatten Image` yields a single Background with white
  opaque pixels and no alpha.
- Given a document with a group, clipping mask, layer mask, and layer styles,
  flattening removes all of them and the composite matches the pre-flatten
  render.
- Given `Ctrl+Alt+E` on two layers, a new layer holds the merged content and the
  originals remain; given `Ctrl+Shift+Alt+E`, all visible layers are stamped
  above the active layer.
- Given a CMYK 16-bit document, merge/flatten preserve mode and depth and match
  the pre-merge composite.
- Given a merge, `Edit > Undo` restores the exact prior tree (kinds, names,
  masks, styles, order) and pixels.
- Given `File > Automate > Merge To HDR Pro` on 3+ exposures, a new document is
  produced at the chosen depth; with 32-bit, all radiance above paper white is
  preserved.
- Given overlapping path components, `Merge Shape Components` produces one
  component with the active boolean operation applied.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  official Photoshop CS6 Help. Established: merge/stamp/flatten command names
  and shortcuts (pages 87–89 shortcut table, page 161 merging layers, page 162
  flatten); "top-layer data replaces overlapping lower data" and "intersection
  of transparent areas stays transparent"; adjustment/fill layers cannot be a
  merge target; clipping-mask merge requires a raster base; 3D merge rules;
  stamping to a new layer; Flatten discards hidden layers, fills transparency
  with white, and cannot be reverted after save; Merge Shape Components is the
  CS6 name replacing CS5's `Combine` (page 439); Merge To HDR Pro dialog flow,
  bit depths, tone-mapping methods, Remove Ghosts, and response curves
  (pages 136–138).

Search-result snippets only (not fetched; treat as unverified):

- `https://helpx.adobe.com/photoshop/desktop/create-manage-layers/color-adjustment-fill-layers/merging-adjustment-or-fill-layers.html`
  — current Adobe Help on merging adjustment/fill layers (rasterization).
- `https://community.adobe.com/questions-712/what-is-the-difference-between-merge-down-merge-visible-and-flatten-image-1083175`
  and `https://www.learn-photoshop.club/resources/free-tutorials/flatten-image-vs-merge-layers`
  — community descriptions of hidden-layer discard and white fill.
- `https://www.photoshopessentials.com/basics/how-to-merge-layers-as-smart-objects-in-photoshop`
  — "merge as smart object" workflow.

## Open questions

- **Result naming and inherited attributes.** Does Merge Layers name the result
  after the top selected layer, and does it inherit blend mode/opacity from the
  bottom? Merge Down apparently keeps the lower layer's name. Resolve with a CS6
  test document.
- **Layer style fate on merge.** Are `fx` effects baked into pixels, dropped, or
  preserved? Community sources conflict. Resolve with a CS6 test.
- **Vector mask fate on merge.** Rasterized to a raster mask or lost? Resolve
  with a CS6 test and PSD inspection.
- **Smart Object merge.** Confirm that merging a smart object with a raster
  layer rasterizes it, and define the "merge as smart object" variants.
- **Flatten confirmation prompt.** CS6 Help does not state whether Flatten
  prompts before discarding hidden layers; community says it does. Resolve with
  a CS6 screenshot.
- **Merge Down onto a Background.** Allowed? What is the result's name, mode,
  and opacity? Unverified.
- **Merge Layers result position** when the selection is non-contiguous.
- **HDR algorithm.** Response-curve solver, exposure weighting, ghost removal,
  and tone-mapping operators are unpublished. Resolve by defining behavioral
  tolerances in `04-image-ops/32-bit-hdr.md`.
- **3D layer merge (Extended)** — core-scope decision; see
  `00-overview/feasibility-and-non-goals.md`.
- **Merge To HDR Pro edition availability** (Standard vs Extended) is not
  stated by the fetched. Resolve from CS6 edition documentation.
