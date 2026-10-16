# Tasks: add-shape-blur

## 1. Engine

- [x] 1.1 Expose `custom_shape_kernel` from `pictura-core`'s Custom Shape catalogue.
- [x] 1.2 Add `Filter::ShapeBlur` with the run-decomposed convolution in `pictura-filters/src/blur/shape.rs`.
- [x] 1.3 List it in `filter_preserves_opacity` and `preview_apron`.

## 2. App

- [x] 2.1 Move `filter_map.rs`'s choice lists to `filter_map_choices.rs` (pure move, keeps the file under the cap).
- [x] 2.2 Map the `shape-blur` kind (Radius, Shape).
- [x] 2.3 Add `FilterControl::Shape` and the `Filter ▸ Blur ▸ Shape Blur` row and menu leaf.

## 3. Verification

- [x] 3.1 Unit tests: the run decomposition equals the direct convolution for every shape (including radii wider than the image), orientation, flat/alpha/no-op, refusals.
- [x] 3.2 ImageMagick differential (`--op shape`), mapping row, kernel digests, profile entry.
- [x] 3.3 Rust mapping test; `tst_shape_blur`; the Shape Blur slot count in `tst_filter_menu`.
- [x] 3.4 Update `docs/dev/STATE.md` (`TASK-ALLOWS-DOCS`).
- [x] 3.5 Run `bash scripts/verify-full.sh` and `openspec validate --all --strict`.
