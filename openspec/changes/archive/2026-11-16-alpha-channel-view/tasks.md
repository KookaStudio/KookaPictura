# Tasks

## 1. Tests first

- [x] 1.1 Add `tst_channels_panel::alphaEyeShowsChannel` for show/hide/composite and overlay pixels.
- [x] 1.2 Add `tst_channels_panel::deletingShownAlphaClearsView`.
- [x] 1.3 Update the alpha-eye row test to assert the eye is enabled and selects the row.
- [x] 1.4 Write `test-plan.md`.

## 2. Channels panel

- [x] 2.1 Add `shownAlpha_`, `shownAlpha()`, `alphaVisibilityChanged`.
- [x] 2.2 Enable alpha eyes and toggle the shown channel.
- [x] 2.3 Clear on composite/colour eye, document switch, delete, and count shrink.

## 3. Frame overlay

- [x] 3.1 Add the PanelHost refresh hook and frame wiring.
- [x] 3.2 Sync a red 50 % rubylith overlay from `channels_alpha_thumbnail`.

## 4. Verification and close-out

- [x] 4.1 `cargo fmt --all`, clippy, and `bash scripts/verify-fast.sh`.
- [x] 4.2 `openspec validate --all --strict`.
- [x] 4.3 Archive `alpha-channel-view`.
