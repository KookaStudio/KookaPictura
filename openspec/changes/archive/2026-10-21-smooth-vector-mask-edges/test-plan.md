# Test plan — Antialias vector-mask and shape edges

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `vma_partial` | Sample a whole-pixel square, a half-pixel strip, and a diagonal triangle | 255/0 on whole pixels; 128 on the half and diagonal pixels, cached or not | `pictura-core::coverage_is_exact_on_whole_pixels_and_partial_on_a_slanted_edge` |
| `vma_bound` | Sample a crafted full-range path over a 256-row clip, then far outside it | 258 cached rows; the uncached row is 255 | `pictura-core::smooth_coverage_bounds_its_rows_to_the_clip` |
| `vma_rotated` | Rotate a red rectangle shape 30° and commit | Blended edge pixels in the composite | `tst_shape_transform::rotatedShapeEdgesAreSmooth` |
| Visual check | Draw a star and an ellipse, rotate and distort them, zoom in | Edges look smooth, comparable to CS6 | manual: perceived edge quality against Photoshop |
