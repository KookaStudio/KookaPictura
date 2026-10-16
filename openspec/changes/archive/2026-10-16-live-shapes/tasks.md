# Tasks: live-shapes

## 1. Engine

- [x] 1.1 `pictura_core::shape` radii, star, smoothing, `outline_in_box`; unit tests.
- [x] 1.2 `vogk` codec, `decode_vector_mask_paths`; round-trip tests; psd-tools oracle.
- [x] 1.3 `shape_layer.rs`; unit test.

## 2. Bridge

- [x] 2.1 `paths.rs` layer target; `shapes.rs` `ShapeSpec` and row helpers.

## 3. App

- [x] 3.1 `shape_dialogs`; shape tool click and overlay; Direct Selection prompt; Layers and Paths panels.

## 4. Verification

- [x] 4.1 `tst_shape_tools`; `bash scripts/verify-fast.sh` with psd-tools and ag-psd present.
