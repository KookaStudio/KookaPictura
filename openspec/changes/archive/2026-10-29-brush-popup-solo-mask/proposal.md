# Proposal: brush-popup-solo-mask

## Why

Issue #1038: with the Brush painting on an active solo-mask layer, opening the
brush popup and moving the cursor back onto the canvas feel slow. Profiling a
4000×3000 document with a mask on the active layer (offscreen, RelWithDebInfo)
shows those actions are already cheap — the popup opens in 1–8 ms and a hover
repaint costs ~0.19 ms — while the debounced `refreshPanels` that runs after
every edit stalls **168–178 ms**, its bulk `channels_active_mask_thumbnail(24)`
at **~170 ms**.

`mask_channel_thumbnail` (`impl_core/display.rs`) built a **document-sized**
grayscale plane and then nearest-neighbour-downsampled it to 24 px. The Channels
panel rebuilds it on every refresh while the active layer carries a mask, so each
brush stroke on the mask (and each solo toggle) blocks the UI thread for ~170 ms.
This is the O(document)-per-refresh class the `m29-large-doc-performance`
milestone removed from `layer_thumbnail` and the histogram; this call was missed.

## What Changes

- `mask_channel_thumbnail` samples `mask_sample` directly on the thumbnail grid
  (O(24×18) ≈ 432 samples) instead of materialising the full document plane,
  through an extracted `mask_thumbnail_samples` free function. The pixels are
  **bit-identical**: the removed full-plane step's only reader was the same
  nearest-neighbour sample.
- A `#[cfg(test)]` sample counter makes the bound a deterministic test, not a
  timing gate.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `ui/channels-panel`: ADDED a requirement that the mask row's thumbnail is
  bounded by its own output size, not by the document size.

## Impact

- `crates/pictura-app/src/cxxqt_object/impl_core/display.rs`:
  `mask_channel_thumbnail`, the new `mask_thumbnail_samples`, and two unit tests.
- No behavioural change: the mask-row thumbnail's pixels are unchanged, and no
  other display consumer reads the removed plane.
- The solo display plane (`mask_display_plane`) stays O(document) but is built
  only on entering solo and on a genuine full recomposite, not per edit; the
  larger mask-paint performance item remains #665.
