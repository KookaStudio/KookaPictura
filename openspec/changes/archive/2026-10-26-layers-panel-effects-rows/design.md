# Design: Layers panel layer-effects rows

## Synthetic rows

The panel already projects Smart Filters as synthetic rows (a group row plus one
row per filter) under a smart-object layer. Layer effects reuse the same shape:
in `LayersPanel::refresh()` each styled layer contributes an `Effects` group row
at `"<path>/@fx"` (`kind` `layer-effects`) and one row per present effect at
`"<path>/@fx/<key>"` (`kind` `layer-effect`). The model attaches children by the
parent path, so the rows are appended after the layer rows like the Smart
Filters rows.

The present effect keys are the snapshot's `styleEffects` list (already filtered
by `<effect>.exists`). The group row's eye needs the master switch and each
effect row's eye needs `<effect>.on`, so the Rust snapshot gains
`style_effects_on` (the enabled subset, newline-joined) and `style_visible` (the
`masterFXSwitch`, default true). Both cross in the one snapshot the panel
already takes, keeping the "one crossing per rebuild" rule.

## Disclosure and eyes

The layer row's `fx` badge is the group's disclosure: a click toggles
`tree_->expand` on the layer index. A newly styled layer is expanded when
**Expand New Effects** is on, tracked by a `styledPathsSeen_` set so a later
user collapse is not overridden. The `Effects` group row is `expandable`, so the
delegate draws its chevron and reserves the slot; the effect rows are not.

The `Effects` eye toggles `fx.visible` and an effect eye toggles `<effect>.on`
through `layer_style_set`, then `layer_style_commit` records the change as one
state, the same pair the Layer Style dialog uses for its live edits.

## Delegate

The `layer-effects` and `layer-effect` rows paint the `layers.fx` glyph in their
thumbnail slot, as the Smart Filters rows do, and carry no right-edge `fx` badge
(their snapshot `has_style` is false).

## Dialog and menu

`LayerStyleDialog` exposes its ordered `{key, title}` effect table, so the panel
names the rows and the effect menu entries from one source. `openLayerStyle`
gains an effect argument, and the row menu / fx-badge menu build the CS6
layer-style menu, reusing the existing `blendingOptions`, `copyLayerStyle`,
`pasteLayerStyle`, `clearLayerStyle`, `scaleLayerEffects`, and hide/show-all
handlers plus the new per-layer disable/enable.

## Ceilings (`ponytail:`)

- A double-click on the `fx` badge also toggles the group, because the first
  click of the pair has already expanded/collapsed it.
- The effect rows follow the dialog list order; CS6's exact panel order beyond
  "Drop Shadow last" is not confirmed.
- Global Light… and Create Layer stay disabled (#580).
