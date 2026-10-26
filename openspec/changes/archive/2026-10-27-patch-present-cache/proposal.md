# Proposal: patch-present-cache

## Why

Issue #968. On a document canvas every brush or preview frame invalidates the
present cache, and the next paint re-crops the whole visible viewport from the
view pyramid: a scalar per-pixel loop plus a second full-copy conversion to a
`QImage`. At fit zoom on a 6000×4000 document that is roughly 6 MP cropped and
copied twice per frame, which alone exceeds the 16 ms budget; a live GPU stroke
pins the crop to level 0, so it premultiplies the whole 24 MP visible document
per frame. In parallel Rust builds a region `QImage` for every `regionBlitted`
that the document canvas never reads.

The pyramid already holds the changed pixels below level 0 in interleaved
premultiplied RGBA format, and level 0 is served on demand. The viewport copy
and the discarded region image can be removed without changing what the canvas
shows; level 0 still builds a damage-sized crop.

## What Changes

- **Damage-only notification.** A document region refresh, including a filter
  commit, emits `regionDamaged(x, y, w, h)` instead of
  `regionBlitted(QImage, x, y)`. No region image is built, converted, or passed
  across FFI for a document canvas. `regionBlitted` and `ImageView::blitRegion`
  stay only for the non-document `image_` fallback; the filter-commit 1×1 marker
  image is removed.
- **Zero-copy stored-level present.** `ViewPyramid` levels below 0 are already
  interleaved premultiplied RGBA, so the render crate exposes a std-only shared
  level handle (`Arc`-backed bytes plus width/height) and `pictura-app` wraps it
  as a borrowed premultiplied RGBA8888 `QImage` over the level's bytes. The
  canvas draws that image into the viewport with a source rectangle instead of
  cropping and copying the viewport. The app keeps the handle alive for the
  QImage; an update that races a live handle copies the level on write (correct,
  and not the steady path).
- **Level-0 patch.** Level 0 is planar straight alpha and cannot be exposed
  zero-copy, so it keeps a premultiplied crop cache. A damage rectangle maps to
  level coordinates and re-crops only that rectangle, patched into the cached
  crop. Per-dab cost becomes O(dab) instead of O(viewport): the document region
  `QImage` is gone, but the level-0 patch still builds a damage-sized crop (not
  zero images) and copies it into the cache.
- **No GPU level-0 pin.** The pyramid update already repairs every stored level
  after a GPU dab, so a live GPU stroke presents the zoom-selected stored level
  (zero-copy) instead of forcing a level-0 crop. `preview_present_level()` no
  longer reports `-1`.
- **BREAKING (internal signal contract).** Shell and test code that reads the
  `regionBlitted` pixel argument for a document refresh must listen to
  `regionDamaged` instead. The self-test checks that assert a blit's pixel
  width are adapted; new GUI coverage lives in Qt Test.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `ui/document-canvas`: the C++ region-blit contract becomes a damage
  notification; the present path uses a zero-copy view for stored levels and a
  damage-patched crop for level 0; a live GPU stroke presents the zoom-selected
  stored level rather than level 0.
- `compositing/view-pyramid`: the render crate exposes a stored level as a
  std-only shared level handle that shares the level's storage and copies on
  write if an update runs while a handle is alive; the Qt display image is built
  app-side from that handle, and `pictura-render` gains no Qt dependency.

## Impact

- `crates/pictura-render/src/view_pyramid.rs`: level storage becomes
  copy-on-write and gains a std-only shared level handle; the handle and its
  tests (no Qt dependency).
- `crates/pictura-app/src/cxxqt_object.rs`, `impl_core/display.rs`,
  `impl_core.rs`, `impl_paint.rs`, `edit.rs`,
  `document_session/adapter.rs`, `impl_history.rs`: `regionDamaged` emission,
  no region image for document refreshes, app-side QImage wrapper for the
  shared level handle, no `-1` present level.
- `crates/pictura-app/cpp/image_view.{h,cpp}`,
  `image_view_document.cpp`, `frame_canvas.cpp`: level view in the present
  path, level-0 crop patch, damage entry point.
- `crates/pictura-app/src/cxxqt_object/filter_tools/jobs.rs`: the filter commit
  carries its clipped rectangle in `Landed::Region` and emits `regionDamaged`;
  the 1×1 marker pixel image is removed.
- Tests: new Qt suite for the present path; existing Qt spies migrated from
  `regionBlitted` to `regionDamaged`; self-test present/paint-perf checks
  adapted (no new self-test checks).
- No dependency or `ROADMAP.md` change. `docs/dev/m32-interactive-canvas.md` and
  `docs/dev/m35-cpp-region-blit.md` are historical milestone notes describing
  the superseded invalidate-on-blit present cache; they are intentionally not
  updated and need no docs write in this change.
