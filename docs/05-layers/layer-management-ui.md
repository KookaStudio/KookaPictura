# Layer Management UI (Layers Panel)

- **Spec ID:** `LAY-002`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `Changed` — CS6 adds **layer filtering/search** at the top of the panel, right-click **color labels**, a Properties panel that edits the selected layer components, simultaneous lock/blend/color edits across a multiple selection, `Rasterize Layer Style`, `00`/`Shift+00` opacity shortcuts, correct opacity/blend display on hidden layers, layer-name tooltips, `Tab`/`Shift+Tab` navigation while renaming, and shape-layer names that reflect the tool. The panel layout, visibility, lock, opacity/fill, blend-mode and panel-menu model is otherwise CS5-era.
- **Depends on:** `LAY-001`, `LAY-003`, `LAY-004`, `LAY-005`, `01-architecture/document-model.md` (`ARCH-008`), `01-architecture/qt6-ui-design.md`, `02-ui-ux/panels/layers-panel.md`, `05-layers/layer-filtering-and-search.md`, `05-layers/merge-and-flatten.md`.

> Module and type names are design proposals. Behavior is from the CS6 Help
> reference (cited under `## Sources`); inferred items are marked.

## CS6 behavior

The Layers panel "lists all layers, layer groups, and layer effects in an image."
It is used to show/hide layers, create layers, and work with groups; additional
commands live in the panel menu. Panel anatomy per the Help: **A** panel menu,
**B** layer group, **C** layer, **D** expand/collapse layer effects, **E** layer
effect, **F** layer thumbnail.

### Panel display and options

- **Display the panel:** `Window > Layers` (F7).
- **Panel menu:** click the triangle in the upper-right corner.
- **Thumbnail size:** `Panel Options` in the panel menu.
- **Thumbnail contents:** `Panel Options > Entire Document` shows the whole
  document in each thumbnail; `Layer Bounds` restricts to the object's pixels.
  Thumbnails can be turned off for performance/space.
- **Expand new effects:** `Layers panel Options > Expand New Effects` controls
  whether layer styles/smart filters appear expanded by default.
- **Expand/collapse groups:** click the triangle left of the folder icon.

### Filtering / search (CS6)

Filtering options at the top of the panel "help you find key layers in complex
documents quickly." You can display a subset of layers based on **name, kind,
effect, mode, attribute, or color label**.

1. Choose a filter type from the popup menu.
2. Select or enter the filter criteria.
3. Click the toggle switch to switch layer filtering on or off.

