# Proposal

## Why

Alpha-channel eyes now show a channel, but only one at a time. CS6 lets several
alpha channels stay visible together. The panel should toggle each eye
independently and combine the visible channels on the canvas.

## What Changes

- Keep a set of shown alpha channels instead of one index.
- Clicking an alpha eye toggles that channel without clearing the others.
- The canvas draws each shown alpha channel as a red 50 % rubylith overlay;
  compositing the overlays makes the red deepen as channels are added.
- The composite eye and colour-channel eyes clear the whole set.
- Deleting a shown channel removes it and shifts the remaining indices;
  switching documents clears the set.

## Capabilities

### Modified Capabilities

- `ui/channels-panel`: the alpha-channel visibility requirement now allows
  multiple shown channels and defines the combined view.

## Impact

- `ChannelsPanel` state/signal and `frame_mask_overlay` overlay composition.
- Tests: `tst_channels_panel::alphaEyesShowMultiple`,
  `tst_channels_panel::alphaEyesAccumulateRedness`.

## Out of Scope

- Per-channel overlay colours, opacity, and Masked/Selected Areas options.
- Separate windows for each alpha channel.
