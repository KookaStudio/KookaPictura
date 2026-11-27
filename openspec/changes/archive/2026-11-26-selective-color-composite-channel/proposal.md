# Selective Color channel availability

## Why

Selective Color can currently launch while another channel is selected or displayed. Issue #577 calls for the composite-channel restriction and the CS6 preset whitelist. The shared whitelist already shipped, so this change adds Selective Color coverage for its existing preset behavior.

## What Changes

- Gate Selective Color in Image > Adjustments, Layer > New Adjustment Layer, the Adjustments panel tile and flyout, and the Layers footer menu.
- Use Channels panel state for both availability and dispatch. Restore availability when the active document returns to its composite channel.
- Cover the existing Selective Color Save and Load Preset behavior through the Properties panel and destructive adjustment dialog.

## Capabilities

### New Capabilities

- `ui/selective-color-channel-availability` governs when users can launch Selective Color.

### Modified Capabilities

- `imaging/adjustment-presets` gains a Selective Color scenario under Load Preset.

## Impact

The Qt shell owns channel availability. Selective Color math, file encoding, stored presets, and rendering of existing layers use their existing paths.
