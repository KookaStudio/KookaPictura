# Tasks

## 1. Tests first

- [x] Add `tst_linked_layers::moveMultiSelectionMovesEveryLayer`.

## 2. Fix

- [x] Rewrite `move_group_paths` to return the selected layers plus their link
  sets, filtered to movable/visible/unlocked and ordered by `flatten_rows`.
- [x] Adapt `begin_move_preview`, `prepare_move_preview`, `commit_move`,
  `translate_layer`, and `move_preview` to the new signature.

## 3. Verification and archive

- [x] Run `tst_linked_layers`, the self-test, `cargo test -p pictura_app`,
  `openspec validate --all --strict`, and the guards.
- [x] Archive the change.
