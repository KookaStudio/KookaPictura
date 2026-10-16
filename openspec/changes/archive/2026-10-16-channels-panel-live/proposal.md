# Proposal: channels-panel-live

## Why

Issue #71: port photorust's Channels panel (`shell/src/panels/ChannelsPanel.cpp`).
Kooka's panel was presentational — composite and RGB rows only, eyes that
changed nothing, no alpha channels (the deferred channel-visibility,
channel-mask, and enumeration work recorded in STATE.md).

## What Changes

- `panels/channels_panel` rewritten: per-mode rows (RGB, Gray, CMYK, Lab,
  Multichannel, Indexed, Bitmap, Duotone — the opened file's mode, from the new
  `channels_mode`) with CS6's Ctrl+number labels and greyscale thumbnails
  (CMYK / Multichannel inks derived from the RGB working data), then the alpha
  channels ("Alpha 1"…, continuing the shortcuts) with their own thumbnails.
- Live visibility: the working RGB channels' eyes emit a channel mask; the
  composite eye is on only while every channel is, and shows them all; the last
  visible channel cannot be hidden. `ImageView::setChannelMask` draws hidden
  channels away with one multiply fill, and a single visible channel as
  greyscale (CS6's default). The mask follows the active document.
- Footer: Load Channel as Selection, Save Selection as Channel, Create New
  Channel (black, one "New Channel" state), Delete Current Channel (one
  "Delete Channel" state).
- `cxxqt_object/channels.rs` (new bridge): `channels_mode`,
  `channels_alpha_thumbnail`, `channels_new_alpha`, `channels_delete_alpha`.
- Tests: Qt Test `tst_channels_panel` (its local-toggle case replaced by canvas
  pixel checks; alpha-channel and footer cases added) and a bridge unit test.

## Capabilities

### Modified Capabilities

- `ui/channels-panel`: rows follow the document's mode and list alpha channels;
  the eyes hide channels on the canvas; the presentational-only requirements
  are removed.

## Impact

- `pictura-app` (bridge, C++). No new dependency.

## Provenance

Rows, shortcuts, mask semantics, and footer from photorust; CS6 behaviour from
the Channels panel spec. Ceiling (`ponytail:`): eyes for CMYK / Lab / other
source modes are disabled (their channels are derived from RGB working data);
alpha-channel overlays and channel targeting for edits are not modelled; Lab
thumbnails show lightness.
