# Tasks: large-image-stroke-boundaries

## 1. Measurement

- [x] 1.1 `large_document_stroke_profile` (ignored) at 16507×16196.

## 2. History

- [x] 2.1 Plane stamps; private current state, parallel tile diff, in-place capture.
- [x] 2.2 In-place undo/redo/jump with restored damage; hollow segment anchor.
- [x] 2.3 Model test against a full-snapshot history.

## 3. Stroke

- [x] 3.1 `Stroke` paints a borrowed document in place, saving touched tiles; cancel restores; a restructured layer is refused.
- [x] 3.2 Port the stroke's callers (paint tools, GPU stroke, bridge, tests); the Background Eraser layers the live document and keeps `stroke_base`.

## 4. Bridge and canvas

- [x] 4.1 Record from the live document by reference; undo/redo/jump refresh only the restored region; level 0 never shares the composite.
- [x] 4.2 `frame_revision`; `refresh()` reuses an unchanged frame; one image build on open; the display image built on every core; undo handlers drop the forced refresh.
- [x] 4.3 The Move tool warms only a movable layer.
- [x] 4.4 Layer lock / colour label record without compositing; blend, opacity and fill repaint the layers' bounds; a type layer is bounded by its rect.
- [x] 4.5 Banded CPU compositing; parallel canvas conversion; zero-page `Plane::build`; row-parallel pyramid shrink.
- [x] 4.6 Present without a full-resolution canvas image (`setDocument` / `replaceDocument`; `image()` built from level 0 on request).
- [x] 4.7 Keep the GPU path (owner's decision): staging writes in parallel, zero-page readback, reused readback buffer; row-parallel region patches.
- [x] 4.8 GPU residency for unchanged layer sources and coverages, keyed by plane stamps.

## 5. Verification

- [x] 5.1 Re-measure on the world map (profile + GUI); numbers in `design.md`.
- [x] 5.2 Qt Test `tst_large_document`; `bash scripts/verify-fast.sh`; `openspec validate --all --strict`.

## 6. Docs

- [x] 6.1 `docs/dev/STATE.md`: record the large-image work.

## 7. Filter follow-ups

- [x] 7.1 GPU filter passes split into bounded submissions (a whole-image pass hung the GPU past the amdgpu timeout); chunked/unchunked parity test.
- [x] 7.2 Device-loss latch: the GPU is disabled for the session, uploads no longer panic, and readbacks fail over to the CPU.
- [x] 7.3 `FilterProxy`: box-reduced copy of a document region; the Filter Gallery previews on it in its pane, with OK filtering the full layer.
- [x] 7.4 `tst_filter_gallery` previews in the pane only and at reduced size; `filter_proxy` unit tests.
