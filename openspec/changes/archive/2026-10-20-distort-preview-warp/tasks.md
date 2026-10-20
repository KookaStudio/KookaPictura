# Tasks

## 1. Tests

- [x] 1.1 Add the exact reported quad as regression tests for raster, layer-mask, vector-mask, and clipped-fill previews.
- [x] 1.2 Add a direct warp test for the horizon front side.

## 2. Implementation

- [x] 2.1 Add `image_view_transform_warp.cpp` and CMake entry.
- [x] 2.2 Warp the visible destination through the projective matrix on the CPU; skip the far side.
- [x] 2.3 Draw the warped preview in `ImageView` and keep the Qt projective draw as a fallback.
- [x] 2.4 Rebuild the warp on paint when the visible region or view scale changes, and sample at the view scale.
- [x] 2.5 Bound only a one-sided source by its projected corners; draw a preset warp's whole preview layer.

## 3. Verification

- [x] 3.1 Build and run `ctest -R '^tst_transform'`.
- [x] 3.2 Run the type, move, smart-guide, and edit-shortcut suites.
- [x] 3.3 `openspec validate distort-preview-warp --strict`, archive, and update `docs/dev/STATE.md`.
