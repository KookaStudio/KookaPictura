# Tasks

## 1. Tests first

- [x] Add `tst_linked_layers::linkedMovePreviewOriginMatchesTheCrop`.
- [x] Add `tst_linked_layers::multiSelectionIsNotItsOwnReference`.

## 2. Fix

- [x] Clamp the group origin in `move_origin_opacity` to match
  `rendered_move_group`'s crop.
- [x] Add the `move_group_paths` free-function bridge.
- [x] Make `smartGuideReferences` take the moving paths; pass
  `move_group_paths(view)` from the Move tool and `{editPath}` from the Type
  tool.

## 3. Verification and archive

- [x] Run `tst_linked_layers`, `tst_smart_guides`, the self-test,
  `cargo test -p pictura_app`, `openspec validate --all --strict`, and the
  guards.
- [x] Archive the change.
