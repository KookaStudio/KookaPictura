# Tasks

## 1. Tests

- [x] 1.1 Add the Rust unit tests in `test-plan.md`
- [x] 1.2 Add the Qt Test cases for the deferred commit and its signal

## 2. Engine

- [x] 2.1 Split `refresh_region_buffer`'s pure composite into `composite_region`
- [x] 2.2 Add the deferred job: threshold, generation, worker, land and settle
- [x] 2.3 Capture the move's history state on land/settle and drop stale results

## 3. App

- [x] 3.1 Defer a large `apply_edit_move` and spawn its worker
- [x] 3.2 Add the `moveCommitted` signal and keep the drag preview until it
- [x] 3.3 Settle at every named reader and make `picture()` pending-aware
- [x] 3.4 Clear a pending job when the document is replaced

## 4. Evidence

- [x] 4.1 Time the deferred commit and its settle in `move_gesture_profile`
