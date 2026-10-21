# Layers Panel Filtering and Layer Search

- **Spec ID:** `LAY-032`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `Yes` — the CS6 Help's "Layers enhancements" section calls the filtering/search row at the top of the Layers panel new in CS6, listing filtering "based on name, kind, effect, mode, attribute, or color label". The later CC-only filter types (`Smart Object`, `Artboard`) are out of scope for CS6 parity; the **`Selected`** type is shipped by decision as the **isolation-mode** entry point (see below), even though the CS6 Help does not list it.
- **Depends on:** `05-layers/layers-overview.md`, `05-layers/layer-management-ui.md`, `05-layers/layer-groups.md`, `05-layers/artboards.md`, `05-layers/layer-styles.md`, `05-layers/blend-modes.md`, `02-ui-ux/panels/layers-panel.md`, `ARCH-008` document-model, `ARCH-003` qt6-ui-design

> All module and type names below are **design proposals**. The repository
> contains a working implementation; this file stays the behavioural contract
> for the feature, and `docs/dev/doc-code-parity-gaps.md` records what shipped
> and what did not. The exact CS6 sub-lists (which effects, modes, and
> attributes appear) are not fully enumerated in the fetched Help and are marked
> *(inferred)* where reconstructed from later-version community sources.

## CS6 behavior

CS6 adds a **filter/search row** at the top of the Layers panel. The user picks
a **filter type** from a pop-up menu, supplies or picks the **filter criteria**,
and flips a **toggle switch** to turn filtering on or off. While filtering is
on, the panel displays only the layers that match; all other layers are hidden
from the panel (not from the document). Turning the toggle off restores the full
list.

The six CS6 filter types are:

| Filter type | Criteria control | Matches |
|---|---|---|
| **Kind** | A row of five buttons (multi-select) | Pixel, Adjustment, Type, Shape, Smart Object layers |
| **Name** | Free-text field | Layer/group names containing the typed string |
| **Effect** | Drop-down of layer-style effects | Layers carrying the chosen effect (Drop Shadow, Bevel & Emboss, etc.) |
| **Mode** | Drop-down of blend modes | Layers with the chosen blend mode (Multiply, Screen, etc.) |
| **Attribute** | Drop-down of layer attributes | Layers with the chosen state/characteristic (e.g. Visible, Locked) |
| **Color** | Drop-down of layer color labels | Layers assigned the chosen label color |

A seventh type, **`Selected`**, is shipped by decision (it is CC-era, not in the
CS6 Help): it has no criteria control, and shows only the **isolated set** — the
layers captured when isolation mode was entered — plus their group ancestors.
It is the mechanism behind **isolation mode**; see `## Isolation mode`.

The default filter type is **Kind** *(per a CS6-era guide)*. The Kind buttons
are combinable (select more than one kind at once). Effect and Mode act as
single-choice drop-downs *(inferred)*.

The Help frames this as "help you find key layers in complex documents quickly"
and notes that the **Properties panel** can then modify the components selected
in the Layers panel.

### Selecting by attribute

CS6 has no dedicated "select all layers matching X" command. The documented
workflow is: enable a filter so only matching layers are listed, then
`Select > All Layers` (`Ctrl+Alt+A` / `Cmd+Option+A`) to select the shown
subset, or click/`Shift`-click/`Ctrl`-click within the filtered list. The
filter is a display predicate; selection is a separate model state. The later
CC "Select Layers by Attribute" command is **not** CS6 scope.

### Scope exclusions (CC-only)

