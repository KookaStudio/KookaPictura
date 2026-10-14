# Tasks

## 1. Bound the row table

- [x] 1.1 In `crates/pictura-core/src/vector.rs`, give `Rows::build` a
  `clip: PsdRect` and clamp the row span to `clip.top - 1 .. clip.bottom + 1`
  intersected with the path's own span, in `i64`; verify the new unit test
  builds 258 rows for a clip of 256.
- [x] 1.2 Clamp each edge's row walk to the table's `lo..hi` so edges spanning
  past the clip contribute only in-range crossings; verify coverage inside the
  clip is unchanged (existing `vector_mask` tests pass).
- [x] 1.3 Give `VectorMask::inside` the `clip` parameter and use it on the
  first build; answer a row outside the table by scanning the edges directly so
  a later caller with a different clip stays correct.
- [x] 1.4 Verify `cargo nextest run -p pictura-core`.

## 2. Move `ident` off the build

- [x] 2.1 Move the coverage `ident` from `Rows` to `CoverageRows`, assigned when
  the mask is built, so `ident()` does not force an unbounded build; verify the
  feathered-plane cache still retires when the mask changes.
- [x] 2.2 In `VectorMask::set_feather`, reset `rows` so a feather change
  rebuilds the table over the grown feather clip; verify the `mask_refine`
  feather tests pass.

## 3. Thread the clip through callers

- [x] 3.1 In `crates/pictura-render/src/vector_mask.rs`, give `coverage` the
  `clip` and pass it to `inside`; update its tests.
- [x] 3.2 In `crates/pictura-render/src/mask_refine.rs`, pass the plane bounds
  to `inside` in `feather_vector` and the canvas to `coverage` in the
  non-feather branch.
- [x] 3.3 In `crates/pictura-render/src/document_ops/layer_ops/shape_layer.rs`,
  give `shape_coverage` the `clip`; update its tests.
- [x] 3.4 In `crates/pictura-app/src/cxxqt_object/helpers_composite.rs` and
  `shapes.rs`, pass the document canvas to `inside` / `shape_coverage`.

## 4. Test and verify

- [x] 4.1 Add a `pictura-core` unit test that a crafted path spanning
  `i32::MIN..i32::MAX` over a document-height clip builds a table bounded by the
  clip, plus a test that a row outside the cached clip falls back to the edges.
- [x] 4.2 Run `openspec validate --all --strict`.
- [x] 4.3 Run `bash scripts/verify-fast.sh`. fmt, clippy, all 3098 nextest
  tests, doctests, file-size, worktree hook, selftest budget, icon provenance,
  guard, and `openspec validate --all --strict` pass. The only failure is the
  pre-existing, environment-dependent
  `pictura-render::oracle::imagemagick_fixtures_reproduce` (`ramp_Divide.rgba`
  differs under this machine's ImageMagick), which also fails on `master` with
  these changes stashed. No golden baseline changed.
