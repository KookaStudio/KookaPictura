# Tasks

## 1. Test first

- [x] 1.1 `tst_transform_controls::previewEdgeMatchesTheCommittedResample` — a
  solid layer scaled to a half-pixel edge has an anti-aliased preview row that
  matches the committed row.

## 2. Preview sampler

- [x] 2.1 `bilinear` treats taps outside the source rectangle and image as
  transparent.
- [x] 2.2 The warp allows one preview-pixel of margin around the source
  rectangle so the destination edge receives partial coverage.

## 3. Finish

- [x] 3.1 Run `tst_transform_controls`, `tst_transform_distort`,
  `tst_transform_perspective`, `tst_transform_warp`.
- [x] 3.2 Run the fast verification checks.
- [x] 3.3 Archive the change.
