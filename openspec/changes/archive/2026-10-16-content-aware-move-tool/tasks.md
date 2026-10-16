# Tasks: content-aware-move-tool

## 1. Engine

- [x] 1.1 `Adaptation` threaded through the Content-Aware synthesis (Medium unchanged).
- [x] 1.2 `pictura_paint::healing::move_layer` / `MoveOptions` (Move, Extend); unit tests.

## 2. Bridge

- [x] 2.1 `content_aware_move` in `cxxqt_object/healing.rs`; the selection follows; one "Content-Aware Move" state.

## 3. Tool

- [x] 3.1 `tool_region_drag.{h,cpp}` shared gesture; `tool_contentawaremove.cpp`; `tool_patch.cpp` on the shared base.
- [x] 3.2 Catalog row enabled, arrow cursor, options-bar row, J cycle.

## 4. Verification

- [x] 4.1 `content_aware_move` self-test (538); `shift_plain` (117) asserts the J cycle; guard (98) probes Red Eye; `bash scripts/verify-fast.sh`.
