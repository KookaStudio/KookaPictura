# Tasks

## 1. Tests first

- [x] Add `a_linked_transform_includes_a_hidden_partner` and
  `a_linked_transform_skips_a_position_locked_partner` Rust units.
- [x] Add `linkedFreeTransformRotatesTheSet`, `hiddenLinkedMemberTransformsToo`,
  and `linkedSkewTransformsTheSet` to `tst_linked_layers`.

## 2. Session model

- [x] Add `members` and per-member `untrimmed` to `TransformSession`.
- [x] Make `prepare_transform` target the link set, including hidden members and
  skipping position-locked or untransformable ones.
- [x] Restore every member's pre-trim extent on cancel.

## 3. Preview

- [x] Render the visible members over transparency for the transform preview and
  hide every member in the backdrop.
- [x] Apply the same warp mesh to every member in the warp preview.

## 4. Commit

- [x] Apply the similarity, skew, projective, and warp maps to every member.
- [x] Carry multiple paths through the deferred transform and land every layer.
- [x] Keep the link set: drop `clear_link_sets` and use region/full edits.

## 5. Verification

- [x] Run the focused Rust and Qt suites.
- [x] Run `cargo fmt --all`, clippy, and the OpenSpec/guards checks.
- [x] Archive the change and keep only `archive/` under `openspec/changes/`.