The `Smart Object` (linked-up-to-date / out-of-date / missing / embedded) and
`Artboard` filter types belong to later Photoshop releases and are documented
only in post-CS6 sources. They are **not** part of this spec; see
`## Open questions`. The `Selected` type is the exception: it is shipped by
decision as isolation mode, even though it too is CC-era.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Layers panel filter row (top) | Inline UI | n/a | Filter-type pop-up, criteria control, on/off toggle |
| Layers panel filter-type pop-up | `QComboBox`-like menu | n/a | Kind, Name, Effect, Mode, Attribute, Color, Selected |
| Layers panel Kind buttons | Toggle buttons | n/a | Pixel, Adjustment, Type, Shape, Smart Object; multi-select |
| Layers panel Name field | Text field | n/a | Substring search over layer/group names |
| Layers panel Effect drop-down | Menu | n/a | Layer-style effect list |
| Layers panel Mode drop-down | Menu | n/a | Blend-mode list |
| Layers panel Attribute drop-down | Menu | n/a | Layer state/characteristics |
| Layers panel Color drop-down | Menu | n/a | Layer color labels |
| Layers panel Selected type | (no criteria) | n/a | Shows the isolated set; entering it is isolation mode |
| Layers panel filter toggle | Switch | n/a | On (accent color) / off (gray) |
| Layer row context menu | Menu | n/a | `Release from Isolation` while isolated |
| Canvas (Move tool) right-click | Menu | n/a | `Isolation Mode` enters on the current selection |
| `Window > Layers` | Menu | `F7` | Opens the panel |
| `Select > All Layers` | Menu | `Ctrl+Alt+A` | Combines with an active filter to select all matches |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Filter type | enum | `Kind` *(per CS6-era guide)* | Kind, Name, Effect, Mode, Attribute, Color, Selected | Six CS6 options (Help) plus the Selected isolation type |
| Filter enabled | bool | off | on / off | Toggle switch |
| Kind selection | bitset | none | Pixel, Adjustment, Type, Shape, Smart Object | Multi-select |
| Name query | string | "" | free text | Substring match *(inferred)*; case sensitivity unverified |
| Effect criterion | enum | first entry | CS6 layer-style effect list | Drop Shadow, Bevel & Emboss, etc. |
| Mode criterion | enum | `Normal` *(inferred)* | 27 CS6 blend modes (groups: Pass Through) | From `05-layers/blend-modes.md` |
| Attribute criterion | enum | `Visible` *(inferred)* | e.g. Visible, Locked, (Lock transparent, Lock position, Empty, Linked, Clipped, Mask, Effects, Advanced Blending — see Open questions) | CS6 sub-list not fully sourced |
| Color criterion | enum | none | Red, Orange, Yellow, Green, Blue, Violet, Gray (plus "None") | CS6 layer label colors |
| Isolated set | path set | empty | panel paths | Captured on entry; shrinks through `Release from Isolation` |

## Algorithms & pipeline

Layer filtering is a **view-level predicate**, not a document mutation.

1. Represent the panel's full layer tree (`LayersModel`, `ARCH-008`) and a
   `LayerFilter { enabled, kind: KindFilter, name: Option<String>, effect:
   Option<EffectKind>, mode: Option<BlendMode>, attribute: Option<Attribute>,
   color: Option<LabelColor>, selected: bool, isolated: Set<Path> }`.
2. A layer `matches(filter)` returns true when **every active criterion**
   matches (conjunction). Kind accepts any selected kind (disjunction within the
   Kind set).
3. Visibility rules for the filtered view:
   - A matching layer is shown.
   - A group/artboard ancestor of a matching descendant is shown so the
     hierarchy is navigable, but a non-matching ancestor itself is not a match.
   - Non-matching siblings are hidden.
   - A collapsed group with a matching descendant is auto-expanded, or shown
     collapsed with an indicator *(inferred — CS6 behavior unverified)*.
4. Apply the filter through a model proxy so the underlying document model is
   untouched; toggling it off restores the full tree with no state loss.
5. Re-evaluate incrementally on `Node` changes (rename, blend-mode change,
   effect add/remove, visibility/lock toggle, color label, kind change via
   rasterize/convert) so the filtered view stays live.

The name comparison is likely case-insensitive substring matching, but the
exact CS6 rule (case, wildcards, anchoring) is unverified.

