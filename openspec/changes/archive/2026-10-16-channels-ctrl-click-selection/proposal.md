# Proposal

## Why

In the Channels panel, Ctrl-hovering a channel row shows the "select all
pixels" overlay, but Ctrl-clicking the row does not load that channel as a
selection. `ChannelsPanel::eventFilter` handled only `MouseMove` / `HoverMove`
and `Leave`, so the click fell through to `QListWidget`'s default row
selection; the footer's load path (`load_`) knew only alpha channels. The
overlay advertised an action the click did not perform (#438).

## What Changes

- `ChannelsPanel::eventFilter` gains a `MouseButtonPress` branch: a Ctrl+left
  press over a row loads that row as the selection — the composite row its
  luminance over the visible channels, each colour row its own grayscale plane,
  an alpha row its stored channel — through one shared `loadRowSelection`, which
  the footer Load button also uses.
- `PictureView::load_channel_selection(row, mask)` builds the selection from the
  document composite with the same grayscale derivation the row thumbnails use
  (`channelValue`): RGB planes, naive CMYK / Multichannel inks, otherwise the
  composite luminance. Hidden RGB channels are zeroed in the composite row,
  matching the canvas.
- `tst_channels_panel` gains a Ctrl-click case (each colour and composite row,
  the composite under a hidden channel, and a plain click that only moves the
  current row) and a Ctrl-click on an alpha row.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `ui/channels-panel`: a new "Load a channel as a selection" requirement —
  Ctrl-clicking a channel or composite row loads it as the selection; a plain
  click only moves the current row.

## Impact

- `crates/pictura-app/src/cxxqt_object/impl_selection.rs` and
  `cxxqt_object.rs` (the `load_channel_selection` bridge method and its
  grayscale derivation, with unit tests).
- `crates/pictura-app/cpp/panels/channels_panel.{h,cpp}` (the press branch and
  `loadRowSelection` / `rowAt`).
- `crates/pictura-app/cpp/tests/tst_channels_panel.cpp`.
- No new dependency. Ceiling (`ponytail:`): the Shift/Alt combine modifiers the
  CS6 docs list are not built; a Ctrl-click replaces the selection. The derived
  modes' planes are the naive conversions the thumbnails already use.
