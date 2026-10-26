# Tasks

## 1. Tests first

- [x] Extend `tst_linked_layers::linkedMovePreviewOriginMatchesTheCrop`: after a
  move the preview is recomputed (no cache hit), and an unmoved group still
  reuses it.

## 2. Fix

- [x] Add `move_group_union` and key `move_cache_valid` on it.
- [x] Store `move_prepared_union` in `compute_move_preview`; clear it with the
  rest of the move cache.

## 3. Verification and archive

- [x] Run `tst_linked_layers`, the self-test, `cargo test -p pictura_app`,
  `openspec validate --all --strict`, and the guards.
- [x] Archive the change.
