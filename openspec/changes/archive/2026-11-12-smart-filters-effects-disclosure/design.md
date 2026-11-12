# Design

## Context

See `proposal.md` — Why. The Layers tree builds synthetic group nodes under each
styled or filtered layer (`layer/@fx`, `layer/@sf`) and lets the delegate paint
them like layer groups: a left chevron slot, an fx thumbnail card, a fixed left
eye gutter on every row, and an `eyeOff` glyph for hidden rows. The list is a
second tree level: the layer row expands to reveal the group, and the group row
expands to reveal its children. The `fx` badge on the layer row also toggles the
layer row.

The new layout needs one list toggle on the layer row, the list as one nested
block inside the layer's cell, and a compact, gutter-aligned presentation. The
existing tree shape stays; only the presentation, expansion bookkeeping, and
selection routing change.

## Goals / Non-Goals

**Goals:**

- One visual block per layer: the left eye cell and the right badge/toggle cell
  span the layer row and its synthetic list, with no horizontal rules between
  the list rows or between the Effects and Smart Filters groups.
- A layer exposes exactly one list toggle. Opening it shows every synthetic
  group the layer owns; closing it removes them all. The toggle points down when
  closed and up when open.
- The synthetic rows have no thumbnail, no disclosure, and an eye aligned to the
  owning layer's image thumbnail rather than indented per depth; a hidden row
  paints the eye-off glyph at 50% opacity.
- Clicking a synthetic row outside its eye selects and highlights the owning
  layer.
- A group (folder) row keeps its left nested-layers chevron, independent of the
  list toggle.

**Non-Goals:**

- No change to the document model, bridge APIs, PSD format, or filter/style
  semantics.
- No change to the smart-filter drag/reorder behavior or the layer-style menu.
- No new icon asset: the up state reuses the existing disclosure-down asset,
  flipped vertically.
- No change to the regular layer rows' nesting indentation or the separator.

## Decisions

**D1 — One toggle controls both lists.**
The layer row paints a single right-edge toggle when it has effects, smart
filters, or both. Opening it shows every synthetic group the layer owns; closing
it hides them all. The toggle sits immediately left of the lock badge and right
of the fx badge. The fx badge click keeps toggling the combined list. Alternative:
keep separate effects and smart-filter toggles. Rejected: the two chevrons
compete visually and a layer with both lists has no single disclosure state.

**D2 — One per-layer expansion set drives model inclusion.**
Replace `effectsExpanded_` and `filtersExpanded_` with `QSet<QString>
listsExpanded_`. `refresh()` includes a layer's synthetic groups only when the
set contains the layer path; closing the list removes them all from the model.
`Expand New Effects` and a first-seen smart filter both insert the layer path,
so either auto-open reveals the combined list. The layer row's Qt expansion
still controls the block's visibility; the group rows are force-expanded when
included. Alternative: keep two sets and synchronise them. Rejected: two sets
are one more place for the combined state to drift.

**D3 — The layer row's left chevron is only for nested layers.**
`row.expandable` stays true for group layers (nested layers) and false for
pixel/fill/adjustment/type/smart-object layers. A smart object no longer sets
`expandable` for its filters list; the right toggle replaces it. The group row
is not `expandable`. This keeps the folder chevron and the list toggle
independent: a group with effects can collapse its nested layers without
changing the list state, and vice versa.

**D4 — Nested geometry is anchored to the image thumbnail, one indent step per
child.**
`nestedEyeColumn(index)` walks to the owning layer row and returns
`imageSlotRight(itemRect, layerIndex)` — the leftmost image thumbnail's right
edge, before any mask or vector thumbnail. The synthetic eye column is the eye
glyph width plus 2 px (18 px at the default row height), and the group header
starts at that edge while each child adds one `kIndent` step
(`nestedLevel * kIndent`). The eye is centred in that narrow column, and the
existing half-name gap leaves a 6 px visible eye-to-name gap, half the previous
12 px. Alternative: keep every row on one x. Rejected: the child rows read as a
second level under their group header, and the tests assert the step.

