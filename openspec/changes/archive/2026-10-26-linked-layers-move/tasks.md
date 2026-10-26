# Tasks

## 1. Tests first

- [x] Add `tst_linked_layers` suite with `stripButtonsCarryIcons`,
  `stripLinkButtonToggles`, `linkedRowShowsChainBadge`,
  `unlinkedRowShowsNoChainBadge`, `dragLinkedLayersMovesSet`,
  `dragUnlinkedLayerAlone`, and `lockedMemberStaysPut`; register it in
  `crates/pictura-app/cpp/tests/CMakeLists.txt`.
- [x] Add `translate_move_group_moves_every_member` and
  `translate_move_group_skips_locked` Rust units.

## 2. Row badge

- [x] Add `showsLink` / `linkBadgeRect` and paint `layers.link2` in
  `layers_panel_internal.h`.
- [x] Reserve the link advance in `badgesRight` and `nameRect`.
- [x] Add a `rowLinkBadgeRectForTest` panel hook.

## 3. Move group

- [x] Add `move_group_paths` and `link_members` to the transform session.
- [x] Generalize `build_move_preview_base` to a path slice and add
  `rendered_move_group`.
- [x] Route `compute_move_preview`, `move_cache_valid`, `commit_move`,
  `translate_layer`, and `move_preview` through the group.
- [x] Store the group key in `move_prepared_layer`; update the clears.

## 4. Strip button

- [x] Add `linked_together` bridge query and `linkButton_` member.
- [x] Enable the Link strip button in `syncStripButtons`, wire its click, and
  swap the link/unlink icon.
- [x] Update the self-test's strip expectations (link now follows the
  selection, not permanently disabled).

## 5. Docs and verification

- [x] Update `docs/05-layers/layer-management-ui.md` and
  `docs/02-ui-ux/panels/layers-panel.md`.
- [x] Update `docs/dev/STATE.md`.
- [x] Run the focused Qt suites, `cargo test -p pictura_app`, `openspec
  validate --all --strict`, and the guards.
- [x] Archive the change and keep only `archive/` under `openspec/changes/`.
