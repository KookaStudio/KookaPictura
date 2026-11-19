# Tasks: vector-mask-path-authoring

## 1. Engine

- [x] 1.1 Replace `VectorMaskKind::CurrentPath` with `Path(&[Subpath])` in `add_vector_mask`.
- [x] 1.2 Add `vector_mask_paths` (any layer) and route `layer_shape_paths` through it.
- [x] 1.3 Keep the flags word in `set_layer_shape_paths`.
- [x] 1.4 Add `VectorPath::set_editing_subpath`.

## 2. App bridge

- [x] 2.1 Add `ActivePath::VectorMask`, the `-4` index in `paths_active`, `paths_set_active`, and `paths_outline`, and `paths_vector_mask_name`.
- [x] 2.2 Extend `layer_target`, `path`, `begin_drawing`, the gesture backup, and the session restore to the mask target; keep the Pen's editing subpath across decodes.
- [x] 2.3 Resolve Current Path from the active row; add `vector_mask_current_path_available`.
- [x] 2.4 Outline the vector-mask thumbnail.

## 3. Shell

- [x] 3.1 Paths panel vector-mask row, selection, Delete Path, and footer.
- [x] 3.2 Vector-mask thumbnail click activates or deactivates the row.
- [x] 3.3 Current Path enable provider uses the selected row.

## 4. Tests and docs

- [x] 4.1 Engine and bridge unit tests; `tst_paths_panel` and `tst_layers_mask_commands` cases.
- [x] 4.2 Tick `ROADMAP.md`, update the parity-gap register and `STATE.md`.
