# Proposal

## Why

On a large document (4000²+, worse at 16000²) a zoom step immediately after a
pan hitches, while the pan itself is smooth (#1066). The present path chooses a
view-pyramid level for the zoom (`ImageView::presentLevelForZoom`). A stored
level below 0 is drawn from a zero-copy borrowed level view, so panning at a
coarse level is cheap. Level 0 is planar and straight alpha, so it is presented
from a cached premultiplied crop keyed on the visible document rectangle
(`ImageView::presentCrop`). That key is the exact rectangle, so **any** change
of the visible rectangle — every pan frame and every zoom frame at level 0 —
discards the cache and re-crops the whole viewport from level 0. At 16000² the
level-0 source is four 256 MB planar planes, so one viewport crop is a burst of
scattered reads; the first zoom step after a pan is the frame that pays it
while the previous pan frames were served from a coarser level.

Change #968 deliberately listed "scrolling a pan by copying the existing cache"
as a non-goal. This change takes that non-goal: the overlap of the new visible
rectangle with the cached crop is already present at the same level and is
reused; only the newly exposed band is cropped.

## What Changes

- Reuse the cached present crop when the visible document rectangle changes at
  the **same** level: copy the overlapping pixels from the cache and crop only
  the newly exposed band(s) from the level provider. A pan reuses nearly the
  whole crop and crops a thin strip; a zoom-in reuses the whole crop and crops
  nothing; a zoom-out crops only the frame around the old rectangle.
- Keep the cache keyed on frame, canvas revision, and level, so a pixel change
  or a level change still discards it and rebuilds whole.
- Keep the presented pixels byte-identical to a fresh crop of the same level
  over the same rectangle.

## Capabilities

### Modified Capabilities

- `ui/document-canvas`: the "Zoom-cached canvas present" requirement is
  strengthened — a change of the visible document rectangle at an unchanged
  level SHALL reuse the cached crop's overlap and crop only the newly exposed
  band(s), instead of re-cropping the whole viewport.

## Impact

- `crates/pictura-app/cpp/image_view.{h,cpp}`: `presentCrop` gains the
  reuse-and-patch path; the present cache records its level-space rectangle.
- Tests: `crates/pictura-app/cpp/tests/tst_present_cache.cpp` gains a
  deterministic pan/zoom crop-reuse case and a print-only pan-then-zoom timing
  profile; `crates/pictura-app/src/cxxqt_object/tests_profiles.rs` extends
  `scroll_zoom_pan_profile` to time a pan-then-zoom sequence.
- No new dependencies. No bridge, codec, or engine change.