(Detailed per-type criteria and the exact toggle UI are covered in
`05-layers/layer-filtering-and-search.md`; the Help states the six dimensions
above but not every control's widget form.)

### Selecting layers

- Click a layer; `Shift`-click for contiguous, `Ctrl/Cmd`-click for
  noncontiguous. Ctrl/Cmd-clicking **outside** the thumbnail selects; clicking the
  **thumbnail** selects the layer's nontransparent areas as a selection.
- `Select > All Layers`; `Select > Similar Layers` (select a layer, then choose
  to select all like-kind layers).
- Deselect one with Ctrl/Cmd-click; click below the bottom layer (or `Select >
  Deselect Layers`) to have none selected.
- Select in the document window via Move tool **Auto Select** (Layer or Group)
  or the right-click/Control-click layer context menu (lists layers with pixels
  under the pointer).
- Linked layers "retain their relationship until you unlink them"; unlike a
  transient multiple selection. Link/unlink via the link icon at the bottom of
  the panel; `Shift`-click the link icon temporarily disables it (red X).

### Visibility, lock, opacity/fill, blend mode

- **Eye icon** toggles visibility. Alt/Option-click an eye shows only that
  layer/group and remembers prior visibility; a second Alt-click restores.
  Dragging through the eye column toggles multiple rows. Right-click (Windows) /
  Control-click (Mac) the eye offers "show/hide this layer/layer group only or
  all layers/layer groups". Only visible layers print.
- **Locks.** Full lock via **Lock All**. Partial locks: **Lock Transparent
  Pixels** (confines editing to opaque pixels; formerly Preserve Transparency),
  **Lock Image Pixels** (no painting), **Lock Position** (no moving). A solid
  lock icon means fully locked, hollow means partially locked; layers in a locked
  group show a dimmed lock. For type/shape layers, Lock Transparency and Lock
  Image are selected by default and cannot be deselected. `Layer > Lock Layers`
  / `Lock All Layers In Group` applies locks to a selection or group.
- **Opacity** is overall transparency (affects styles and blending modes);
  **Fill** affects only pixels/shapes/text, not effects such as drop shadows.
  Select one or more layers or groups and edit Opacity and Fill. **If a group is
  selected, only Opacity is available.** Background and locked layers cannot
  change opacity.
- **Blend mode** popup in the panel; also via `Layer > Layer Style > Blending
  Options`. There is no Clear mode for layers. See `LAY-001` for mode/depth
  restrictions.

### Create, duplicate, delete, rasterize, export

- **New layer/group:** Create a New Layer / New Group buttons, `Layer > New >
  Layer`/`Group`, or the panel menu. Alt/Option-click the button opens the New
  Layer dialog (Name, Use Previous Layer to Create Clipping Mask [not for
  groups], Color, Mode, Opacity, Fill With Mode-Neutral Color). Ctrl/Cmd-click
  adds below the current layer. `Shift`-click New Group adds the current selection
  to a new group.
- **Layer Via Copy / Layer Via Cut:** with a selection, `Layer > New > Layer Via
  Copy` copies the selection to a new layer; `Layer > New > Layer Via Cut` cuts
  it. Smart Objects and shape layers must be rasterized first.
- **Duplicate:** drag to the Create a New Layer button, or `Duplicate Layer` /
  `Duplicate Group` (panel/menu), optionally into another or a new document.
  Copy/paste duplicates pixels only (no layer properties).
- **Create layer from existing file:** drag a file onto the image (creates a
  Smart Object by default unless the preference is off).
- **Rasterize** (`Layer > Rasterize`): Type, Shape, Fill Content (leaves vector
  mask), Vector Mask (becomes a layer mask), Smart Object, Video, 3D (Extended),
  Layer (all vector data), All Layers. `Layer > Select Linked Layers` first to
  rasterize linked layers.
- **Delete:** Delete icon with confirmation, or Delete Layer/Group from the menu;
  drag to the Delete icon or press Delete (Alt/Option-click the icon) to skip
  confirmation. `Layer > Delete > Hidden Layers`; delete linked layers after
  `Select Linked Layers`.
- **Export layers:** `File > Scripts > Export Layers To Files` (all or visible).
- **Merge** and **Flatten** are covered in `05-layers/merge-and-flatten.md`;
  note adjustment/fill layers cannot be a merge target, and Merge Visible needs a
  visible layer selected.

### CS6 layer-management additions (from the JDI list)

| Change | Detail |
|---|---|
| Filtering/search | Six-dimension layer subsetting at the top of the panel. |
| Color labels via right-click | CS6 right-click the layer/group to choose a color (CS5 used layer properties). |
| Properties panel | Edits the components selected in the Layers panel without opening the old Masks/Adjustments panels. |
| Multi-select edits | Simultaneously change locking, blend mode, or color label for multiple selected layers. |
| `Rasterize Layer Style` | New command merges layer effects into the layer. |
| Style order | Layer styles reordered to the order they are applied (e.g. Drop Shadow below other effects). |
| Blend If badge | Shows a badge on a layer whose blending options have been customized. |
| FX toggle | Alt/Opt-click FX toggle arrows shows/hides all layer effects. |
| Hidden-layer readout | Correct opacity and blend mode are displayed for hidden layers. |
| Shortcuts | `00` and `Shift+00` set layer and fill opacity to 0%. |
| Ctrl/Cmd+J | Duplicates selected layer **groups** as well as layers. |
| Rename navigation | `Tab` goes to the next layer, `Shift+Tab` to the previous. |
| Tooltips | Layer tooltips include the layer name. |
| Shape naming | Shape layer names reflect the tool name (e.g. "Rectangle 1"). |

### Panel keyboard reference (selected)

| Result | Windows | Mac OS |
|---|---|---|
| Load layer transparency as selection | Ctrl-click thumbnail | Cmd-click thumbnail |
| Load layer mask as selection | Ctrl-click mask thumb | Cmd-click mask thumb |
| Open the layer mask menu | Right-click mask thumb | Right-click mask thumb |
| Delete the active layer mask | Delete/Backspace | Delete/Backspace |
| Group layers | Ctrl+G | Cmd+G |
| Ungroup layers | Ctrl+Shift+G | Cmd+Shift+G |
| Create/release clipping mask | Ctrl+Alt+G | Cmd+Option+G |
| Select all layers | Ctrl+Alt+A | Cmd+Option+A |
| Merge layers | Ctrl+E | Cmd+E |
| Merge visible | Ctrl+Shift+E | Cmd+Shift+E |
| New layer with dialog | Alt-click New Layer | Option-click New Layer |
| New layer below target | Ctrl-click New Layer | Cmd-click New Layer |
| Select top/bottom layer | Alt+. / Alt+, | Option+. / Option+, |
| Select next layer down/up | Alt+[ / Alt+] | Option+[ / Option+] |
| Move target layer down/up | Ctrl+[ / Ctrl+] | Cmd+[ / Cmd+] |
| Move layer to bottom/top | Ctrl+Shift+[ / ] | Cmd+Shift+[ / ] |
| Show/hide all other layers | Alt-click eye | Option-click eye |
| Toggle lock transparency | `/` | `/` |
| Toggle layer mask on/off | Shift-click mask thumb | Shift-click mask thumb |
| Toggle mask/composite view | Alt-click mask thumb | Option-click mask thumb |
| Toggle rubylith | `\` or Shift+Alt-click | `\` or Shift+Option-click |
| Create clipping mask | Alt-click dividing line | Option-click dividing line |
| Rename layer | double-click name | double-click name |
| New group below | Ctrl-click New Group | Cmd-click New Group |
| Add mask hides all/selection | Alt-click Add Layer Mask | Option-click Add Layer Mask |
| Vector mask reveal all/path | Ctrl-click Add Layer Mask | Cmd-click Add Layer Mask |
| Vector mask hide all/path | Ctrl+Alt-click Add Layer Mask | Cmd+Option-click Add Layer Mask |

(Full shortcut map: `02-ui-ux/keyboard-shortcuts.md`.)

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Layers panel | Dock | `F7` | Rows and columns: visibility, layer/group thumbnail, name, mask thumbnails, style badge, effect expander, link icon. |
| Panel header | Widget | n/a | Blend mode popup, Opacity, Fill, lock strip, (CS6) filter/search field + toggle. |
| Panel bottom | Widget | n/a | Link, Layer Style (fx), Layer Mask, Adjustment Layer, New Group, New Layer, Delete buttons. |
| Layers panel menu | Menu | n/a | Panel Options, New/Duplicate/Delete, Merge/Flatten, Blending Options, Lock, Select, and mask/vector-mask/clipping entries. |
| Row context menu | Context menu | right-click | CS6 color label, layer-type-specific entries, group properties, and disabled placeholders for Copy CSS/SVG and Artboards/Frames. |
| `Layer` menu | Menu | various | New, Arrange, Rasterize, Layer Mask, Vector Mask, Clipping Mask, Group, Smart Objects, Merge, Flatten. |
| Rename field | Inline edit | double-click name | `Tab`/`Shift+Tab` move between layers in CS6. |
| Properties panel | Dock | n/a | Contextual editor for the selected layer components (CS6). |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Thumbnail size | enum | medium | None / small / medium / large | Panel Options. |
| Thumbnail contents | enum | Entire Document | Entire Document / Layer Bounds | Panel Options. |
| Expand New Effects | bool | on | on / off | Panel menu. |
| Filter dimension | enum | none | name, kind, effect, mode, attribute, color label | CS6. |
| Filter on/off toggle | bool | off | on / off | CS6. |
| Blend mode | enum | Normal | 27 layer modes + group Pass Through | Panel header. |
| Opacity | percent | 100 | 0–100 | Group: available; Background/locked: no. |
| Fill | percent | 100 | 0–100 | Groups: not available. |
| Visibility | bool | on | — | Eye column. |
| Lock flags | bool set | none | All / Transparent Pixels / Image Pixels / Position | Type/shape force Transparent+Image. |
| Color label | enum | none | preset palette | Right-click in CS6. |
| Layer name | string | "Layer N" / tool name | — | Rename inline. |

## Algorithms & pipeline

The panel is a view over the document node tree; it holds no authoritative state.

1. **Model projection.** Flatten the `ARC-008` node arena into display order
   (topmost first), emitting a row per node and a child row per layer effect /
   smart filter. `NodeKind::Group`/artboard rows are expandable.
2. **Roles.** Emit id, name, kind, visibility, lock flags, blend mode, opacity,
   fill, clipping, color label, mask presence, and style badge as item roles.
3. **Filtering (CS6).** A proxy layer filters rows by the six dimensions; the
   toggle enables/disables the proxy. Filters are a view concern and must not
   mutate document state.
4. **Commands, not direct mutation.** Every edit (visibility, lock, opacity,
   fill, blend, color, rename, move, duplicate, delete, rasterize, link) is a
   command with an undo record (`ARCH-009`); the GUI thread queues commands and
   the model/view updates on completion.
5. **CLI parity.** Layer masks are stored as alpha channels; loading layer
   transparency/mask as a selection (Ctrl/Cmd-click thumbnail) shares the
   selection engine (`08-selection/`). *(mechanism inferred)*
6. **Reset semantics.** `Alt`-click solo-visibility snapshots prior visibility so
   it can be restored exactly.

## Rust module mapping

- `pictura_core::document::layer_ops` — command constructors: `NewLayer`, `DuplicateLayer`, `DeleteLayer`, `Reorder`, `SetVisibility`, `SetLock`, `SetOpacity`, `SetFill`, `SetBlend`, `SetColorLabel`, `Rename`, `Rasterize`, `Link`, `LayerViaCopy`, `LayerViaCut`.
- `pictura_core::layer_filter` — `LayerFilter { dimension, criterion, enabled }` and `filter_view(&Document) -> Vec<NodeId>`.
- `pictura_core::rasterize` — `RasterizeTarget { Type, Shape, FillContent, VectorMask, SmartObject, Video, ThreeD, Layer, AllLayers }`, producing replacement pixel nodes and alpha masks.
- `pictura_core::command` (`ARCH-009`) — undo/redo records for each op.
- Crossing types: `NodeId`, `LockFlags`, `BlendMode`, `RasterizeTarget`, `ColorLabel`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `LayersModel` | `QAbstractItemModel` | Node tree; roles for all panel columns and expandable effect/smart-filter rows. |
| `LayersFilterProxyModel` | `QSortFilterProxyModel` | CS6 six-dimension filtering; enable/disable toggle. |
| `LayersPanel` | `QWidget` (dock) | Header controls (blend/opacity/fill/lock/filter), `QTreeView`, and bottom button strip. |
| `LayerRowDelegate` | `QStyledItemDelegate` | Thumbnails, mask/clip/style badges, indentation/underline for clipping, lock/eye painting, color label. |
| `BlendModeComboBox` | `QComboBox` | Mode list, filtered by color mode and bit depth. |
| `LayerFilterBar` | `QWidget` | Filter-dimension popup, criterion editor, on/off switch. |
| `LayerPropertiesDialog` / `GroupPropertiesDialog` | `QDialog` | Name, color, mode, opacity (and clip-to-previous for layers). |
| `LayersPanelMenu` | `QMenu` | Panel-menu + row context-menu actions. |

Models are GUI-thread-only (`ARCH-003`); mutations go through the command layer.

## Data-model impact

- No new persistent fields beyond `LAY-001`: visibility, lock flags, opacity,
  fill, blend, clipping, color label, and link state already exist on `Node`.
- **Link groups** need a representation (a link-set id on `Node`, or a
  document-level `Vec<LinkSet>`); unlink clears it. *(design choice)*
- **Filter/search** is transient view state, not serialized.
- **Color label** maps to the layer's color field (PSD layer record); CS6's
  right-click editor does not change the value model, only the input path.
- Undo granularity: one state per user action; multi-select edits are a single
  command touching multiple node ids.

## Edge cases

- **Background and locked layers:** opacity/fill/blend/reorder must be refused,
  not silently applied.
- **Group selected:** Fill control is disabled; only Opacity is offered.
- **Type/shape layers:** Transparent and Image locks are forced on.
- **Layer Via Copy/Cut on Smart Objects/shape layers:** must require rasterizing.
- **Adjustment/fill merge targets:** refused.
- **Hidden layers:** must still report correct opacity/blend (CS6 behavior) and
  are excluded from printing/merge-visible.
- **Large selections / PSB:** multi-edit and filtering must stay O(rows), not
  copy pixel data.
- **Filter with no matches:** panel shows an empty view without deselecting the
  underlying active layer.
- **Undo/redo:** reversing a delete restores the node's children, order, masks,
  styles, link membership, and id.
- **GPU unavailable:** panel operations are CPU/command-path and unaffected.

## Parity acceptance criteria

1. Given a document, the panel lists rows top-to-bottom matching PSD/PSD layer
   order, with groups expandable and clipped layers indented under an underlined
   base name.
2. Given a group selected, the Fill control is unavailable and Opacity is
   editable.
3. Given a Background layer, opacity/blend/reorder edits are refused.
4. Given a type or shape layer, Lock Transparency and Lock Image are on and
   cannot be deselected in the lock strip.
5. Given CS6 filtering by each of name/kind/effect/mode/attribute/color label,
   only matching layers are shown, and toggling filtering off restores all rows
   without changing document state.
6. Given multiple selected layers, changing blend mode, lock, or color label
   applies to all selected in one undo step.
7. Given `00` and `Shift+00`, layer and fill opacity are set to 0%.
8. Given `Ctrl/Cmd+J` on a selected group, the whole group is duplicated.
9. Given a selected layer, `Alt`-click solo visibility snaps to that layer and a
   second `Alt`-click restores the prior per-layer visibility exactly.
10. Given a selection on a raster layer, Layer Via Copy creates a new layer
    containing only the selection, and Layer Via Cut additionally clears it.

## Sources

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  Photoshop CS6 Help (fetched with `curl`, extracted with `pdftotext -layout`).
  Sections used: "Layers panel overview" / "Filter layers (CS6)" (pp. 156–157);
  "Convert background and layers" / "Duplicate layers" (pp. 157–158);
  "Managing layers" — rename, color, rasterize, delete, export, merge
  (pp. 160–162); "Selecting, grouping, and linking layers" (p. 163–164); "Moving,
  stacking, and locking layers" (pp. 165–166); "Create layers and groups" /
  "Show or hide a layer, group, or style" (pp. 185–186); "Layer opacity and
  blending" (pp. 192–193); "Keys for the Layers panel" (pp. 89–90);
  "What's new in CS6" — Layers enhancements (p. 7) and JDI Layers list (p. 9).

## Open questions

- **Filter UI details.** The Help names six filter dimensions and a toggle but
  does not enumerate each dimension's criteria editor widget. *Resolves with:* a
  CS6 UI capture or the Layers `Panel Options` documentation.
- **Link-set persistence in PSD.** Whether CS6 stores link relationships as a
  distinct PSD block or as a transient state is not sourced. *Resolves with:* a
  CS6 PSD containing linked layers, or the file-format spec's linked-layer
  section.
- **Color-label palette.** The palette entries/coordinates are not itemized in
  the PDF. *Resolves with:* a CS6 UI capture or preset dump.
- **Rasterize Layer Style semantics.** Whether it bakes only effects or also
  clears the style list, and its undo granularity, is not documented. *Resolves
  with:* a controlled CS6 test.
- **Panel Options defaults** (thumbnail size/contents, Expand New Effects) on a
  fresh install are not stated. *Resolves with:* a default-preferences dump.
