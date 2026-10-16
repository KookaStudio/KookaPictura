# Tasks: edit-image-menu-commands

## 1. Engine

- [x] 1.1 `pictura-paint::bucket::stroke_selection` (+ `StrokeAlign`, dilate/erode) and unit tests.
- [x] 1.2 `pictura-app::History` `purge` / `purge_states` / `purge_snapshots`.

## 2. App bridge

- [x] 2.1 `paint_tools/fills.rs`: `edit_fill` and `edit_stroke`.
- [x] 2.2 `impl_history/purge.rs`: `purge_all` / `purge_history` / `purge_named_history`.
- [x] 2.3 `image_adjust/image_ops.rs`: `trim_image`, `duplicate_into`, `duplicate_name`.

## 3. C++ UI

- [x] 3.1 `fill_dialog.*`, `stroke_dialog.*`, `duplicate_image_dialog.*`.
- [x] 3.2 Enable the commands in `commands.h` / `command_tree.cpp`.
- [x] 3.3 Handlers in `frame_menus_edit.cpp` and `frame_menus_image.cpp`.

## 4. Verification

- [x] 4.1 `tst_fill_tools` (Fill / Stroke), `tst_edit_clipboard` Purge, `tst_image_ops` (Trim / Duplicate).
- [x] 4.2 `bash scripts/verify-fast.sh`.
