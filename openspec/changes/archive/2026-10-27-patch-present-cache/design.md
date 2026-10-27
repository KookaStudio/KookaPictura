# Design: patch-present-cache

## Context

See `proposal.md` for motivation. Current state that shapes the approach:

- `ViewPyramid` stores levels below 0 as interleaved premultiplied RGBA
  (`PyramidLevel::data`), and level 0 as a caller-owned planar straight-alpha
  `PixelBuffer` that is never stored (`crates/pictura-render/src/view_pyramid.rs`).
- `ImageView::presentCrop` (`cpp/image_view.cpp:484`) chooses a level from the
  zoom (with a preview-level override), crops the visible rectangle plus a
  one-level-pixel margin, and caches the result keyed on frame, canvas revision,
  level, and document rectangle. `blitRegion` on a document canvas only
  invalidates that cache.
- `canvas_display_image` (`src/cxxqt_object/impl_core/display.rs:20`) converts the
  crop to a `QImage` through `level_display_image`, which copies the crop bytes
  (`helpers_composite.rs:970`).
- Region refreshes (`refresh_region`, `refresh_regions`, `preview_present`,
  `edit`, history restore) and a filter job finish fold pixels into level 0 and
  the pyramid, then emit `region_blitted(QImage, x, y)`; the filter commit sends
  a 1×1 marker image. For a document canvas the image is never read; the
  destination path is re-cropping.

## Goals / Non-Goals

**Goals:**
- No viewport-sized crop or copy on a brush/preview frame at any zoom.
- No region `QImage` for document canvases.
- The canvas remains pixel-identical to a full recomposite.
- Keep the present path's level choice and its preview/GPU semantics, minus the
  GPU level-0 pin.

**Non-Goals:**
- Scrolling a pan by copying the existing cache instead of re-presenting.
- Changing `image()`, `sample_argb`, thumbnails, or export conversion.
- A new QImage binding with an explicit `bytesPerLine`; a whole-level view plus
  a source rectangle gives the same mapping without one.
- Adding a full-resolution interleaved level-0 copy.

## Decisions

### Stored levels are shared, not cropped

Levels below 0 are already in the exact RGBA format Qt wants, but
`pictura-render` must stay Qt-free. `ViewPyramid` therefore gains a std-only
shared level handle: `Arc<Vec<u8>>` bytes plus width and height, with storage
behind `Arc::make_mut` (copy on write). `pictura-app` wraps that handle as a
premultiplied RGBA8888 `QImage` (`QImage::from_raw_parts` in cxx-qt-lib 0.10)
whose cleanup keeps the handle alive. The canvas draws it as
`drawImage(target, image, sourceRect)`, mapping the same level rectangle and
target the crop path maps today.

The app-side `QImage` handle must not be kept between paints: it is asked for at
the start of the present step and dropped at the end. With no live handle,
`make_mut` is in-place, so a dab pays no level copy. A handle held across an
update would force a copy on write — correct, but not the steady path.

Alternatives:
- *A borrowed `&[u8]` with a documented lifetime and a no-op cleanup*: no
  `Arc`, but the QImage could dangle after the level is updated or dropped.
  Rejected.
- *A render-crate `QImage` constructor*: would put Qt and cxx-qt in
  `pictura-render`, which is spec math with no Qt dependency. Rejected.
- *Explicit-stride app-side QImage binding*: only needed for a sub-rectangle
  image. A whole-level image keeps `bytesPerLine = width * 4` implicit and the
  source rectangle picks the region. Rejected as unneeded plumbing.
- *Keep the pixel cache for every level*: requires copying the viewport on every
  miss and retains the current per-frame patch cost. Rejected for stored levels.

### Level 0 keeps a patched crop

Level 0 cannot be borrowed: it is planar straight alpha, and converting it
wholesale would reintroduce the 96 MB interleaved copy the module deliberately
avoids. Level 0 therefore keeps the existing cached premultiplied crop. A
document damage notification maps the changed document rectangle into the
cached crop's level, reads only that level rectangle, and row-copies it into the
crop. The mapping rounds outward to whole level pixels, so the patch equals a
fresh crop over the same document rectangle.

