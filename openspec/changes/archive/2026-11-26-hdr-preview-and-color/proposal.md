# HDR preview and color

## Why

HDR users need to inspect highlights and pick or measure scene colors above display white. Issue #570 tracks the three missing controls.

## What Changes

- Add a 32-bit Preview Options command and a per-view status-bar exposure control.
- Add an HDR foreground and background picker with float RGB, intensity and stepped previews; synchronize float state with existing color changes.
- Show floating-point scene samples in both Info readouts on 32-bit documents.
- Preserve Pictura preview metadata in PSD, PSB and TIFF resources. Full HDR TIFF pixel round trips, native float Brush painting and Adobe preview-setting interoperability remain open, as described in the design.

## Capabilities

### New Capabilities

- `color/hdr-preview-and-color` covers HDR presentation, color selection, and Info readouts.

### Modified Capabilities

None.

## Impact

The renderer, PSD and TIFF metadata path, Qt canvas and color controls, and Info panel are affected. No dependencies are added.
