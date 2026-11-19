# Proposal

## Why

A Modern crop content pan paints the whole document on every frame. `ImageView::image()` answers a document canvas with `levelProvider_.crop(0, 0, 0, docW, docH)`, so each pan frame premultiplies and interleaves every pixel of level 0 and then copies the result again (`to_vec()`): about 64 MB per frame at 4000² and about 1 GB at 16000², for a frame that shows only the viewport. Every other present already goes through the zoom-selected view-pyramid level crop, and the pan path computes that crop too and then discards it.

## What Changes

- The Modern crop content pan presents through the same zoom-selected level crop as the idle present, shifted by the content offset, instead of rescaling the full-resolution document.
- The present path takes its visible rectangle from a private helper — `visibleDocumentRect()` minus the content offset, intersected with the document — so it crops the content actually on screen and the view reported to the render session names the region shown. The public `visibleDocumentRect()` is unchanged, so the adjustment, filter, and colour dialogs keep their current preview regions.
- The content offset is folded into the existing present targets; the special full-image draw branch is removed. A single-channel view keeps moving with the content during a pan instead of reverting to the composite for the duration of the drag.
- All-channel pixels above 50 % are unchanged: the pan still draws level 0. At or below 50 % the pan frame becomes the level crop the idle present uses, which is the intended pixel contract. The single-channel view now keeps the channel at any zoom. This replaces the issue's literal "≤50 % identical to current output" criterion; note that on #1097 when the PR opens.
- A straightened Modern pan now matches the still frame instead of drawing the whole image behind the rotation; the still frame already clips the rotated corners, so this change does not introduce a new artifact class.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `tools/crop-straighten`: adds a requirement that a Modern crop content pan presents through a viewport-bounded level crop (allowing the present's sampling margin), requests no full-resolution document, and produces the same frame as the idle present of the same content.

## Impact

- `crates/pictura-app/cpp/image_view.cpp` and `image_view_present.cpp` — a private present-path visible-rect helper and the `paintEvent` present targets; the public `visibleDocumentRect()` is untouched.
- `crates/pictura-app/cpp/tests/tst_present_cache.cpp` — new Qt Test cases for bounded crops during a content pan, pan-frame equality with a view pan, and the single-channel pan.
- `ImageView::image()` keeps its whole-document contract; its remaining callers are the Navigator panel's non-document fallback (`panels/navigator_panel.cpp:288`) and the canvas self-test.
- `docs/dev/STATE.md` — the "left" list no longer names the crop content pan.
