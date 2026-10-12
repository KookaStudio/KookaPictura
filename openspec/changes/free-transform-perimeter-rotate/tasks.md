# Tasks: free-transform-perimeter-rotate

## 1. Hit testing

- [x] 1.1 `geometry.rs`: replace the corner-only rotate band with the minimum
  point-to-edge distance over the four quad sides (`point_segment_dist2`),
  keeping handles-first, inside-move, and the projective early return.
- [x] 1.2 Unit tests: edge midpoints on all four sides rotate; beyond the band
  is `-1`; the band is screen-space at zoom; projective outside is `-1`.

## 2. App

- [x] 2.1 `tst_transform_options`: an edge-midpoint press in the band rotates
  and changes the angle; a press 40 px beyond the band leaves the session
  active and adds no history state.

## 3. Verification

- [x] 3.1 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D
  warnings`, `cargo nextest run --workspace`, `ctest --test-dir build -R
  '^tst_'`, `bash scripts/verify-fast.sh`, `openspec validate --all --strict`.
  The unified report's only failure is the pre-existing local ImageMagick
  fixture mismatch, which fails on master too.
