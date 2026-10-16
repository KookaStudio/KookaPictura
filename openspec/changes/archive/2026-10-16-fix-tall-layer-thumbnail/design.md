# Design

## Context

See `proposal.md - Why`. The two producers of a row thumbnail that carry
document context are:

- `layer_thumbnail_positioned(layer, doc_w, doc_h, size)` in
  `helpers_composite.rs`. It computes one `scale = size / max(doc_w, doc_h)`,
  scales the layer's `rect.left/top` and size by it, and allocates a
  `size × size` RGBA buffer, copying layer pixels only inside the scaled dest
  rect. The rest is transparent.
- `shape_thumbnail(layer, doc, size)` in `shapes.rs`. It samples
  `shape_coverage` on a `size × size` grid over the document extent using
  `scale = max(doc_w, doc_h) / size`, again producing a square.

The delegate (`layers_panel_internal.h`) computes `shaped = letterboxedThumb(box,
index)` — the document-aspect rect inside the square thumbnail slot — and paints
the source into it with `painter->drawImage(shaped, image)`, which scales the
**entire** source image to fill `shaped`. The outline, checkerboard, and active
bracket all use `shaped`, and `thumbRect`/hit-testing use `letterboxedThumb`
too. So the target aspect is already correct and must stay fixed; only the
source aspect is wrong.

## Goals / Non-Goals

**Goals:**

- A full-width layer in a tall (or wide) document fills the thumbnail's full
  width (height) with no transparent strip and no horizontal squash.
- Shape-layer thumbnails share the fix.
- `thumbRect`, `letterboxedThumb`, and every hit-test/adjacency computation stay
  exactly as they are.

**Non-Goals:**

- Changing the delegate's letterbox math or the row layout.
- Mask / vector-mask thumbnail aspect (their source is the mask's own
  aspect-kept square and their target is the document-aspect slot; a full-doc
  mask already matches, so they are out of scope here).
- Any document, codec, or compositing behavior change.

## Decisions

### Emit the document aspect from the bridge, not the delegate

Chosen: make `layer_thumbnail_positioned` and `shape_thumbnail` produce a
document-aspect image, matching `letterboxedThumb`'s `tw`/`th` rule:

```
if doc_w >= doc_h:  tw = size, th = max(1, doc_h * size / doc_w)
else:               tw = max(1, doc_w * size / doc_h), th = size
```

This is the issue's first option and the one that "keeps `thumbRect`/hit-testing
unchanged": the delegate continues to letterbox its target to the document
aspect, and now the source shares that aspect, so `drawImage` is an
aspect-preserving scale. For a square document the output is identical to today
(`tw == th == size`), so every existing fixture (16×16, 4×4) is unaffected.

Alternative considered: keep the square source and change the delegate to draw
it preserving its own aspect (a source-aspect sub-rect). Rejected — it would
make the painted area disagree with `thumbRect` and the outline/checkerboard,
and it duplicates the letterbox math in a second place.

### Place the layer with independent x/y scales

`layer_thumbnail_positioned` scales the layer offset and size by `tw/doc_w` and
`th/doc_h` (not one shared `scale`), so the layer lands at its true
document-relative position in the non-square canvas. A full-width layer
(`layer.rect.width() == doc_w`) therefore yields `dest_w == tw` and no transparent
right column; clipping to `tw`/`th` handles the rounding at most one pixel off.

### `shape_thumbnail` samples on the non-square grid

The shape thumbnail keeps sampling `shape_coverage` per target pixel but over a
`tw × th` grid with independent x/y scales, so the shape's document coverage is
preserved without squash. It already samples the full document extent regardless
of Entire Document / Layer Bounds, which is unchanged.

## Risks / Trade-offs

- [Rounding makes the source one pixel off the delegate's target] → Accepted;
  `drawImage` then scales by a sub-pixel amount and the full-width content still
  fills the target, so there is no visible strip. The Rust unit test asserts the
  exact `tw`/`th` for the chosen fixture.
- [Existing golden/snapshot expectations change] → Only for non-square documents;
  all current thumbnail fixtures are square, so `tst_layers_snapshot` and the
  Rust tests keep passing. Any change would be reported explicitly.
- [Shape thumbnails lose a little vertical resolution on a tall doc] → Intended:
  the long edge is still `size`, matching the drawn rect.
