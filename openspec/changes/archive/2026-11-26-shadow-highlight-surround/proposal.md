# Proposal

## Why

Shadows/Highlights uses just two amounts and assigns the same correction to every pixel of the same luminance. Issue #562 calls for independent local correction and the eight advanced controls available in each supported document mode.

## What Changes

- Use separate neighborhood radii and tonal widths for shadow lift and highlight pull.
- Add a collapsed Show More Options section for tonal widths, radii, Color Correction or Brightness, Midtone Contrast, and the two clipping controls.
- Use the existing destructive adjustment preview and selection-aware commit. Support RGB and Grayscale at 8 and 16 bpc with native sample precision. Preserve the existing 8-bit working-space route in other modes where available.
- Preserve existing four-byte `shdH` adjustment settings and use a distinct, versioned `shd2` block for new settings.

## Capabilities

### Modified Capabilities

- `imaging/image-adjustments` gains the local Shadows/Highlights operator and controls.

## Impact

The adjustment engine, parameter codec, destructive render route and Qt adjustment dialog change together. The operator approximates the behavior described in the project contract. Smart Filter integration and CS6 settings-file interoperability are separate from the operator and controls in #562.
