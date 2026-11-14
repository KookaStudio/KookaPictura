# Proposal

## Why

Saving an RGB PSD that carries an alpha channel writes one channel too many: the app's display composite is RGBA, `save_view` trims that display alpha only for a recorded source mode, and the writer then counts all four display planes as color channels. In the reported file the fourth and fifth planes are byte-identical; in general the fourth is the recomposited display alpha (opaque 255 for a layerless document) and the fifth is the real extra channel, so a later read reports two extra channels instead of one. This corrupts the channel layout for every RGB document that has document-level extra channels and is saved after the composite is recomposited (found saving `OpenTS_logo2.1_Hyper-V1.psd`, issue #1115).

## What Changes

- `save_view` trims the display alpha plane of an RGB composite when the document carries document-level extra channels, not only when it carries a recorded source mode.
- The header channel count of an RGB save becomes `mode color channels + document extra channels` (3 + extra) instead of `4 + extra`.
- A document with no extra channels keeps its current behavior: its RGBA composite still supplies a fourth header channel.
- No codec or on-disk format change; the writer continues to use the composite plane count that `save_view` hands it.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `color/image-mode-conversion`: restates the `save_view` rule that a display-only RGBA composite is written from three color planes when the document carries a source mode or document-level extra channels, so an RGB save's header count is the mode's color channels plus the extra channels.

## Impact

- `crates/pictura-render/src/document_ops/mode/mod.rs` (`save_view` trim condition and its contract comment).
- `crates/pictura-render/src/document_ops/mode/tests.rs` (trim test plus a save round-trip test).
- Application save paths that call `save_view` (native save, crash recovery, PDF export) inherit the fix.
- Dirty 16/32-bit RGB saves that refresh the composite through `refresh_native_composite` also stop changing the channel count, consistent with `compositing/native-depth-save`.
