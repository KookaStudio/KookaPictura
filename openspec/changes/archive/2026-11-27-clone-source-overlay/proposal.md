# Proposal

## Why

The Clone Source panel exposes overlay controls but disables them, so Clone Stamp and Healing Brush users cannot preview their source to align a retouch. Issue #537 requests the documented overlay controls.

## What Changes

- Enable Show Overlay, Auto Hide, Clipped, Opacity, Normal/Darken/Lighten/Difference mode, and Invert for Clone Stamp and Healing Brush.
- Draw a transient source preview using the active source mapping and brush footprint; never change image pixels or history through the preview.
- Retain overlay settings in session state. Keep frame controls, overlay movement shortcuts, and other unimplemented source features outside this change.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ui/clone-source-panel`: enable and specify the source overlay and its controls.

## Impact

Qt panel, tool state and canvas feedback, including Healing Brush source integration; Qt Test coverage. No new dependency is planned. The contract marks overlay opacity default and blend math as unverified CS6 behavior; this implementation is an approximation, not verified Adobe parity.
