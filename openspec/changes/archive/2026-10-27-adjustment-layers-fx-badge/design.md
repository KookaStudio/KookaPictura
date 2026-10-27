# Design: Adjustment layers show no fx badge

## One predicate

`LayerRowDelegate::showsFx()` is the single source for whether a row paints the
`fx` badge. `fxRect()`, `linkBadgeRect()`, `badgesRight()`, `nameRect()`, and
`paint()` all call it, so dropping the adjustment term fixes the painted badge,
its hit target, and the right-edge space reserved for the mask thumbnail and the
name in one place.

## Effects, not just a style block

`has_adjustment` is set for adjustment and fill layers alike
(`impl_layers.rs`), and `has_style` comes from `has_layer_style` (an `lfx2` or
legacy `lrFX` block). The badge keys on the layer's *present* effects — the
snapshot's `style_effects` list (`<effect>.exists`) — because `write_lfx2` keeps
the block when an effect is only switched off: a lingering block with no effects
must not show the badge. Keying on the same list the `Effects` group rows are
built from keeps the badge and the group in step, so an adjustment or fill row,
or a layer whose effects are all gone, shows neither. A layer that carries a
real effect (Paste Layer Style, or an opened PSD) shows both.

## Untouched paths

- The `layer-effects` / `layer-effect` rows and the smart-filter rows paint the
  `fx` glyph in their thumbnail slot through their own kind branch, independent
  of `showsFx()`.
- The row menu, the click and double-click handlers, and the
  Alt-click-all-effects handler already gate on the layer-style role, so they
  need no change; the Alt-click handler uses `fxRect()`, which is now empty for
  an adjustment row, so the gesture is inert there.

## Ceiling (`ponytail:`)

None: the change only narrows an existing predicate.
