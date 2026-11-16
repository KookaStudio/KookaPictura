# Tasks

## 1. Tests

- [x] 1.1 Add `tst_filter_gallery::panReusesTheCachedPreview`: a scroll smaller
  than the pan margin leaves the cached region and image unchanged after the
  debounce, and the region still covers the pane.
- [x] 1.2 Add `tst_filter_gallery::panPastTheMarginRefilters`: a scroll past the
  margin renders a new region that covers the pane.
- [x] 1.3 Add `tst_filter_gallery::zoomChangeRefiltersTheCache`: a zoom change
  discards the cached copy and the new region covers the pane.

## 2. App

- [x] 2.1 Add `visibleRegion()`, `neededRegion()`, and `paddedRegion()` to
  `FilterGalleryDialog`; `runPreview` filters the padded region.
- [x] 2.2 Connect the scrollbars to `previewScrolled()`, which reuses a clean
  cache that covers `neededRegion()` and otherwise calls `schedulePreview()`.
- [x] 2.3 Mark the cache dirty in `schedulePreview()` and clean after a
  successful `runPreview`; compare the cached scale.
- [x] 2.4 Check the cache on a viewport resize.
- [x] 2.5 Add `previewRegion()` and update the `previewImage()` comment.

## 3. Gates

- [x] 3.1 Build and run `tst_filter_gallery`; run the application self-test.
- [x] 3.2 `python3 scripts/check-test-plan.py`, `openspec validate --all --strict`.
- [x] 3.3 Archive with `openspec archive filter-gallery-preview-pan`.
