# Proposal

## Why

Three follow-ups on the Pictura Raw dialog. The slider names sit against the
panel's left border, so the first glyph touches the edge. The HSL/Grayscale
tabs paint every Hue slider with the full spectrum and the Saturation and
Luminance sliders grey, where Camera Raw paints each of the eight range sliders
with that range's own colours. The Effects tab uses a droplet icon while the
ninth tab carries the `fx` icon and the settings actions, but Camera Raw's `fx`
is a settings menu button, not a tab, and the Effects tab is the one that reads
as `fx`.

## What Changes

- Inset every panel's content by a few pixels on the left and right so the
  slider names clear the border.
- Give the eight HSL ranges their own colour ramps (reds, oranges, yellows,
  greens, aquas, blues, purples, magentas) and use them on the Hue, Saturation,
  and Luminance tabs; keep the Grayscale Mix grey.
- Give the Effects tab the `fx` icon and remove the Presets tab from the tab
  strip. Move the settings actions to a separate `fx` button at the right of the
  tab strip that opens the same list as a popup.

The render, settings codec, and existing settings actions are unchanged.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `codec/pictura-raw`: the dialog's panel padding, HSL slider colours, tab
  iconography, and settings-menu placement change. The requirements are
  additive; the tabs, preview, settings actions, and OK/Cancel contracts stay.
