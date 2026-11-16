# Design

## One rect for paint and hit-testing

`nameRect` already mirrors paint's left and right bounds: content after the
thumbnail block, the clipping indent, and the right-edge lock/list/fx/link/
Blend If badges. The bug is only its width: it returned the whole band up to
the badges.

The delegate now has `nameRect(itemRect, index, baseFont)`:

1. Compute the same available label band.
2. Build the row's name font and elide the display text to the band width.
3. Return the elided text's advance width, clamped to the band and floored to
   1 px.

`nameRect` defaults to the application font for hit-testing and test hooks.
`paint()` calls it with `option.font`, then draws the already-elided text into
the returned rect. Synthetic rows use the same helper, so their compact rows
keep their halved gap.

## Why the application font

Paint receives `option.font` from the view. The layers tree inherits the
application font (the shell sets it on startup and after a UI-scale change), so
the two-argument `nameRect` uses `QApplication::font()` for hit-testing and test
hooks. Paint passes its own `option.font`, keeping the hit rect aligned with the
painted text without threading a style option through the event filter.

## Behavior outside the label

`LayersPanel::eventFilter` checks `nameRect` before its Layer Style branch. A
narrower rect routes an empty-row double-click to the existing
`openLayerStyle(path)` call (or the Background conversion branch). No new
branch is needed.
