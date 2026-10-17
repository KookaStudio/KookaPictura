# Design

The delegate already picks the drawn thumbnail rect with
`LayerRowDelegate::letterboxedThumb(box, index)`, and `thumbRect` calls the same
function, so the paint and hit-test rects agree by construction. The fix keeps
that single source of truth and adds one card rule ahead of the letterbox:

- A new `thumbnailCard(box, index)` returns the type layer's square card
  (`side = min(box.width(), box.height())`, centred in the slot) and otherwise
  defers to `letterboxedThumb`.
- `paint` uses `thumbnailCard` for the image thumbnail's `shaped` rect, and
  `thumbRect` uses it too, so a click on the square card still hits.

`thumbnailCard` is applied only to the **image** thumbnail. `letterboxedThumb`
stays the mask and vector-mask path, so a raster mask on a type layer keeps its
document aspect instead of stretching into a square. The slot is `thumb` wide
and the row's full height minus the pad (always at least `thumb`), so the card
side equals `thumb` in practice; the `min` keeps it square if that ever changes.

The type branch in `paint` already fills `shaped` white and draws the `T`; it
needs no change because `shaped` is now the square. The outline block skips
`type` in addition to `group`, so the card is bare. The active-target brackets
draw at `imageRect.adjusted(-1, -1, 1, 1)` and are unchanged — they now hug the
square card.

No Rust change: the document width/height and the row kind already cross the
boundary (`DocumentWidthRole` / `DocumentHeightRole`, `KindRole`).
