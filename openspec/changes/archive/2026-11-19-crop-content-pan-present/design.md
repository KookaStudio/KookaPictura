# Design

## Context

See `proposal.md` — Why.

The document canvas holds no full-resolution image; it presents from the view
pyramid. `presentLevel()` picks the level for the zoom, `presentLevelRect()`
turns the visible document rectangle into a level-space crop rectangle, and
`presentCrop()` returns that crop, reusing the previous frame's crop and cropping
only the bands a pan exposes. Two of the paint path's four draws use that crop;
the other two fall back to `image_` (a non-document canvas) or a level-0 viewport
crop.

A Modern crop content pan sets `cropContentOffset_`, a document-space translation
applied when drawing. The paint path handles it before those draws, by drawing
`ImageView::image()` — for a document canvas, a level-0 crop of the whole
document — at the shifted origin. `presentLevelRect`/`presentCrop` still run
first, so each pan frame also pays for a viewport crop it never draws. The
viewport crop it fetches is for the *unshifted* visible rectangle, which is the
wrong region while content is panned.

## Goals / Non-Goals

**Goals:**

- A content pan frame costs what a hand pan frame costs: a viewport-bounded level
  crop, never a whole-document materialisation.
- The panned frame is the idle present of the same content, shifted.
- The change stays inside the existing present machinery; no new provider method,
  no new cache, no new translation unit.

**Non-Goals:**

- Changing `image()`'s whole-document contract or its other callers (the
  Navigator panel's non-document fallback, the canvas self-test).
- Changing what a content pan *selects* (committed region, box position); that
  stays as specified by the existing Crop repositioning requirement.
- Removing the pyramid-free fallbacks for a canvas without a level provider.

## Decisions

**Take the present's visible rectangle from a private helper, not from
`visibleDocumentRect()`.** With a content offset `c`, document point `p` is drawn
at `offset + (p + c) * zoom`, so the content on screen is the viewport in document
coordinates moved back by `c`. The shift has to be applied **before** clipping to
the document: a document sitting inside a wider viewport (any small image, and
every fit-zoom view, which leaves a 5 % margin) still shows the band the offset
moves into that margin, and clipping first drops it. `viewportDocumentRect()` is
therefore the unclipped mapping, `visibleDocumentRect()` keeps the clip, and
`presentVisibleRect()` shifts the unclipped rect and then clips. That answer is
only wanted by the present: seven dialog and filter callers
(`apply_image_dialog.cpp:349`, `replace_color_dialog.cpp:304`,
`adjustment_dialog.cpp:631,641`, `menu_adjust.cpp:98`, `menu_filter.cpp:49`,
`match_color_dialog.cpp:310`) draw their previews in unshifted document space and
can run while a Modern box is panned, so handing them a shifted rectangle would
move the requested region away from where it is drawn. Only `presentLevelRect`
and the two paint fallbacks call the helper. Their preview-during-pan
inconsistency is pre-existing and out of scope. Alternative rejected: changing
the public method — a wider blast radius for no present benefit.

**Fold the offset into the existing targets and delete the special branch.** All
four targets are `offset + docRect * zoom`; the offset adds a constant
screen-space shift `cropContentOffset_ * zoom`, so one local origin serves them
all. This removes the whole `image()` draw (the last `image()` call in
`paintEvent`) for about five deleted lines and one added local. Alternatives
rejected: snapshotting the full image at pan start — still a 64 MB materialise,
still one frame of cost, and a stale snapshot while the document is repainted;
cropping level 0 to the viewport instead — bounded but re-premultiplies per frame
and abandons the cache and reduced levels the rest of the present already uses.

**Define frame identity against the idle present, per level and filter.**
Reduced levels are prefiltered copies, not the result of any particular
downscale, so drawing level 0 at 40 % and drawing level 1 at 40 % cannot be
byte-identical in general; the corpus already scopes pixel identity this way for
the zoom-dependent sampling filter and proves level-0 equality only at 100 %
(`level0PresentMatchesTheTransformDraw`). Consequence: above 50 % the pan is
pixel-identical to today; at or below it the pan becomes the frame the crop
session shows when still, which is the intended contract. The >50 % case is
asserted directly by comparing a panned frame with the direct draw
(`setPresentLevelCropForTest(false)`), which is level 0 either way.

**Shift the single-channel path too.** Today a panned frame silently draws the
composite while one channel is visible, then reverts on release. Shifting the
same targets keeps the channel moving, is the smaller diff, and costs what a
hand pan already costs with one channel visible (the greyscale image is rebuilt
from the viewport crop per frame, not from the document).

**Level quantisation is accepted.** `presentLevelRect` snaps the crop to whole
level pixels, so a content offset that is not a multiple of `1 << level` lands on
the same grid a view pan of that amount would. Tests compare a content offset of
10 px at level 0 and level 1, where both are exact.

## Risks / Trade-offs

- The present cache patches the visible crop by copying the overlap each pan
  frame → viewport-bounded (a few MB at typical window sizes), the same cost an
  ordinary hand pan already pays; the whole-document conversion it removes was
  one to two orders of magnitude larger.
- Straighten plus a content pan: the old branch drew the whole image under the
  rotation, so a panned straightened frame could cover rotated-corner content
  that the level crop (the unrotated visible rectangle) does not. The change
  makes the pan match the still frame, which already shows that band, so no new
  artifact class appears; the straighten pivot also does not follow the content
  offset, which is a pre-existing inconsistency this change leaves alone. No test
  covers straighten plus pan.
- The pixel-equality test sets no crop box. The box overlay draws at
  `p * zoom + offset_`, so it would move in the view-pan frame but not in the
  content-offset frame; with no box, the compared frames differ only in the
  present source. The 4000×3000 fixtures cover the viewport, so the canvas rect and
  clip do not affect the compared interior; a 520×120 case covers the
  viewport-wider-than-document path, where the offset moves content into the
  vacated margin and the whole document must stay presented.
- `image_view.cpp` is at 1195 of its 1200-line cap → the change deletes more than
  it adds there, and the helper goes to `image_view_present.cpp`; the test cases
  go to `tst_present_cache.cpp` (897 of 1400), so no allowlist entry is needed.
- A test comparing a content offset with a view pan fails spuriously if the
  offset is not a multiple of the level scale, because both frames quantise to
  the level grid → use 10 px, which is a multiple of both level-0 and level-1
  scale.
- The presented view reported to the render session (`reportView`) now names the
  panned rectangle, so tile pinning follows a content pan → covered by the fake
  provider's `reportView` assertion in `cropPanPresentsThroughALevel`.
