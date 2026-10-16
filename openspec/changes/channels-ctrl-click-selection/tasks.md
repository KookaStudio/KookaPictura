# Tasks

## 1. Engine

- [x] 1.1 Add `PictureView::load_channel_selection(row, mask)` to
  `crates/pictura-app/src/cxxqt_object/impl_selection.rs` with the row→grayscale
  derivation (`channel_row_selection`, `channel_value`, `cmyk_ink`, `q_gray`,
  `color_rows`) and its bridge declaration in `cxxqt_object.rs`.
- [x] 1.2 Unit tests for the derivation (`channel_row_selection_reads_the_row_plane`,
  `multichannel_rows_derive_naive_inks`, `gray_and_ink_match_the_panel_derivation`).

## 2. App

- [x] 2.1 `ChannelsPanel::eventFilter` gains the `MouseButtonPress` branch; add
  `rowAt` and `loadRowSelection`, and route the footer Load button through
  `loadRowSelection` (`crates/pictura-app/cpp/panels/channels_panel.{h,cpp}`).

## 3. Tests

- [x] 3.1 `tst_channels_panel::ctrlClickLoadsChannelSelection` (each colour and
  composite row, the composite under a hidden channel, a plain click) and a
  Ctrl-click on an alpha row in `alphaChannelsAndFooter`.

## 4. Verification

- [x] 4.1 `cargo nextest run -p pictura_app`.
- [x] 4.2 `ctest --test-dir build -R '^tst_channels_panel' --output-on-failure`.
- [x] 4.3 `openspec validate --all --strict`.
- [x] 4.4 `bash scripts/verify-fast.sh`.
