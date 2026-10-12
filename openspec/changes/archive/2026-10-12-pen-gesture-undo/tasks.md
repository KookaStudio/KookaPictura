# Tasks: pen-gesture-undo

## 1. Bridge

- [x] 1.1 `PictureViewRust` gesture backup field; `path_gesture_begin` /
  `path_gesture_abort` / `path_gesture_end`.

## 2. Tool

- [x] 2.1 `PenToolHandler` session undo (`toolUndo` / `canToolUndo`, backup
  lifecycle on press/release/deactivate).

## 3. Verification

- [x] 3.1 `tst_pen_tools` cases; `bash scripts/verify-fast.sh`.
