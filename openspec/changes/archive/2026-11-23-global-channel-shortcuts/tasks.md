# Tasks

## 1. Tests

- [x] 1.1 Add `tst_channels_panel::channelShortcutsToggleFromTheCanvas`: with the
  canvas focused, `Ctrl+3/4/5` toggle Red/Green/Blue and `Ctrl+2` restores all
  channels.

## 2. App

- [x] 2.1 Add `ChannelsPanel::toggleChannelForShortcut(int digit)`, resolving the
  digit against the row's published shortcut and reusing `toggle()`.
- [x] 2.2 Handle `Ctrl+2`…`Ctrl+5` in `PicturaMainWindow::dispatchCommandKey` and
  call the panel entry point.

## 3. Gates

- [x] 3.1 Build `pictura_shell` and `tst_channels_panel`.
- [x] 3.2 Run `tst_channels_panel`, `python3 scripts/check-test-plan.py`, and
  `openspec validate --all --strict`.
- [x] 3.3 Archive with `openspec archive global-channel-shortcuts`.
