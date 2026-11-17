# Layers panel program — CS6 research, gap analysis, and staged plan (M36–M43)

- **Status:** research + plan. M36 (layer attributes) is implemented and
  archived; M37 (layer creation and grouping) is implemented and archived; **M38
  is the user-requested icon/cursor library**
  (`openspec/changes/m38-icon-cursor-library`); the panel stages M39–M43
  are not yet proposed.
- **Contract:** `docs/05-layers/layer-management-ui.md` (`LAY-002`) is the
  long-form behavior spec; `docs/02-ui-ux/panels/layers-panel.md` (`PAN-001`) is
  the widget spec; the canonical requirements are
  `openspec/specs/ui/layers-panel/spec.md`, `openspec/specs/codec/psd-layer-io/spec.md`,
  and `openspec/specs/compositing/layer-compositing/spec.md`.
- **Goal:** make the Layers panel behave like Photoshop CS6's.
- **Non-goals:** changing `docs/` (this file is additive), the canvas-compositing
  perf series (`docs/dev/canvas-compositing-plan.md`), or any behavior owned by
  a different milestone.

> Milestone-numbering note. This program claims **M36–M43**. The deferred
> canvas-performance tracks previously sketched as "M36–M38" in
> `docs/dev/STATE.md` (history copy-on-write, resident GPU layer sources, 256²
> tiles) are **renumbered/deferred after M43**; `docs/dev/canvas-compositing-plan.md`
> is left untouched, so its older M36–M38 references now point at this program's
> milestones and must be read as the perf tracks by name, not by number.
>
> **Interruption note (M38).** A user-requested milestone — the full CS6 toolbox
> icon/cursor library and the panel icons
> (`openspec/changes/m38-icon-cursor-library`, contract
> `docs/dev/m38-icon-cursor-library.md`) — takes **M38**, ahead of the panel
> stages. The panel stages therefore shift by one from M38 onward: panel
> anatomy → **M39**, filtering/search → **M40**, remaining management
> operations → **M41**, styles/effects → **M42**, smart objects / vector masks /
> artboards / layer comps → **M43**. M36 and M37 keep their numbers.

## 1. CS6 behavior summary

The panel lists "all layers, layer groups, and layer effects in an image"
(CS6 Help, *Layers panel overview*). This section summarises the researched
behaviour and cites the source for each claim; where the CS6 Help PDF is silent
the reconstructing source is named.

### Panel anatomy

- **Header strip**, left to right: blend-mode popup, **Opacity**, **Fill**
  (Photoshop Essentials, *Layers Panel Essentials*; Design Shack, *Master
  Guide*). The **lock strip** sits directly below the blend/opacity/fill strip,
  as four icons (Photoshop Essentials; Design Shack).
- **Filter/search row** at the very top (CS6-new): filter-type popup, criterion
  control, and an on/off **light-switch toggle** on the right (Photoshop
  Essentials; Design Shack; `LAY-032`).
- **Row**: eye column, preview thumbnail, name, color label, mask/vector-mask
  thumbnails with a link icon, clipping indent/underline, fx expander and badge
  (`LAY-002`; `PAN-001`).
- **Bottom button strip**, left to right (seven buttons): **link layers, layer
  styles (fx), add layer mask, new fill or adjustment layer, new group, new
  layer, delete** (Design Shack, *The Buttons on the Bottom*).
- **Panel menu** (top-right fly-out) holds `Panel Options` and the remaining
  commands (`LAY-002`).

### Locks

- Four locks, left to right: **Lock Transparent Pixels, Lock Image Pixels, Lock
  Position, Lock All** (Design Shack, *Locks*; SitePoint, *Locking Transparent
  Pixels*).
- Semantics (Design Shack): transparent-pixels confines editing to opaque
  pixels; image-pixels prevents any pixel change but still allows move/resize and
  layer styles; position prevents move/resize but allows appearance edits; Lock
  All prevents painting, moving, layer styles, and even deletion, though stacking
  order can still change. A locked layer shows a small lock icon on the row; the
  Background is locked by default (Photoshop Essentials).
- Lock icons are only fully available on a pixel layer (Photoshop Essentials);
  the CS6 Help (via `LAY-002`) adds that type/shape layers force Lock
  Transparency + Lock Image on and cannot deselect them.
