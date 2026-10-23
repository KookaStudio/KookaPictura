# Tasks

## 1. Tests first

- [x] 1.1 Add `tst_merge_commands::layerMergeSubmenu`: the Layer menu ends with a
  `Merge` submenu whose entries are the six merge commands in order, and none of
  them is a direct Layer action.
- [x] 1.2 Update `tst_command_tree::layerMenuGroupsAndDelete` so the merge group
  ends at the `Merge` submenu.
- [x] 1.3 Update `tst_merge_commands::mergeMenuItems` to search the `Merge`
  submenu, keeping the existing enabled/disabled assertions.

## 2. Implementation

- [x] 2.1 Point the six merge entries in `command_tree.cpp` at
  `{"Layer", "Merge", "<label>"}`.
- [x] 2.2 Build and run `tst_command_tree`, `tst_merge_commands`, and the
  offscreen self-test.

## 3. Verification

- [x] 3.1 `openspec validate --all --strict`
- [x] 3.2 `python3 scripts/check-test-plan.py`
