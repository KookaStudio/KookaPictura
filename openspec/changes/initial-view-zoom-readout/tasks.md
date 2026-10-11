# Tasks: initial-view-zoom-readout

## 1. Fix

- [x] 1.1 Add the pending-settle flag to `ImageView`; set it in
      `setImage`/`setDocument`.
- [x] 1.2 Re-fit on every resize while the settle is pending.
- [x] 1.3 Re-fit in `showEvent`, synchronously and once queued, and end the
      settle on the queued pass.
- [x] 1.4 Clear the settle on every explicit user view change (`panBy`,
      `setOffset`, `setZoom`, `actualPixels`, pan press).

## 2. Verification

- [x] 2.1 `tst_initial_view`: a small document settles at 100 %, centred.
- [x] 2.2 `tst_initial_view`: a large document settles at the fit zoom,
      centred.
- [x] 2.3 `tst_initial_view`: a user zoom survives later window resizes.
- [x] 2.4 `tst_smoke`: a fractional zoom renders as the rounded integer
      percent, fits the value box, and matches the tab title.
- [x] 2.5 Full Qt Test suite, app self-test, file-size, and guard gates pass.

## 3. Housekeeping

- [x] 3.1 Move the zoom/pan view-state family to `image_view_document.cpp`
      (pure moves) to keep `image_view.cpp` under the file-size cap.
