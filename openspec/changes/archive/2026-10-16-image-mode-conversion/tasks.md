# Tasks: image-mode-conversion

## 1. Engine

- [x] 1.1 `pictura-codec`: export the profile-free mode conversions.
- [x] 1.2 `pictura-render::document_ops::mode`: availability matrix, `convert_mode` (Grayscale / RGB / CMYK / Lab), `convert_bit_depth`, and `save_view`.
- [x] 1.3 `mode/flat.rs`: `convert_to_indexed` (flatten + quantize + palette/index stores) and `convert_to_bitmap` (Threshold / Pattern / Diffusion, packed depth-1 store).
- [x] 1.4 Unit tests with `write_psd` → `read_psd` round trips for every mode and depth.

## 2. App bridge

- [x] 2.1 `image_adjust/image_mode.rs`: mode/depth queries, conversions, the Indexed preview / cancel (`mode_preview`), one `Convert Mode` state each.
- [x] 2.2 `PictureView::save` writes through `save_view`.

## 3. C++ UI

- [x] 3.1 Checkable Mode commands with frozen ids in `commands.h` / `command_tree.cpp`.
- [x] 3.2 `frame_menus_image_mode.cpp`: handlers, check marks, the Discard / Flatten prompts, and the 32-bit HDR route.
- [x] 3.3 `indexed_color_dialog.*` and `bitmap_mode_dialog.*`.

## 4. Verification

- [x] 4.1 Qt Test `tst_image_mode`.
- [x] 4.2 `bash scripts/verify-full.sh`.
