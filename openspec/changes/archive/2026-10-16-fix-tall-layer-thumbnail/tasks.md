# Tasks

## 1. Emit document-aspect thumbnails from the bridge

- [x] 1.1 In `layer_thumbnail_positioned` (`crates/pictura-app/src/cxxqt_object/helpers_composite.rs`), compute `tw`/`th` from the document aspect (the same rule as `layer_thumbnail_image`), allocate `tw × th`, and place the layer offset with independent `tw/doc_w` and `th/doc_h` scales; verify with task 3.1.
- [x] 1.2 In `shape_thumbnail` (`crates/pictura-app/src/cxxqt_object/shapes.rs`), sample `shape_coverage` on a `tw × th` document-aspect grid instead of `size × size`; verify a shape layer's thumbnail on a tall document is not squashed.

## 2. Tests

- [x] 2.1 Add a Rust unit test in `crates/pictura-app/src/cxxqt_object/tests.rs` asserting a full-width layer in a tall document yields a document-aspect `layer_thumbnail_positioned` image whose rightmost column is layer content (non-zero alpha), and that a square document still yields `size × size`.
- [x] 2.2 Extend `crates/pictura-app/cpp/tests/tst_layers_panel_fixes.cpp` (or add a case) asserting a tall-document full-width layer's painted row thumbnail has document aspect and no transparent right column; run `ctest --test-dir build -R '^tst_layers_panel_fixes' --output-on-failure`.

## 3. Verification

- [x] 3.1 Run `cargo nextest run -p pictura_app` and confirm the new unit test and the existing thumbnail tests pass.
- [x] 3.2 Run `openspec validate --all --strict` and confirm it passes.
- [x] 3.3 Run `bash scripts/verify-fast.sh` (after a CMake build so CTest runs) and confirm it passes; report any golden baseline change explicitly.
