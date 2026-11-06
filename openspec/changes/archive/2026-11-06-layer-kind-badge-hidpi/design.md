# Design

## Context

`LayerRowDelegate::paint` draws two kinds of badge on the image thumbnail's
lower-right corner: `layers.kindShape` for a shape row and
`layers.kindSmartObject` for a smart-object row. Both compute
`side = qMax(8, thumb / 2)` and draw
`pictura::icon(id).pixmap(side, side)` into a `side`-sized logical rect. That
overload renders the SVG at a device pixel ratio of 1.0
(`SvgIconEngine::pixmap` → `scaledPixmap(size, mode, state, 1.0)`), so on a 2x
screen the compositor upscales a `side × side` pixmap to `2·side × 2·side`.

The delegate already has the correct path: `paintAsset` renders the eye and
chevron with `pictura::icon(id).pixmap(QSize(side, side), dpr)`, where
`dpr = painter->device()->devicePixelRatioF()`, and draws it centred in the
target rect. `SvgIconEngine::scaledPixmap` renders at `size * dpr` pixels and
sets the pixmap's device pixel ratio, so the pixmap carries the screen's
resolution.

## Goals / Non-Goals

**Goals:**

- The shape and smart-object corner badges are rendered from an SVG pixmap at
  the painter's device pixel ratio and stay crisp at 1x and 2x.
- One shared helper owns the badge's base fill, corner, and DPR-aware render, so
  the two badges cannot drift apart.
- A Qt test paints a shape row and a smart-object row at 2x and pins the result.

**Non-Goals:**

- Changing the badge's corner, size, base fill, asset, or the row's other
  content.
- Changing the badge glyph itself, the SVG assets, or `SvgIconEngine`.
- Any document, codec, compositing, or dependency change.

## Decisions

### Route both badges through a shared `paintKindBadge` built on `paintAsset`

Chosen: add `paintKindBadge(painter, corner, assetId, base)` that fills the
corner with the base colour and calls `paintAsset(painter, corner, assetId)`.
`paintAsset` already derives `dpr` from the painter's device and draws the
DPR-sized pixmap at the badge's logical size, so the badge inherits the correct
HiDPI behaviour with no new render code. Both the shape and smart-object branches
call it.

Alternative considered: inline `.pixmap(QSize(side, side), dpr)` in each branch.
Rejected: it duplicates the DPR lookup and the base fill, which is exactly how
the two badges drifted from `paintAsset` in the first place.

### A `kindBadgeRect` accessor shared by `paint` and the test

Chosen: add a public `kindBadgeRect(itemRect, index)` on the delegate that
returns the lower-right corner of the image thumbnail slot
(`imageThumbBox`-based, `side = qMax(8, thumb / 2)`). `paint` uses it for both
badges and the test reads it, so the painted corner and the asserted corner are
the same rectangle by construction.

Alternative considered: recompute the corner in the test from the thumbnail
card. Rejected: the badge sits on the thumbnail *slot*, which for a square
document is taller than the letterboxed card, so the card rect is the wrong
corner.

### Paint at 2x through the existing row seam

Chosen: give `LayersPanel::rowImageForTest` an optional device-pixel-ratio
parameter (default 1.0) that sizes the backing image at `vr.size() * dpr` and
sets the image's device pixel ratio before painting. Existing callers keep the
1x default. The test renders the row at 2x, crops the badge corner, and compares
it against two references: the badge drawn from a 2x pixmap and the badge drawn
from a 1x pixmap into the same 2x device. It asserts the actual pixels are
closer to the crisp reference, which is self-calibrating and needs no magic
threshold.

Alternative considered: a test-only hook recording the DPR of the pixmap `paint`
drew. Rejected: it adds mutable production state to observe something the
rendered pixels already prove.

## Risks / Trade-offs

- [The test's badge corner also contains the active-thumbnail brackets] -> The
  test clears the tree selection before painting, so `singularActive` is false
  and no brackets are drawn; the badge's base fill covers the outline and
  bottom rule in the corner either way.
- [`rowImageForTest` grows the near-cap test seam file] -> The change is one
  statement plus a DPR setter; the file stays under its 1400-line test cap.
- [A fractional device pixel ratio] -> `paintAsset` already rounds `size * dpr`
  and sets the pixmap DPR, the same path the eye and chevron use; no new
  rounding is introduced.

## Migration Plan

None. Session state, document format, and saved files are unchanged.
