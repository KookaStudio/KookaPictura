# Tasks

- [x] 1.1 `delete_paths` keeps the last top-level node; engine test (`delete_paths_keeps_the_last_layer`).
- [x] 1.2 `layers_can_delete` greys out the trash, the row menu entry, and Layer > Delete > Layer; remove the overridden duplicate registration.
- [x] 1.3 Qt Test `tst_last_layer`; `dropOnDelete` now drops a second layer (it relied on deleting the only one).
- [x] 1.4 `test-plan.md`.
- [x] 1.5 `can_delete_paths`: a read-only availability check sharing `delete_paths`' per-node rule, replacing the document copy in `layers_can_delete`.
- [x] 1.6 `remove_layer` (control server `layer_op` delete) goes through `delete_paths` and reports a refusal; Qt Test.
