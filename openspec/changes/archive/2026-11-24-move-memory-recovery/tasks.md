# Tasks: move-memory-recovery

## 1. Tests

- [x] 1.1 Add `a_cow_snapshot_reserves_only_its_tables`.
- [x] 1.2 Add `a_flat_plane_stays_flat_when_the_room_is_small`.

## 2. Implementation

- [x] 2.1 Reserve only the snapshot's tile tables and metadata in the recovery
  capture.
- [x] 2.2 Gate history plane tiling with `reserve_if_room` and leave the plane
  flat when it does not fit.

## 3. Verification

- [x] 3.1 The `pictura_app` nextest suite passes.
- [x] 3.2 `tst_recovery`, `tst_memory_governor`, and the Move suites pass.
- [x] 3.3 `openspec validate --all --strict` passes.
- [x] 3.4 Archive the change.