The Rust test for this path proves the positional premise only: cropping the
damage rectangle from level 0 equals that rectangle of the cached crop. The Qt
test proves the in-place patch: after the canvas applies the damage, the patched
cached crop is byte-identical to a fresh crop over the same document rectangle.

Alternative: store an interleaved premultiplied level 0 — rejected because it
violates the "level 0 is supplied, not stored" contract and costs a full-res
copy per document.

### Damage notification replaces the region image for documents

A new `regionDamaged(x, y, w, h)` signal carries the document rectangle;
`refresh_region`, `refresh_regions`, preview present, edit/preview, and history
restore emit it instead of building `premultiplied_display_image` for the
region. The filter-commit path carries its clipped rectangle in
`Landed::Region` (from `Refresh::Region`) and emits the same damage rectangle;
its 1×1 marker image is removed. The Rust view still patches level 0 and the
pyramid before emitting, so the canvas needs no pixels. `regionBlitted(QImage,
x, y)` and `blitRegion` stay only for the non-document `image_` fallback.

The signal is emitted only after the pixel fold, as today, so the canvas always
reads the updated level.

### The damage handler mirrors the present level choice

`presentCrop` selects `presentLevelForZoom` with a preview override. The damage
handler uses the same function. If it selects a different level than the cached
crop, the cache is discarded instead of patched — this is what keeps a preview
stroke correct, because a preview patches only its stored level and never level
0. A level above 0 needs no patch: its app-side borrowed level view reads live
bytes.

After a level-0 patch the cache's revision is advanced to the current canvas
revision; otherwise the next paint would discard the patch. Any later change
bumps the revision again, so a stale cache is still detected.

### The GPU level-0 pin goes away

`refresh_region` already folds a GPU dab into level 0 and repairs every stored
level through `update_pyramid`. The pin existed because the canvas could only be
trusted to crop a level that the blit had just refreshed; with damage patching
and borrowed views that reason is gone. `preview_present_level` returns the
preview level while a preview runs and `0` otherwise; `-1` disappears from the
provider contract and the C++ override.

### Single-channel mode keeps the crop path

`channelImage` caches a greyscale conversion by `QImage::cacheKey`. The app-side
QImage is a fresh wrapper per paint, so its key changes and the channel
conversion would rerun over the whole level every paint. When a single channel
is visible the canvas keeps using the cropped, cached image. This is a rare
display mode and not on the stroke hot path.

## Risks / Trade-offs

- **Qt may sample a source rectangle differently from a separate crop near the
  source boundary** → The source rectangle is the same rectangle (with the same
  one-level-pixel margin) the crop path maps, so the mapping should be
  identical. A Qt test renders both ways at several zooms/offsets and asserts
  pixel equality; if it fails, keep the crop cache for that level and fall back
  to patching it (the mechanism stays available).
- **A shared level handle held across an update silently doubles level memory
  and copies the level** → The app-side QImage (and its handle) is created and
  dropped inside one paint; a Rust test holds a handle across an update and
  asserts the old handle's bytes are unchanged (copy-on-write), and a second
  test asserts an update with no handle keeps the storage pointer. Reviewers see
  the ownership in one type.
- **External-buffer `QImage` requires 4-byte alignment** → Assert alignment when
  constructing the view; `Vec<u8>` allocation meets it on supported targets.
- **Signal contract change breaks shell and test consumers** → Mechanical
  migration; the compiler and `QSignalSpy` uses enumerate every site. The two
  pixel consumers named above are kept.
- **Level-0 patch rounding can miss a pixel column at the crop edge** → Map the
  damage rectangle with floor/ceil to whole level pixels and clip to the cached
  crop; a byte-identity test against a fresh crop covers the edges.
- **Damage with no cache yet** → Do nothing; the next paint fills the cache as
  it does today.

## Migration Plan

One pull request: Rust storage and the std-only shared level handle, the
app-side QImage wrapper, the signal and its emit sites, the C++ present path,
then tests. No persisted state changes, so rollback is a revert. Existing Qt
tests that read `regionBlitted` for documents move to `regionDamaged`; the
self-test's present/paint-perf checks are adapted in place, with no new
self-test checks.

## Open Questions

None.