Because filtering is view-only, it creates **no history state** and is not
saved with the document. A CS6-era source reports that **Actions cannot record
layer filtering** (recording a filter adds nothing to the Action), which
supports the view-only model.

## Isolation mode

**Isolation mode** is the `Selected` filter type, shipped by decision (CC-era,
absent from the CS6 Help). It focuses the panel and the Move tool on one set of
layers:

- **Enter.** Choosing `Selected` with the toggle on, or the Move tool's canvas
  right-click **`Isolation Mode`**, captures the current selection as a fixed
  **isolated set**.
- **While isolated.** The panel shows only the isolated layers and their group
  ancestors. The Move tool's **Auto-Select** hit test ignores layers outside the
  set, so a press picks the topmost isolated layer under the point (or, in Group
  mode, its nearest isolated ancestor) and keeps the active layer when no
  isolated layer covers it. Selecting a shown row does not change the set.
- **Release.** The layer row context menu offers **`Release from Isolation`**,
  which removes that row from the set; releasing the last row ends isolation.
- **Exit.** Turning the filter toggle off, choosing another dimension, pressing
  **Escape**, or switching documents ends isolation.

Isolation is view-only: it never mutates the document, adds no history state,
and is not serialized. Adding a layer to the set from the canvas (the CC
workflow) is a ceiling — only Release shrinks the set.

## Rust module mapping

Proposals:

- `pictura_layers::filter` — `LayerFilter { enabled: bool, kind:
  KindFilter, name: Option<String>, effect: Option<EffectKind>, mode:
  Option<BlendMode>, attribute: Option<Attribute>, color: Option<LabelColor> }`,
  `matches(&Node, &LayerFilter) -> bool`.
- `pictura_layers::filter::kind` — `KindFilter(u8)` bitset over
  `{ Pixel, Adjustment, Type, Shape, SmartObject }`.
- `pictura_layers::filter::attribute` — `Attribute { Visible, Locked,
  LockTransparent, LockPosition, Empty, Linked, Clipped, HasMask, HasEffect,
  AdvancedBlending }` (define the CS6 set before implementing).
- `pictura_core::node` — the fields the predicate reads (`kind`, `name`,
  `visible`, `lock_flags`, `blend`, `styles`, `mask`, `clipping`, `color_label`)
  already exist; add a `color_label` field if absent.
- `pictura_layers::filter::view` — `visible_rows(tree, filter) -> Vec<NodeId>`
  with ancestor promotion for tree display.

Filtering must not allocate pixel data; it reads node metadata only, so it is
cheap even on large layer counts.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `LayersFilterProxyModel` | `QSortFilterProxyModel` | Predicate over `LayersModel`; `filterAcceptsRow` + ancestor promotion; live updates |
| `LayerFilterBar` | `QWidget` | Hosts the filter-type `QComboBox`, the criteria stack, and the on/off toggle |
| `LayerFilterTypeCombo` | `QComboBox` | Kind / Name / Effect / Mode / Attribute / Color |
| `LayerKindButtonGroup` | `QButtonGroup` | Five checkable buttons, multi-select; exclusive to the Kind filter type |
| `LayerFilterCriteriaStack` | `QStackedWidget` | Swaps the criteria widget for the selected filter type |
| `LayerFilterToggle` | `QToolButton` (checkable) | Enables/disables the proxy; accent vs gray state |
| `LayerColorMenu` / `LayerEffectMenu` / `LayerModeMenu` | `QMenu` | Criteria menus sourced from the shared style/blend-mode registries |

`QSortFilterProxyModel` is the correct primitive: it keeps the document model
authoritative and makes the filter a pure view concern. The five Kind buttons
map to a bitset criterion. Colors come from the same registry the layer context
menu uses (`05-layers/layer-management-ui.md`).

## Data-model impact

- **No document mutation.** Filtering changes only panel view state, so there is
  no undo record and nothing to serialize to PSD/XMP.
