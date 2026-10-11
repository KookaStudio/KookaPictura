# Tasks: expose-custom-filter

## 1. App

- [x] 1.1 Map the `custom` kind to 27 slots (weights row by row, Scale, Offset), defaulting to the pass-through kernel.
- [x] 1.2 Add `FilterControl::Matrix` and `FilterControl::Number` to the shared filter controls.
- [x] 1.3 Add the `Filter ▸ Other ▸ Custom` row ahead of High Pass, in CS6's menu order.

## 2. Verification

- [x] 2.1 Add a Rust mapping test for the defaults, slot order, and arity.
- [x] 2.2 Add `tst_custom_filter` (dialog, menu preview and commit, Last Filter) and the Custom slot count to `tst_filter_menu`.
- [x] 2.3 Update `docs/dev/STATE.md` (`TASK-ALLOWS-DOCS`).
- [x] 2.4 Run `bash scripts/verify-full.sh` and `openspec validate --all --strict`.
