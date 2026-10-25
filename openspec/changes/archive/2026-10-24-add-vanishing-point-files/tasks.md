# Tasks: add-vanishing-point-files

## Tests

- [x] Rust: the `pictura-filters`, `pictura-codec` and `pictura-app` tests named in `test-plan.md`
- [x] Qt: new `tst_vanishing_point_files`, registered in `cpp/tests/CMakeLists.txt`

## Engine and codec

- [x] `VpPlanes::to_bytes` / `from_bytes` (`distort/vanishing_point/persist.rs`)
- [x] The planes resource helpers; `RasterMetadata` carries the planes; the JPEG APP13 and TIFF tag-34377 block on save and open

## App

- [x] `filter_tools/vanishing_point_file.rs`; `replace_document` reads the planes; the PSD/PSB, raster and recovery saves write them; the raster open reads the block
- [x] OK that changes the planes marks the document modified

## Close-out

- [x] `ROADMAP.md`, `docs/dev/doc-code-parity-gaps.md`, `docs/dev/STATE.md`
- [x] `openspec validate --all --strict`, `scripts/check-test-plan.py`, then archive the change
