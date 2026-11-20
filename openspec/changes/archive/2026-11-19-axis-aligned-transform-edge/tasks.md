# Tasks

## 1. Tests first

- [x] 1.1 `pictura-render::an_axis_aligned_scale_keeps_a_hard_edge` — a solid
  layer scaled 2× with no rotation has no fractional alpha.
- [x] 1.2 `tst_transform_controls::previewEdgeMatchesTheCommittedResample` — an
  axis-aligned resize previews and commits a hard edge.

## 2. Engine padding

- [x] 2.1 `PlaneMap::pad` returns zero for an axis-aligned similarity or
  projective map and `PAD` otherwise.
- [x] 2.2 The tiled resampler and the dense oracle use the map's padding.

## 3. Preview padding

- [x] 3.1 `warpTransformPreview` uses the clamped sampler and no edge margin for
  an axis-aligned local matrix, and the padded sampler otherwise.

## 4. Finish

- [x] 4.1 Run `pictura-render` transform tests and the transform Qt suites.
- [x] 4.2 Run the fast verification checks.
- [x] 4.3 Archive the change.
