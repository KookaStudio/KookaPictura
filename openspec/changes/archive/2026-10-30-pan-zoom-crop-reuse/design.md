# Design: pan-zoom-crop-reuse

## Context

`ImageView::presentCrop` (`crates/pictura-app/cpp/image_view.cpp`) caches one
premultiplied crop, keyed on the frame (`frameKey_`), the canvas revision, the
level, and the exact document rectangle. It is used for level 0, which is
planar straight alpha and has no borrowed view, and for any level when the
zero-copy level view is unavailable (single-channel mode, or a provider without
`levelImage`). A hit requires `presentCache_.docRect == docRect`, so any change
of the visible rectangle re-crops the whole viewport.

`presentLevelRect` maps the visible document rectangle to a level rectangle
(`sourceRect`) and its document rectangle (`docRect = sourceRect * 2^level`),
with a one-level-pixel margin. `levelProvider_.crop(level, x, y, w, h)` returns
a premultiplied crop of that level rectangle.

The zero-copy level view path (`level > 0` with `levelProvider_.levelImage`) is
unaffected: it never populates the crop cache and drops it each paint.

## Goals / Non-Goals

**Goals:**
- A pan or zoom at an unchanged level copies the cached crop's overlap and
  crops only the newly exposed band(s).
- The reused crop is byte-identical to a fresh crop of the same level over the
  same rectangle.
- A canvas revision change or a level change still rebuilds whole.

**Non-Goals:**
- A stored interleaved level-0 buffer (rejected in #968: 16 bytes per pixel,
  256 MB at 4000², ~1 GB at 16000²).
- Caching across a level change: a different level is a different resolution.
- Changing the level choice, the preview-level override, or the sampling
  filter.

## Decisions

### Reuse the overlap, crop only the exposed band

`presentCrop` gains a second cache path. When the frame, revision, and level
match but the rectangle differs, and the new level rectangle overlaps the
cached one, it builds the new crop from two sources: the overlapping pixels are
row-copied out of the cached crop (both are the same level's pixels at the same
resolution), and the region of the new rectangle outside the old one is cropped
from the level provider. It then replaces the cache with the new crop and its
rectangle.

Decomposing the newly exposed region into bands gives at most four rectangles,
of which at most two are non-empty for a translation:

- left band `[new.left, old.left) × new.height`
- right band `[old.right, new.right) × new.height`
- top band `[max(new.left, old.left), min(new.right, old.right)) × [new.top, old.top)`
- bottom band `[max(new.left, old.left), min(new.right, old.right)) × [old.bottom, new.bottom)`

A pan crops one or two thin strips. A zoom-in has the new rectangle inside the
old one, so every band is empty and no provider crop runs. A zoom-out crops the
frame around the old rectangle. The work is bounded by the newly exposed area,
never the viewport.

### Level-space arithmetic

The cache is stored in level pixels and the document rectangle is always the
level rectangle scaled by `2^level`, so the cached level rectangle is the
document rectangle divided by the scale and the overlap is computed in level
pixels directly. The provider takes level-space rectangles, so the bands are
already in its frame. No floating-point rounding is involved: the rectangles
are exact multiples of the scale.

### Fallback to a whole rebuild

The patch is attempted only when the cache matches on frame, revision, and
level, the cached crop is non-null, the two level rectangles overlap, the
provider returns each band at its requested size and format, and the new crop
allocates. Any failure falls through to the existing whole-crop path, which
also counts a rebuild. A band whose provider crop is null, short, or of a
different format abandons the patch; the partially built image is discarded.

### Pixel identity

Every pixel of the new crop comes from the level provider over the same
rectangle: the overlap from a crop the provider returned for that rectangle
earlier in the same revision, the bands from a fresh provider crop. Because the
level's bytes are unchanged within a revision (a pixel change bumps
`canvasRevision` and discards the cache), the result equals a fresh crop. The
Qt test asserts byte identity against a cache-disabled draw and against a fresh
crop at several pan and zoom steps.

### The rebuild counter keeps its meaning

`presentCache_.rebuilds` counts whole crops, so a reuse does not increment it.
The pan/zoom reuse test asserts the counter is unchanged across a pan and across
a zoom-in, and that the presented pixels still match. `presentCacheRebuiltLastPaint_`
keeps its current meaning (the presented level or rectangle changed), so the
existing checks are unaffected.

## Risks / Trade-offs

- **A patch that is subtly wrong near the crop edge** → The overlap copy and the
  bands are disjoint and cover the new rectangle exactly; a test renders the
  reused present and a cache-disabled draw and asserts byte identity.
- **More provider calls (up to four bands) could cost more than one crop** →
  Bands are small; a pan crops one or two strips, a zoom-in none. The timing
  profile prints pan and zoom frame times so the trade is measured, not assumed.
- **The single-channel / no-level-view path also patches** → It uses the same
  math, and `channelImage` is keyed on the crop's cache key, which a patch
  changes; the existing `channelImageKey_ = -1` invalidation in `damageRegion`
  is mirrored by rebuilding the key from the new crop.

## Migration Plan

One pull request: the C++ present-cache change, the Qt reuse test, the extended
Rust profile, the spec delta, and the test plan. No persisted state changes, so
a revert is clean.

## Open Questions

None.
