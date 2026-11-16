# Tasks

## 1. Tests

- [x] 1.1 Add `tst_blur_gallery_dialog::zoomFieldWheelAndShortcuts`: the field
  opens on the fit percentage, a typed `200%` applies, the wheel zooms in and
  out, `Ctrl+=` / `Ctrl+-` step the zoom, and Fit in View restores the fit.

## 2. Preview canvas

- [x] 2.1 Add the document-relative zoom, fit zoom, anchored zoom, and fit view
  to `BlurGalleryCanvas`, placing the picture through a focus fraction.
- [x] 2.2 Zoom on the wheel about the pointer and re-fit on resize.
- [x] 2.3 Emit `zoomChanged` for the dialog's field.

## 3. Workspace

- [x] 3.1 Add the bottom-left `−` / `+` buttons and the editable zoom field with
  the presets and named views to the options bar.
- [x] 3.2 Wire the shortcuts, the field's typed percentage, and the live label
  sync.

## 4. Gates

- [x] 4.1 Build `tst_blur_gallery_dialog` and run it with the other Blur Gallery
  suites.
- [x] 4.2 `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo fmt --all --check`, `python3 scripts/check-test-plan.py`,
  `openspec validate --all --strict`.
- [x] 4.3 Archive with `openspec archive blur-gallery-zoom`.
