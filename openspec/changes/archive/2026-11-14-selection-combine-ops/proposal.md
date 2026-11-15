# Proposal

## Why

`Select > Save Selection` and `Select > Load Selection` still open bare name/channel prompts: Save always appends a channel and discards the entered name, and Load always replaces the current selection. CS6 supports combine operations, load inversion, existing-channel and layer-mask destinations, channel names, same-size cross-document use, and modifier-click combine in the Channels panel. This is parity gap SEL-7 (issue #615).

## What Changes

- Replace both prompts with Save/Load Selection dialogs: Document, Channel, Name (new channel only), Invert (load only), and Operation.
- Save into an existing extra channel with Replace, Add, Subtract, or Intersect.
- Save into any existing raster layer mask with the same operations, preserving mask metadata; selected areas become white (revealed).
- Load an extra channel or layer mask with Invert and New, Add, Subtract, or Intersect.
- Persist channel names in the PSD alpha-name image resources (1006/1045), show them in the Channels panel and dialogs, and keep `Alpha N` as the fallback.
- Save and Load against another open document of the same pixel dimensions; Save can create a new single-channel grayscale image with the selection as its extra channel.
- Channels panel: Ctrl/Cmd-click New, +Shift Add, +Alt Subtract, +Shift+Alt Intersect on alpha, colour, and composite rows; drag a channel onto Load; Alt-click Save opens the dialog.
- Layers panel raster-mask thumbnail: Ctrl-prefixed Add/Subtract/Intersect; Ctrl alone stays New.
- Refuse a Save that would create channel 57; the engine-wide cap and anti-aliased rasterisation remain #614.
- Reuse `Selection::combine`; no second combine algebra.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `tools/select-menu`: the Save and Load Selection requirement gains the dialog contract, operations, inversion, channel and mask targets, cross-document rules, and name handling.
- `tools/selection-channels`: gains save-into-existing-channel, load-with-invert-and-operation, and channel-name round-trip requirements.
- `ui/channels-panel`: the Ctrl-click load requirement gains the Shift/Alt combine chain, custom alpha names, drag-to-load, and Alt-click Save.
- `compositing/layer-masks`: gains writing a selection into an existing raster mask and Ctrl-prefixed combine on its thumbnail.

## Impact

- `pictura-codec`: image-resource 1006/1045 read/write for alpha names; no channel-plane format change.
- `pictura-app` Rust bridge: save/load signatures, target listing, coverage transfer for cross-document operations.
- `pictura-app` C++: new Save/Load dialog, Select menu wiring, Channels panel, Layers panel mask thumbnail.
- Tests: codec resource round-trip, Rust bridge unit tests, Qt Test dialog and panel suites.
- Specs and docs: OpenSpec deltas in this change; parity register SEL-7 and ROADMAP #615 updated in a docs commit.

## Out of Scope

- Selected Areas / Masked Areas polarity (#611).
- 16/32-bit alpha and selection channel storage (#1331).
- Cross-image channel duplication and selection dragging (#1332).
- Colour channels as Save/Load targets (#1333).
- Load-dialog colour rows; they stay a Channels-panel interaction.
