# Proposal

## Why

The Channels panel lists alpha channels, but their eye icons are disabled, so a
saved selection or a duplicated channel cannot be inspected on the canvas. This
is the remaining manual-testing blocker for #1332 and a PAN-002 parity gap.

## What Changes

- Enable the eye of each alpha channel row.
- Clicking an alpha eye shows that channel on the canvas as a red 50 %
  rubylith overlay over its masked (black) area; clicking it again restores the
  composite.
- Clicking the composite eye or a colour-channel eye restores the composite.
- The view is display-only: it does not alter channel pixels, the selection,
  history, or the saved document.
- Deleting the shown channel, switching documents, or reducing the alpha count
  clears the view.
- The view is built from the channel coverage scaled to the canvas, so very
  large documents do not allocate a full-resolution overlay.

## Capabilities

### Modified Capabilities

- `ui/channels-panel`: gains alpha-channel eye visibility.

## Impact

- `pictura-app` C++: Channels panel state/signal, panel-host refresh hook, and
  the canvas overlay sync.
- Tests: `tst_channels_panel`.
- Docs: channels-panel parity note and `ROADMAP.md` if needed.

## Out of Scope

- Editing alpha channels with paint tools.
- Per-channel overlay colour, opacity, and Masked/Selected Areas options.
- Alpha-channel views in a separate window.