**D5 — The block's cells are painted per row, with one closing rule.**
The delegate paints the fixed left cell (flat gutter background + 1 px right
border) for the layer row and for every included synthetic row, even though the
synthetic eye is in the content column. The layer row's bottom rule is
suppressed while its list is open. A synthetic row suppresses its bottom rule
when the next visible row in the view is also synthetic; the last synthetic row
draws the block's closing rule. This removes the separator between the Effects
and Smart Filters groups while keeping a single block boundary. Alternative:
compare next siblings only. Rejected: the last child of one group and the next
group's header are not siblings, so the old check leaves a rule between the
groups.

**D6 — Hidden rows paint the eye-off glyph at 50% opacity.**
`paint()` draws `layers.eyeOn` when `VisibleRole` is true and `layers.eyeOff` at
50% opacity otherwise. The eye rect remains the hit target for every row. This
applies to layer rows and synthetic rows.

**D7 — The up chevron reuses the down asset.**
The list toggle draws `layers.disclosureDown` when closed and the same pixmap
flipped vertically when open. No new icon id, no `ui/icon-assets` spec delta.

**D8 — Filter rows keep their drag target.**
The nested layout does not change the model flags: a `smart-filter` row stays
drag- and drop-enabled within its own list, the `smart-filters` header does not
drag, and the drop validator still confines the move to the owning chain. The
tree redirects a left press on a synthetic row to the owning layer after the
base press, but records the pressed filter path so `startDrag` still carries the
filter row. The block's merged cells are paint-only.

**D9 — Hit testing uses an index-aware eye column.**
`eyeColumnContains(rect, index, pos)` uses `eyeColumnRect(rect, index).contains`.
For a layer row it is the fixed gutter; for a synthetic row it is the narrow
nested eye column. The fixed left column of a synthetic row is not a toggle
target: clicking it selects the owning layer. The event filter checks the list
toggle before the other controls.

**D10 — A synthetic-row click selects the owning layer.**
After the base view handles a left press on a synthetic row, `LayersTreeView`
selects and makes current the nearest non-synthetic ancestor. The eye, list
toggle, and other row controls are consumed by the panel before the base view
sees them, so they keep their own actions. This avoids leaving the synthetic row
highlighted while preserving the filter-row drag path from D8.

**D11 — The eye gutter is flat, lighter, and centred.**
The fixed eye cell is filled with `Theme::shade(Window, 2)` — one shade lighter
than the row's `shade(Window, 1)` — instead of the darker base gutter, and on a
selected row it takes the row's selection surface. The eye glyph is centred in
the gutter (equal left and right padding); the 1 px separator stays at the
gutter's right edge. This removes the two-tone/gradient look without moving the
glyph off the column centre. The tree also disables `allColumnsShowFocus` so
QTreeView draws no focus frame; the delegate already paints the row and ignores
`State_HasFocus`, so no proxy style (and no captured application style that a
theme switch would dangle) is needed.

**D12 — The lock badge is 80% of its previous side.**
`lockBadgeSide()` returns 80% of the old two-thirds-thumbnail side (about 13 px
at the default thumbnail), so the lock becomes visibly smaller. The lock stays
the rightmost badge; the fx badge and the list toggle sit to its left.

## Risks / Trade-offs

- **One combined state changes auto-expand semantics** → a first-seen smart
  filter now also reveals a layer's effects, and vice versa. Accepted: the user
  asked for one toggle for both.
- **Selection redirect vs drag** → the tree stores the pressed synthetic path
  for `startDrag`; `tst_smart_filter_apply::dragReordersSmartFilters` must stay
  green. If the base view starts the drag from the redirected current row
  instead of the pressed row, keep the pressed path in the tree and use it in
  `startDrag`.
- **Bottom-rule suppression depends on the next visible row** → use
  `QTreeView::indexBelow`; a helper test renders the block and checks the rule
  rows. A paint with no view keeps the row ruleless.
- **Right-edge order with a lock badge** → the single toggle sits between `fx`
  and the lock; update the lock/toggle geometry tests.
- **Image-thumbnail anchor with masks** → `imageSlotRight` deliberately stops
  before mask/vector thumbnails; a test adds a layer mask and proves the nested
  eye does not move right.
- **Flush-left eye changes pixel sampling** → the faded-eye tests sample the
  gutter background right of the glyph, not x=1.

## Migration Plan

UI-only; no data migration. Rollback is the single revert of the change.

## Open Questions

None.
