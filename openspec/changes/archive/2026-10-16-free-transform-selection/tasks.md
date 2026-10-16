# Tasks: free-transform-selection

## 1. Engine

- [x] 1.1 `lift_selection`, `merge_lifted`, `can_lift_selection` (`layer_ops/move_content.rs`); unit tests.

## 2. App

- [x] 2.1 Transform session: lift on begin, restore on cancel / identity, merge back on commit; `layer_can_free_transform` counts a liftable selection.

## 3. Verification

- [x] 3.1 `tst_edit_shortcuts::freeTransformLiftsTheSelectedPixels` (Ctrl+T on an opened photo's Background: disabled without a selection; lift, Escape restores, Return commits one state and keeps the Background; undo).
- [x] 3.2 `bash scripts/verify-full.sh`; `openspec validate --all --strict`.
