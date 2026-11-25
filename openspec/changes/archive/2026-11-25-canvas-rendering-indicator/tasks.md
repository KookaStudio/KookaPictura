# Tasks

## 1. Pending state

- [x] 1.1 `PictureViewRust::tiles_pending()` (`in_flight || !wanted.is_empty()`) and the `tiles_pending` qinvokable; regenerate the verb manifest.
- [x] 1.2 Qt test: requested level-0 tiles of a 5000 × 4000 document read pending until they land.

## 2. Indicator

- [x] 2.1 `RenderingIndicator` widget (`rendering_indicator.{h,cpp}`): delay and animation timers, `poke()`, corner placement, mouse-transparent; register it in the shell sources.
- [x] 2.2 `ImageView` hosts it, pokes it after each paint, and gains `LevelProvider::tilesPending`; `frame_canvas.cpp` wires it to `tiles_pending()`.
- [x] 2.3 Qt Test suite `tst_rendering_indicator` covering shows, hides, fast, mouse pass-through and idle.

## 3. Close out

- [x] 3.1 Build once; run clippy, nextest, the new suite and the canvas suites.
- [x] 3.2 Update `docs/dev/STATE.md`; review `ROADMAP.md` and `PARITY.md`.
- [x] 3.3 Archive the change.
