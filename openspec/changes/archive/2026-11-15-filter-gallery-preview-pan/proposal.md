# Proposal

Tracking issue: #452.

## Why

Panning the Filter Gallery preview while zoomed in is sluggish. Every scrollbar
change schedules `FilterGalleryDialog::runPreview`, which re-reduces the newly
visible document region and re-runs the whole effect stack. A slow drag or a
pause after each step therefore re-filters continuously instead of moving
already-filtered pixels.

## What Changes

- The dialog caches the filtered copy for a document region larger than the
  pane: the visible part, the existing filter-context margin, and a pan margin
  of a quarter of the viewport on each side.
- A scroll that stays inside the cached region reuses the copy and starts no
  filter run. A scroll past it schedules a new copy after the existing 120 ms
  debounce.
- Stack and zoom changes discard the cached copy, so the preview always shows
  the current stack at the current scale.
- `previewImage()` remains the cached filtered copy; a new `previewRegion()`
  accessor reports the document rect it covers.
- OK still filters the full-resolution layer; panning cannot change the commit.

## Capabilities

### Modified Capabilities

- `imaging/filter-gallery-preview`: the preview copy gains a pan margin and is
  reused across small scrolls; stack and zoom changes re-filter it.

## Impact

- `crates/pictura-app/cpp/filter_gallery_dialog.{h,cpp}`: the cache, region
  helpers, scroll slot, and accessor.
- `crates/pictura-app/cpp/tests/tst_filter_gallery.cpp`: reuse and re-filter
  coverage.
- No engine, bridge, or document changes.
