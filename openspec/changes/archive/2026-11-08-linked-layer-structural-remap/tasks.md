# Tasks

## 1. Tests first

- [x] Add `remap_after_insert` / `remap_after_delete` Rust units.
- [x] Add `linkSurvivesAddingAndDeletingLayers` and
  `linkedControlsBoxResizesTheSet` to `tst_linked_layers`.

## 2. Link remap

- [x] Add `link_remap` to `PictureViewRust`; install it in `apply_edit_with`
  instead of clearing when a command stashed one.
- [x] Remap on new layer/group, duplicate, and delete (including delete hidden).
- [x] Keep reorder/group/ungroup/merge/rasterize clearing the links.

## 3. Controls box

- [x] Make `controls_box_extents` return the link set's content union.
- [x] Add `layer_link_key` and include it in the controls-box cache key.

## 4. Verification

- [x] Run the focused Rust and Qt suites.
- [x] Run `cargo fmt --all`, clippy, and the OpenSpec/guards checks.
- [x] Archive the change and keep only `archive/` under `openspec/changes/`.
