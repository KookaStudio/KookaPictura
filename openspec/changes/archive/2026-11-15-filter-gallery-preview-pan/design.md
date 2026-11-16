# Design

## Cached region

`runPreview` used to filter exactly the pane plus `kPreviewMargin` (16 screen
pixels of filter context). It now filters that region expanded by a pan margin
of a quarter of the viewport on each side, clamped to the document. The proxy
is still at `min(1, zoom × devicePixelRatio)` device pixels per document pixel,
so the cached image stays pane-sized (at most 1.5× the pane per dimension) and
never follows the document.

Three helpers keep the geometry in one place:

- `visibleRegion()` — the document rect under the pane's viewport, from the
  scrollbar values and `zoom_`, clipped to the document.
- `neededRegion()` — `visibleRegion()` expanded by `kPreviewMargin / zoom_`, the
  context the filters read at the visible border.
- `paddedRegion()` — `neededRegion()` expanded by a quarter of the viewport per
  side, clipped to the document; this is what `runPreview` filters.

`previewRegion_` stores the document rect of the cached image and
`previewScale_` the scale it was rendered at.

## Reuse on scroll

The scrollbar `valueChanged` connection now reaches `previewScrolled()`, not
`schedulePreview()`. It reuses the cache when it is clean
(`previewDirty_ == false`), the cached scale equals the current scale, and
`previewRegion_` contains `neededRegion()`. Otherwise it calls
`schedulePreview()`, which sets `previewDirty_` and starts the existing 120 ms
single-shot timer. A successful `runPreview` stores the new image, region and
scale and clears the dirty flag.

The canvas already sits at the document's zoomed size and paints the image at
`previewRegion_ × zoom_`, so Qt's scroll area moves the cached pixels during a
pan; no repaint code changes. Because reuse requires the pane plus its context
margin to stay inside the cached region, visible pixels always have at least the
same filter context a fresh render would give them.

Stack changes, parameter changes, visibility toggles, reordering, add/delete,
and zoom changes all go through `schedulePreview()` and therefore discard the
cache. A viewport resize is checked with `previewScrolled()` so growing the
dialog cannot expose an uncached strip.

## Debounce and commit

The 120 ms debounce is unchanged: scroll changes that need a new copy restart
the timer, so a slow drag queues at most one filter run. The cache removes the
runs that only moved within the margin. The commit path
(`filter_stack_commit_start`) reads the document, never the preview cache, so
OK still filters the full-resolution layer exactly as before.

## Tests

`tst_filter_gallery::panReusesTheCachedPreview` pans less than the margin and
asserts, after the debounce, that the region and image are unchanged and the
region still covers the pane. `panPastTheMarginRefilters` pans past the margin
and waits for a new region that covers the pane. `zoomChangeRefiltersTheCache`
checks that a zoom change discards the cached copy. `previewRegion()` is the
observable seam; the image itself is the cached copy.
