# Tasks: patch-tool

## 1. Engine

- [x] 1.1 `pictura_paint::healing::patch_layer` / `PatchOptions` (Source, Destination, Transparent, Content-Aware); unit tests.

## 2. Bridge

- [x] 2.1 `patch_selection` in `cxxqt_object/healing.rs`; one "Patch Tool" state.

## 3. Tool

- [x] 3.1 `tool_patch.cpp` (outline, then drag the outline); catalog row enabled; options-bar row.
- [x] 3.2 J-group flyout cycles Spot Healing, Healing, and Patch.

## 4. Verification

- [x] 4.1 `patch_tool` self-test (537); `shift_plain` (117) asserts the J cycle; guard (98) probes Content-Aware Move; `bash scripts/verify-fast.sh`.
