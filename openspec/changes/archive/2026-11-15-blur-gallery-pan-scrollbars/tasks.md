# Tasks

## 1. Tests

- [x] 1.1 Add `tst_blur_gallery_dialog::scrollbarsAndMiddleDragPan`: the fit
  hides both scrollbars, zooming in shows them, scrolling moves the picture,
  and a middle drag pans it.

## 2. Preview canvas

- [x] 2.1 Add the horizontal and vertical scrollbars, ranged over the overflow
  and placed inside the pasteboard margin.
- [x] 2.2 Place a fitted axis centred and an overflowing axis by its scrollbar.
- [x] 2.3 Keep the wheel anchor with the scrollbar value and pan on a
  middle-button drag.

## 3. Gates

- [x] 3.1 Build `tst_blur_gallery_dialog` and run the Blur Gallery suites.
- [x] 3.2 `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo fmt --all --check`, `python3 scripts/check-test-plan.py`,
  `openspec validate --all --strict`.
- [x] 3.3 Archive with `openspec archive blur-gallery-pan-scrollbars`.