- **Shortcut disagreement.** `LAY-002`/`PAN-001` and SitePoint's FAQ say `/`
  toggles **Lock Transparent Pixels**; Design Shack says `/` toggles **Lock
  All**. The repo contract keeps `/` = Lock Transparency; the discrepancy is
  recorded in §5.

### Fill vs Opacity

- Opacity scales the whole rendered layer including styles; Fill scales only the
  layer's own pixels (Design Shack, *Opacity vs. Fill*; Photoshop Essentials;
  `LAY-002`; `LAY-010`).
- With a non-Normal blend mode, reducing Fill and reducing Opacity produce
  visibly different results — Fill changes the amount of source entering the
  blend, Opacity scales the blended result (Design Shack; corroborated by the
  Fill path in psd-tools' compositor,
  `psd_tools/composite/composite.py:1413`).
- **If a group is selected, only Opacity is available** (CS6 Help; `LAY-002`,
  `LAY-003`). Background and locked layers cannot change opacity; Fill likewise
  (CS6 Help; `LAY-002`, `LAY-001`).

### Color labels

- CS6 sets the label from the row's right-click context menu (CS6 Help; `LAY-002`,
  `LAY-001`). Palette: **Red, Orange, Yellow, Green, Blue, Violet, Gray** plus
  "None" (reconstructed in `LAY-032`; confirmed as the seven names by
  [frameandfocal](https://frameandfocal.com/post-processing/group-objects-photoshop)).
  The PSD enum in psd-tools is `0=None, 1=Red, 2=Orange, 3=Yellow, 4=Green,
  5=Blue, 6=Violet, 7=Gray` (`psd_tools/constants.py`, `SheetColorType`); CC adds
  8–11, which are out of CS6 scope.
- Input-path disagreement: some sources say right-click the layer row, others
  right-click the thumbnail/eye or use Layer Properties (CS5). CS6 Help (via
  `LAY-002`) is right-click the layer/group; §5 records it.

### Selection, linking, visibility

- Click selects; `Shift`-click contiguous; `Ctrl`/`Cmd`-click non-contiguous;
  `Ctrl`/`Cmd`-click **outside** the thumbnail selects, clicking the thumbnail
  loads transparency as a selection (`LAY-002`).
- **Alt/Option-click the eye** hides all other layers and remembers the prior
  visibility; a second Alt-click restores it; right-click the eye offers
  show/hide this or all ([Adobe Community idea
  thread](https://community.adobe.com/t5/photoshop-ecosystem-ideas/request-option-click-alt-click-the-eye-icon-to-display-only-the-contents-of-that-layer-or-group/idi-p/13047360);
  [Photoshop Training Channel](https://photoshoptrainingchannel.com/tips/displaying-layers-eye-icon);
  `LAY-002`). Dragging through the eye column toggles successive rows.
- Linked layers retain their relationship until unlinked; link via the link
  button (`LAY-002`).

### Filtering / search (CS6)

- A popup selects the dimension — **name, kind, effect, mode, attribute, color
  label** — then a criterion control, then a toggle (CS6 Help; `LAY-032`).
- Default dimension is **Kind**, whose criterion is a row of five multi-select
  layer-type icons: pixel, adjustment, type, shape, smart object; **Name** is a
  text field; the rest are value menus (Design Shack, *Search*; Photoshop
  Essentials, *The Layer Search Bar*; `LAY-032`).
- Filtering hides rows from the panel only — it does not change document
  visibility (Photoshop Essentials; `LAY-032`).

### Panel Options and defaults

- **Thumbnail size**: None / Small / Medium / Large. **Thumbnail contents**:
  Entire Document / Layer Bounds. **Expand New Effects**. Also **Add "copy" to
  Copied Layers and Groups** and **Use Default Masks on Fill Layers** (`PAN-001`;
  Design Shack, *Panel Options*; Photoshop Essentials, *Essential Layers Panel
  Preferences*).
- Design Shack shows the CS6 default dialog and recommends changing Thumbnail
  Contents to Layer Bounds and turning Expand New Effects off — i.e. the shipped
  defaults are **Entire Document** and **Expand New Effects on**. The
  thumbnail-size default remains unstated (`PAN-001` leaves it *inferred*).

### Keyboard (panel-relevant)

- `F7` toggles the panel; `00` / `Shift+00` set layer/fill opacity to 0% (CS6);
  `Ctrl/Cmd+J` duplicates a selected group; `Tab`/`Shift+Tab` navigate while
  renaming; `Alt`-click the fx arrow shows/hides all effects; double-click the
  name renames; double-click the mask thumbnail opens mask display options
  (`LAY-002`; `docs/02-ui-ux/keyboard-shortcuts.md`).

## 2. Current state

### 2.1 Model — `crates/pictura-core/src/lib.rs`

`Layer` (`:274–286`) carries: `name`, `rect: PsdRect`, `blend: BlendMode`,
`opacity: u8`, `clipping: bool`, `visible: bool`, `mask: Option<LayerMask>`,
`adjustment: Option<AdjustmentData>`, `channels: Vec<Channel>`,
`children: Vec<Layer>`, `is_group: bool`. `BlendMode` has the 27 layer modes plus
group `PassThrough` (`:106–215`). `LayerMask` (`:260`) has
`rect/default_color/disabled/flags/data`.

**Not modelled:** `fill`, lock flags, color label, link sets, layer kind
(type/shape/fill/smart-object/background), vector mask, layer styles/effects,
blend-if/advanced blending, layer comps, stable layer id, and per-node
expansion state. There is no `Node`/arena: the tree is nested `Vec<Layer>`.

### 2.2 Panel — `crates/pictura-app/cpp/panels/layers_panel.{h,cpp}`

- `LayersModel : QAbstractTableModel` (`:102–232`) with five columns:
  0 visibility (`CheckStateRole`), 1 thumbnail (`DecorationRole`), 2 name
  (editable), 3 mode, 4 opacity. `setData` handles visibility and rename.
- Header (`:234–252`): `QComboBox` with the 27 blend names→keys, and a
  `QSpinBox` 0–255 for opacity; both start disabled.
- `QTreeView` (`:255–264`) with `setRootIsDecorated(false)`,
  `setItemsExpandable(false)`, `SingleSelection`, 24 px icons.
- Four buttons (`:266–295`): `Add Adjustment` (menu of five adjustment kinds),
  `Delete Layer`, `Move Up`, `Move Down`.
- `refresh()` (`:354–389`) reads top-level layers **topmost-first** (`for i =
  count-1 .. 0`) into `LayerRow {index,name,kind,visible,blend,opacity,thumbnail}`.
- `syncControls()` (`:391–405`) enables blend/opacity for the current row only.
- **Not present:** fill, lock strip, color label, context menu, panel menu,
  filter row, Panel Options, group rows/expansion, indentation, mask/style
  badges, multi-selection, tooltips beyond `name (kind)`, drag-reorder, inline
  rename `Tab` navigation.

### 2.3 Bridge — `crates/pictura-app/src/cxxqt_object.rs`

`PictureView` layer surface: `layer_count:711`, `topmost_pixel_layer_index:718`,
`layer_name:726`, `layer_kind:732` (`"group"`/`"adjustment"`/`"pixel"`),
`layer_visible:741`/`set_layer_visible:745`, `layer_blend:767`/`set_layer_blend:773`
(4-byte PSD key), `layer_opacity:800`/`set_layer_opacity:804`,
`set_layer_name:824`, `move_layer:844` (adjacent swap), `layer_thumbnail:864`,
`add_adjustment` (`adjustment_layer:1883`), `remove_layer`. Every setter follows
the same shape: mutate `doc.layers[i]`, `recomposite()` (or
`refresh_region` for visibility), then `record(label)` (`:1672`) for undo.

**Not present:** fill, lock, color, link, duplicate, rasterize, merge, convert
background, group/tree commands, `layer_styles`, mask toggle, selection
multi-edit.

### 2.4 Codec — `crates/pictura-codec/src/lib.rs`

- `read_layer_section:327`, `read_layer_info:375`, `read_layer_record:436`.
  The additional-layer loop (`:515–557`) handles **`luni`** (Unicode name) and
  **`lsct`** (group marker + group blend key), and stores one adjustment key
  opaquely. Unknown tags are skipped (not preserved).
- `write_layer_info:739`, `write_record:792`, `write_extra:861` emit `luni`,
  the mask block, `lsct` for sections, and the opaque adjustment block.
- No `lspf`, `lclr`, `iOpa`, `lyid`, styles, vector-mask, or link blocks. The
  record's flags byte is written as `visible ? 0 : 0x02` (`:808`); the legacy
  transparency-protected bit is not used.

### 2.5 Compositor — `crates/pictura-render`

- CPU `blend_into` (`src/lib.rs:423–464`) applies `as_ = src_a × opacity × mask`;
  groups isolate/pass-through (`composite_layer:109`), adjustments transform the
  backdrop.
- GPU `SHADER` `Params` (`src/gpu.rs:302`) and `cs_main` (`:626–652`) mirror the
  same formula; `Gpu::dispatch` (`:986`) passes `opacity: u8` normalized to f32.
- No fill factor anywhere.

## 3. Gap analysis

Legend — **Model** = missing `pictura_core` field/kind; **Codec** = missing PSD
block; **Bridge** = missing `PictureView` method; **UI** = missing widget/behavior.
Stage is the milestone that closes it.

| Feature | CS6 behavior | Current | Model | Codec | Bridge | UI | Stage |
|---|---|---|---|---|---|---|---|
| Fill opacity | 0–255, pixels only, groups excluded | done (M36) | `Layer.fill` | `iOpa` | get/set | Fill spinbox | M36 |
| Lock flags | 4 toggles, forced type/shape locks | done (M36) | `LockFlags` | `lspf` | get/set-per-flag | 4-button strip | M36 |
| Color labels | 7 colors + None, right-click | done (M36) | `ColorLabel` | `lclr` | get/set | row context menu | M36 |
| New layer / group | New Layer, New Group buttons + Layer menu | **absent** | — | — | `add_layer`/`add_group` | buttons + menu | **M37** |
| Duplicate (incl. groups) | `Ctrl/Cmd+J`, menu, `" copy"` naming | **absent** | — | — | `duplicate_layer` | menu | **M37** |
| Group / Ungroup | wrap/ungroup in place, `Ctrl+G` | **absent** | — | — | `group_layer`/`ungroup_layer` | menu | **M37** |
| Group tree | expand/collapse, folder rows | flat top-level only | ids already nested; expansion state | — | tree accessors | `QAbstractItemModel` tree + delegate | M39 |
| Clipped-layer display | indent + base underline | absent | `clipping` exists | — | — | delegate | M39 |
| Mask thumbnail / link / clip / style badge | extra thumb + link icon + badges | absent | mask exists; link/vector/styles missing | mask `-2` exists; link/vector/styles missing | accessors | delegate | M39 (mask/clip), M42 (fx), M43 (vector) |
| Multi-selection | shift/ctrl, one command over N | `SingleSelection` | — | — | multi-id setters | selection model + commands | M39 |
| Bottom quick-action strip | 7 buttons | 6 buttons (link/fx/mask missing) | — | — | new commands | buttons | M39 |
| Solo visibility | Alt-click eye, restore | absent | — | — | snapshot/restore | eye handler | M39 |
| Inline rename Tab/Shift+Tab | next/previous while renaming | single-row edit | — | — | — | delegate/key handling | M39 |
| Panel Options | thumb size/contents, expand effects, copy, default masks | absent | — | — | — | dialog + persistence | M39 |
| Panel + row menus | full command set | absent | — | — | commands | `QMenu` | M39 |
| Tooltips incl. name | present | `name (kind)` | — | — | — | delegate | M39 |
| Filtering/search | 6 dims + toggle, view-only | absent | needs `color`/`lock`/effects | — | — | filter bar + proxy | M40 |
| Rasterize variants | Type/Shape/Fill/Mask/Layer/All | absent | kind + vector/fill content | various | command | menu | M41 |
| Merge / Flatten | Merge Down/Layers/Visible, Flatten | absent | — | — | commands | menu | M41 |
| Layer Via Copy/Cut | selection → new layer | absent | — | — | commands | menu | M41 |
| Convert Background | from/to Background | absent | `is_background`/kind | `B0` name convention | commands | menu | M41 |
| Link sets | link/unlink, link icon | absent | link-set id | unsourced | commands | icon/delegate | M41 |
| Delete hidden layers | `Layer > Delete > Hidden` | absent | — | — | command | menu | M41 |
| Layer styles / fx | fx menu, badge, effect rows, dialog, render | absent | `styles` | `lrFX`/`lfx2` + descriptors | commands | fx UI + rendering | M42 |
| Blend-If badge / advanced blending | badge when customized | absent | advanced-blending fields | `brst`/`knko`/blend ranges | accessors | delegate | M42 |
| Smart Objects | convert/edit/replace/rasterize | absent | smart-object kind | `SoLd`/`lnk2` | commands | panel + menus | M43 |
| Vector masks / clipping | thumb, density/feather, clip runs | `clipping` bool only | vector mask, clip run | `vmsk`/`vsms` | commands | delegate + Properties | M43 |
| Artboards | **not CS6** (non-goal) | — | — | — | — | — | M43 (non-goal) |
| Layer Comps | capture/apply, resource 1065 | shipped | `LayerComp` store | image resource 1065 | commands | panel | M43 |
| Type/shape kinds | forced locks, tool names | `kind` is pixel/adj/group | `NodeKind` | `TySh`/`vmsk` | `layer_kind` | delegate | M41/M43 |
| `00`/`Shift+00` | layer/fill opacity 0% | absent | — | — | — | shortcut | M39/M40 |
| Select All/Similar/Linked | menus | absent | — | — | commands | menu | M41 |
| Properties panel binding | contextual editor | placeholder panel | — | — | — | Properties panel | cross-cutting |
| 32-bit / Lab gating | mode-restricted modes/tools | 8-bit RGB/gray only | depth/mode gating | — | — | menus | deferred |

## 4. Staged plan

Ordering rationale: **M36 is the prerequisite** every later stage reads — the
tree rows need fill/lock/color roles, filtering needs color/lock, and management
operations need the fields. **M37** then lands the most basic panel action, which
was missing entirely: creating a layer or group, duplicating one, and wrapping or
unwrapping it. Nothing else in the panel can be exercised end-to-end on a fresh
document until a user can add a node, so creation comes before the structural
panel work. **M39** makes the panel structurally CS6 (a tree, selection, menus,
options). **M40** is view-only and depends on M39's model/proxy. **M41**
operations depend on M39's selection and tree and on M37's node creation. **M42**
is the largest visual payoff but needs M39's fx rows and M36's Fill (the
Fill-vs-Opacity distinction only becomes observable once effects exist). **M43**
is the deepest and last. Sizes are relative (S ≤ 1 week, M ≈ 1–2, L ≈ 2–4,
XL > 4, at this repo's pace).

### M36 — Layer attributes end-to-end (M, Core) — done

Implemented as `openspec/changes/m36-layer-attributes` (archived); the result is
recorded in `docs/dev/STATE.md`.

- `Layer.fill: u8`, `Layer.lock: LockFlags`, `Layer.color: ColorLabel`; update
  44 literal sites / 14 files; no new dependency.
- Effective alpha `opacity/255 × fill/255` on CPU **and** GPU, byte-identical at
  fill 255, ±1 LSB otherwise; groups ignore fill.
- PSD `lspf` (lock, 4-byte low bits 0x01/0x02/0x04), `lclr` (8-byte `H6x`),
  `iOpa` (1-byte fill), omitted at defaults; round-trip + psd-tools oracle.
- Bridge `layer_fill`/`set_layer_fill`, `layer_lock`/`set_layer_lock`,
  `layer_color`/`set_layer_color`, history-recorded; `layer_kind` reports
  Background.
- Panel Fill spinbox, 4-button lock strip, row color-label menu; group Fill
  disabled; Background/locked refused.

**Dependencies:** M1 (model/codec), M2 (compositor), M14/M20 (history/panel),
M31/M34 (region composite/composite coherence). **PSD-interop risk: medium** —
`lspf` Lock All encoding (0x07 vs 0x80000000) and `iOpa` payload length are not
fully sourced; defaults are safe.

### M37 — Layer creation and grouping (M, Core)

Proposed as `openspec/changes/m37-layer-creation`.

- `document_ops::layer_ops`: pure `add_layer(doc, above, name)`,
  `add_group(doc, above, name)`, `duplicate_layer(doc, index)`,
  `group_layer(doc, index)`, `ungroup_layer(doc, index)`, plus
  `next_layer_name(doc, prefix)`; no new dependency.
- A new layer is a document-sized transparent raster layer (channels
  `0/1/2/-1`), so the composite is unchanged; a new group is an empty `is_group`
  node. Insertion is directly above the selected layer, or the top of the stack
  when there is no selection.
- Deep duplicate (children/mask/adjustment/attributes) named `"<name> copy"`;
  group wraps the selected layer in place; ungroup splices children in order;
  groups are named `"Group N"`.
- Bridge `add_layer`/`add_group`/`duplicate_layer`/`group_layer`/
  `ungroup_layer`, each recompositing then recording one labelled undo state.
- Panel New Group / New Layer buttons before Delete; the five `Layer` menu
  leaves become implemented commands.

**Dependencies:** M36 (attributes on the cloned node), M14/M20 (history/panel),
M34 (composite-then-record). **PSD-interop risk: none** — in-memory document
nodes; no file-format change.

### M39 — Panel anatomy (L, Core)

Tree model for groups (expand/collapse, indentation, folder icon), clipped-layer
indentation + base underline, mask thumbnail + link icon + clip/style badges,
multi-selection (contiguous/non-contiguous) with one command per multi-edit, the
seven-button bottom quick-action strip, Alt-click solo visibility with exact
restore, inline rename with `Tab`/`Shift+Tab`, Panel Options (thumbnail
size/contents, Expand New Effects, Add "copy", Use Default Masks) persisted in
the session store, panel menu + row context menu, layer-name tooltips, and
drag-reorder with command validation.

**Dependencies:** M36 (attributes as row roles), M37 (nodes to display),
M23/M24 (chrome/rail). **PSD-interop risk: low** — mostly view state;
link/mask flags serialise later. The multi-selection work here upgrades M37's
single-layer Group/Ungroup/Duplicate to per-selection operations.

### M40 — Layer filtering/search (M, Core)

The six CS6 dimensions (name, kind, effect, mode, attribute, color label) behind
a `QSortFilterProxyModel` and the on/off toggle, with ancestor promotion; kind
is a multi-select icon row; name is a text field; effect/mode/attribute/color
are value menus. View-only (no history, no serialization). Effect/attribute
criteria that need FX/advanced-blending data degrade until M42 (matching
`LAY-032`'s CC-only exclusions).

**Dependencies:** M39 (tree + model roles), M36 (color/lock values).
**PSD-interop risk: none** — transient view state.

### M41 — Layer management operations (L, Core)

The remaining structural operations: rasterize variants (Type/Shape/Fill
Content/Vector Mask/Smart Object/Video/Layer/All Layers), merge/flatten
(`Merge Down`/`Layers`/`Visible`/`Clipping Mask`, `Flatten Image`), Layer Via
Copy/Cut, Convert Background / Background From Layer (needs the M36 Background
detection to become a first-class flag), Select All / Similar / Linked, link
sets, Delete Hidden Layers, and the New Layer/Group **dialogs** (neutral-color
fill, use-previous-as-clipping) that M37 deliberately ships without.

**Dependencies:** M36, M37, M39. **PSD-interop risk: low–medium** — structural
operations that recompute the composite; link-set serialization is unsourced
(`LAY-002` open question).

### M42 — Layer styles / effects (XL, Core)

The `fx` menu and badge, effect child rows in the tree, the Layer Style dialog
for the seven effect families, effect rendering as compositor passes,
`Rasterize Layer Style` / `Create Layers`, Global Light, Scale Effects, and the
Styles preset panel (`.asl`). Advanced Blending + Blend-If + the "customized"
badge. This is when `Fill` must move **after** the effect passes and `Opacity`
must scale the effect result — a formal dependency back on M36's compositor.

**Dependencies:** M36 (fill/opacity split), M39 (fx rows). **PSD-interop risk:
high** — `lrFX` plus the extended descriptor blocks (`lfx2`, `vscg`, `vogk`
family) are only partly catalogued; unknown keys must be preserved.

### M43 — Smart objects, vector masks, artboards, layer comps (XL, Core + Extended/non-goal)

Smart Objects (embedded; convert/edit/replace/export/rasterize; `SoLd`/`lnk2`),
vector masks and clipping masks (thumb, density/feather, clip runs, `vmsk`/
`vsms`), layer comps (capture/apply, image resource 1065, caution states), and
linked/embedded object handling (`lnkD`/`lnk3`; linked is post-CS6, preserved
only). **Artboards are a non-goal** for CS6 parity (`docs/05-layers/artboards.md`).

**Dependencies:** M36–M42. **PSD-interop risk: high** — SoLd/lnk2/vmsk/1065
descriptors and layer identity (`lyid`, resource 1044) are only partly
documented.

## 5. Risks and open questions

Carried forward from `LAY-002` / `PAN-001`:

- **Filter UI details.** The CS6 Help names the six dimensions and the toggle
  but not every criterion editor. Reconstructed from Design Shack/Photoshop
  Essentials: Kind = five icons (multi-select), Name = text, Effect/Mode/
  Attribute/Color = menus. Attribute/Effect sub-lists remain unverified.
- **Link-set persistence in PSD.** Whether CS6 stores link relationships in a
  block or transiently is unsourced. Blocks the M41 link-set serialization.
- **Color-label palette coordinates.** The exact RGB of each label is not
  sourced; only the seven names and the 0–7 PSD values.
- **Rasterize Layer Style semantics.** Whether it bakes only effects or also
  clears the style list, and its undo granularity, is undocumented. M42.
- **Panel Options defaults.** Thumbnail contents = Entire Document, Expand New
  Effects = on, Add "copy" = on, Use Default Masks = on are sourced; the
  thumbnail-size default is still not stated.

New from this research:

- **`lspf` Lock All encoding.** psd-tools uses `ProtectedFlags.COMPLETE =
  0x80000000`, while the spec text and the model use three low bits. M36 reads
  the low bits and writes `0x07`; a CS6 multi-lock PSD is needed to settle it.
  PSD-interop risk for M36.
- **`iOpa` payload length/presence.** psd-tools reads a 1-byte `ByteElement`;
  an ag-psd issue reports 4 bytes (byte + 3 pad). M36 reads the first byte and
  writes 1 byte. Whether CS6 always writes `iOpa` (even at 255) is unknown, so
  "byte-identical to a CS6 save" is not yet claimable — only "byte-identical to
  the pre-change codec" is.
- **Group `iOpa`.** CS6 exposes no group Fill; whether it writes a meaningful
  group fill or a constant is open (`LAY-003`). M36 preserves and ignores it.
- **Background identification.** M36's `index 0 && name == "Background"`
  heuristic is a ceiling; a first-class `is_background`/`NodeKind::Background`
  lands in M39/M41. Type/shape forced locks need layer kinds too.
- **Expansion state persistence.** Whether CS6 persists group expand/collapse is
  not sourced; M39 chooses session state (matching workspace persistence) unless
  a preference dump says otherwise.
- **M37 insertion sentinel and group blend.** An out-of-range or negative
  `above` inserts at the top (including the no-selection `-1`), so there is no
  way to insert below the bottom layer; CS6's New Layer buttons never need that.
  A new group is stored with a `Normal` blend rather than `PassThrough`; the
  choice is invisible while the group is empty and may need revisiting when a
  group-with-children parity baseline exists.
- **Perf-track numbering collision.** The deferred canvas-perf tracks are
  labelled M36–M38 in `docs/dev/STATE.md`; this program reuses those numbers.
  `docs/dev/canvas-compositing-plan.md` is frozen and still uses them.
- **Layer identity.** Comps/link sets/styles ultimately need a stable layer id
  (`lyid`, resource 1044); the model has none. Needed by M43 (and comps).
- **`write_psd` unknown-tag preservation.** The codec currently drops unknown
  additional-layer tags; CS6 files with styles/vector masks/link blocks will not
  round-trip until those tags are either modelled or preserved opaquely (M42/M43).

## 6. Sources

Fetched for this document:

- CS6 Help reference
  `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  cited through `docs/05-layers/*` and `docs/02-ui-ux/panels/layers-panel.md`
  (fetched with `curl`, extracted with `pdftotext -layout` in the repo's earlier
  research). The most recent direct fetch of the colour-label palette named the
  seven CS6 colors.
- `https://designshack.net/articles/software/the-master-guide-to-the-photoshop-layers-panel`
  (fetched, `webfetch`) — the seven bottom buttons and their order; the four
  locks and their order/semantics; the filter default Kind + five kind icons +
  the on/off switch + the dimension list; Panel Options (Entire Document /
  Expand New Effects on / Add "copy" on); Opacity-vs-Fill.
- `https://www.photoshopessentials.com/basics/layers/layers-panel/` (fetched) —
  header order blend→Opacity→Fill; the four lock icons below the blend mode;
  the lock icon on a locked layer; the filter bar at the top, defaults to Kind,
  five icons, multi-select, and the light-switch toggle; Panel Options thumbnail
  size (None/Small/Medium/Large).
- `https://www.sitepoint.com/photoshop-tip-locking-transparent-pixels/` (fetched)
  — lock order left-to-right; `/` toggles Lock Transparent Pixels.
- `psd-tools` 1.19 source, installed as the codec oracle
  (`/usr/lib/python3.14/site-packages/psd_tools`): `constants.py`
  (`SheetColorType` 0–11; `ProtectedFlags` 0x01/0x02/0x04/0x08 + `COMPLETE
  0x80000000`; `BLEND_FILL_OPACITY = b"iOpa"`), `psd/layer_and_mask.py`
  (`ProtectedSetting` flags and `SheetColorSetting` `H6x`),
  `psd/tagged_blocks.py` (`BLEND_FILL_OPACITY: ByteElement`),
  `api/layers.py` (`fill_opacity`, `sheet_color`, `lock`),
  `composite/composite.py:1413` (fill folds into source alpha).
- `https://docs.aspose.com/psd/net/list-of-psd-layer-resources/` (fetched) —
  layer-resource key catalogue: `lspf` (protected setting), `lclr` (sheet
  color), `clbl`/`infx`/`knko`, `lsct`, fill keys, `vmsk`/`vsms`, `lnkD`/`lnk2`.
- `https://github.com/iamgqb/psd-spec-translate/blob/master/The%20Photoshop%20File%20Format/6.Layer%20and%20Mask%20Information%20Section.md`
  (fetched) — a translation of the Adobe layer/mask spec: `lspf` 4 bytes with
  bits 0–2 transparency/image-pixels/position; `lclr` 8 bytes; the
  additional-layer-info block structure.
- `https://community.adobe.com/t5/photoshop-ecosystem-ideas/request-option-click-alt-click-the-eye-icon-to-display-only-the-contents-of-that-layer-or-group/idi-p/13047360`
  and `https://photoshoptrainingchannel.com/tips/displaying-layers-eye-icon`
  (fetched) — Alt-click the eye solos the layer; a second click restores.
- `https://photoshopessentials.com/basics/layers/essential-layers-panel-preferences`
  and `https://frameandfocal.com/post-processing/group-objects-photoshop`
  (fetched) — Panel Options and the seven color-label names.

Could not fetch / not used:

- `https://helpx.adobe.com/photoshop/using/layers.html` — **HTTP 403**. Modern
  Adobe Help pages are inaccessible; the archived CS6 Help PDF and the repo's
  cited extracts are used instead.
- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` was
  not re-downloaded in this pass; its claims reach this document through the
  repo's existing `## Sources` sections.
- `https://www.adobe.com/devnet-apps/photoshop/fileformatashtml` was not fetched
  directly; the `lspf`/`lclr`/`iOpa` layouts come from the psd-tools source and
  the spec translation above.
- SearXNG URL reads of Design Shack and Photoshop Essentials returned
  `301 Moved Permanently`; the `webfetch` tool followed the redirects to the
  final pages cited above.

## 7. Verification of this deliverable

- `openspec validate m37-layer-creation --strict` and
  `openspec validate --all --strict`.
- `git status --porcelain` shows the `docs/` changes and the new
  `openspec/changes/m37-layer-creation/` directory.
- The M37 implementation is verified by `cargo test --workspace`,
  `cargo fmt`/`clippy`, `cmake --build build`, and the app self-test's
  `m37_create` step (exit codes 90–94).
