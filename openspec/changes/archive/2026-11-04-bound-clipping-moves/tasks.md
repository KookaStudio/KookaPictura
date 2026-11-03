# Tasks

## 1. Tests

- [x] 1.1 Add `releasing_a_clipping_mask_damages_the_freed_run` in
  `crates/pictura-render/src/damage.rs`: a clicked middle member frees a
  full-canvas member above it; assert the freed path is in the damage refs and
  that the expanded damage covers the canvas and a region composite over it
  equals the full composite.
- [x] 1.2 Add `inert_bottom_move_under_a_knockout_is_not_neutral` in
  `crates/pictura-render/src/document_ops/layer_ops/neutral.rs`.
- [x] 1.3 Add `inserted_edit_with_pending_preview_rebuilds` in
  `crates/pictura-app/src/cxxqt_object/damage_policy_tests.rs`.

## 2. Clipped-member damage

- [x] 2.1 In `crates/pictura-render/src/damage.rs` `content_influence`, return
  the base's reach for a clipped layer with a non-Background base; keep the
  union for an unbased run or a Background base.
- [x] 2.2 Expose `clipping_release_paths` in
  `crates/pictura-render/src/document_ops/layer_ops/clipping.rs` and use it in
  `release_clipping_mask`.
- [x] 2.3 Export `clipping_release_paths` from `layer_ops`, `document_ops`, and
  `lib.rs`.
- [x] 2.4 In `crates/pictura-app/src/cxxqt_object/clipping/imp.rs`
  `clipping_release`, pass the union of the clicked and freed paths to
  `batch_changed`.

## 3. Neutral moves

- [x] 3.1 In `crates/pictura-render/src/document_ops/layer_ops/neutral.rs`
  `move_is_content_neutral`, return false when either document contains a
  knockout.
- [x] 3.2 In `crates/pictura-app/src/cxxqt_object/edit.rs`, upgrade
  `Edit::Inserted` to a full structural edit when a pending preview is active.

## 4. Verification

- [x] 4.1 `cargo fmt --all` (CI runs the build, tests, and clippy).
- [x] 4.2 `openspec validate --all --strict`.
- [x] 4.3 `bash scripts/check-openspec-archived.sh`.

## 5. Archive

- [x] 5.1 Archive the change with `openspec archive bound-clipping-moves -y`.
