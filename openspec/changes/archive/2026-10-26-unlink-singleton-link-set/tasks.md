# Tasks

## 1. Tests first

- [x] Add `tst_linked_layers::unlinkClearsTheSet` (link two, unlink one, assert
  neither row is linked and `select_linked` is empty).
- [x] Update the `lpr_link_linked_unlink` self-test: after unlinking one of two,
  `select_linked` on the partner is empty.

## 2. Fix

- [x] Add `prune_singleton_links` and call it from `apply_link` on unlink.

## 3. Verification and archive

- [x] Run `tst_linked_layers`, the self-test, `cargo test -p pictura_app`,
  `openspec validate --all --strict`, and the guards.
- [x] Archive the change.