- **Transient UI state.** Best treated as session state (like panel scroll
  position), optionally restorable via workspace layouts
  (`02-ui-ux/workspace-and-docks.md`); whether CS6 persists it is unverified.
- **New node field needed:** `color_label: Option<LabelColor>` on `Node`
  (color labels are a CS6 Layers-panel attribute; `05-layers/layer-management-ui.md`).
- **Selection interplay.** `Select > All Layers` after filtering selects the
  shown `NodeId`s; selection is model state and *is* undo-relevant
  (`ARCH-009`), so the combination produces one selection state.
- **Model notifications.** Filter changes emit proxy layout signals, not
  document `dataChanged`; predicate-relevant node edits must invalidate the
  proxy so the visible subset stays correct.

## Edge cases

- **No matching layers** — panel shows an empty list (or an empty-state
  message); the document is unaffected.
- **Single-layer / empty document** — filter is a no-op.
- **Hidden/ancestor layers** — a matching layer inside a collapsed/hidden group
  must still be reachable; define whether hidden ancestors are revealed.
- **Groups** — group name, color, blend mode, and effects are themselves
  filterable; a group matches on its own attributes even if no child matches
  *(inferred)*.
- **Adjustment/fill layers** — count as `Adjustment` kind; they have no pixel
  content but are valid Kind matches.
- **Layers with a mask** — `HasMask` attribute must consider both raster and
  vector masks.
- **Empty layers** — `Empty` attribute means no pixels and no content bounds
  *(inferred)*.
- **Linked layers** — `Linked` attribute reflects the layer link flag.
- **Name query with regex/wildcards** — treat as literal text unless CS6
  behavior proves otherwise.
- **Case sensitivity and whitespace** — unverified; define before parity.
- **Filter active while the user edits** — a layer edited out of the current
  predicate must disappear from the view without losing selection coherence.
- **Filter active during a merge/delete/rasterize** — the proxy must handle
  rows vanishing underneath it.
- **Artboards (CS6)** — artboards are layers; Kind has no Artboard button in
  CS6, so an artboard matches on its general attributes only.
- **Localization** — Kind/Attribute labels and color names are localized;
  matching is structural, never on display strings.
- **Huge layer counts (thousands)** — predicate evaluation must be O(n) per edit
  or incrementally cached; no per-layer allocations.
- **Actions** — filtering must be inert to Action recording (view-only).

## Parity acceptance criteria

- Given a document with pixel, adjustment, type, shape, and Smart Object layers,
  enabling `Kind > Type` shows only type layers (and any ancestor groups).
- Given `Kind` with Type + Shape selected, both are shown.
- Given a name query, only layers/groups whose names contain the string are
  shown.
- Given `Mode > Multiply`, only layers set to Multiply are shown.
- Given `Effect > Drop Shadow`, only layers carrying that effect are shown.
- Given `Color > Red`, only red-labeled layers are shown.
- Given `Attribute > Visible`, only visible layers are shown (hidden layers
  excluded).
- Given a filter that matches nothing, the panel is empty and the document is
  unchanged; toggling the filter off restores every layer.
- Given filtering on, `Select > All Layers` selects exactly the shown layers.
- Given a layer rename or blend-mode change while a filter is active, the layer
  appears/disappears from the filtered view accordingly.
- Given filtering, no history state is added, and the operation is not
  recordable by Actions.
- Given isolation entered on two selected layers, only those layers (and their
  ancestors) show, the Move tool's Auto-Select picks only among them, releasing
  one hides it, and releasing the last restores every layer with no history
  state.
- Given isolation active, `Escape`, the filter toggle off, and a document switch
  each end isolation and unscopes Auto-Select.
- Given a filtered view, closing and reopening the document starts unfiltered
  (if CS6 does not persist it) or restores the last filter (if it does) — the
  chosen behavior is covered by an Open question.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  official Photoshop CS6 Help. Established: "Filter layers (CS6)" is new; the
  panel displays a subset "based on name, kind, effect, mode, attribute, or
  color label"; the three-step procedure (choose filter type, select/enter
  criteria, flip the toggle switch); "Properties panel to quickly modify the
  layer components selected in the Layers panel"; color layers/groups by
  right-click in CS6 (page 157).
- `https://crossmed.blogspot.com/2013/05/filter-layers-in-cs6.html` — CS6-era
  review. Established: six filter types; Kind exposes five buttons (pixel,
  adjustment, type, shape, smart objects); Name uses a text box; Effect, Mode,
  Attribute, Color use pop-up menus; filtering hides non-matching layers.
- `https://designshack.net/articles/software/the-master-guide-to-the-photoshop-layers-panel`
  — CS6-era 12-minute guide. Established: default search/filter type is Kind;
  Kind presents pixel/adjustment/type/shape/smart-object icons; a small switch
  turns filtering on and off.
- `https://photoshoptrainingchannel.com/tips/searching-for-layers` — tip page.
  Established: "In CS6 or newer, you can search for layers in the Layers panel
  by layer kind, layer name, effect, mode, attribute, color, or smart object."
- `https://bjango.com/articles/layertags` — CS6-era article. Established:
  Photoshop CS6 introduced layer searching; filter by layer color and by layer
  name tags; effect and visibility ("Not Visible") searches; Actions cannot
  record layer filtering.
- `https://blog.yarsalabs.com/how-to-use-layer-filters-in-photoshop` —
  later-version (CC) guide. Established post-CS6 filter types (`Smart Object`,
  `Selected`, `Artboard`) and richer Kind/Attribute lists; used **only** to
  identify CC-only scope and to reconstruct the likely Attribute set, not as a
  CS6 source.
- `https://simonabrams.com/blog/2013/8/8/photoshop-quick-tips-isolation-mode`
  and `https://creativepro.com/using-isolate-layers-photoshop-cc/` — CC-era
  isolation-mode guides. Established: the `Selected` filter type (or the Move
  tool's canvas right-click `Isolation Mode`) isolates the selected layers; the
  panel shows only those layers; `Release from Isolation` in a layer's row menu
  removes it from the set; clearing the filter exits. The CS6 Help does not
  mention isolation, so it is shipped by decision rather than CS6 parity.

## Open questions

- **Exact CS6 Attribute entries.** Which of Visible, Locked, Lock Transparent,
  Lock Position, Empty, Linked, Clipped, Mask, Effects, Advanced Blending exist
  in CS6? The fetched Help does not enumerate them and the detailed lists come
  from a CC-era source. Resolve from a CS6 screenshot or the CS6 manual's
  Attribute menu.
- **Exact CS6 Effect and Mode lists.** Are all 27 blend modes and every effect
  selectable, or a subset? Resolve from a CS6 screenshot.
- **Kind button exact set.** Five buttons (Pixel/Adjustment/Type/Shape/Smart
  Object) per CS6-era sources; confirm CS6 does not also include Group/Artboard.
- **Name matching rules.** Case sensitivity, substring vs prefix, wildcards, and
  handling of spaces are unverified.
- **Group/ancestor display rule.** How a matching descendant inside a
  collapsed/hidden group is surfaced in CS6 is unverified.
- **Attribute multi-select.** Can several Attribute values be active at once, or
  is it single-choice like Effect/Mode?
- **Persistence.** Does CS6 save the active filter with the panel/workspace, or
  reset it on reopen?
- **Selection linkage.** Whether CS6 has any command that selects all filtered
  layers directly (beyond `Select > All Layers`), and whether `Select > All
  Layers` respects the filter, needs a CS6 confirmation.
- **`color_label` in the document model.** Confirm how layer color labels are
  stored in CS6 PSD (image resource vs layer record) so the data-model field is
  placed correctly; see `05-layers/layer-management-ui.md`.
