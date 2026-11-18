# Proposal

## Why

The Print dialog sends an sRGB image to the printer and keeps defaults for the current session. [Issue #634](https://github.com/KookaStudio/KookaPictura/issues/634) adds control over colour conversion, proofing and marks, and requires portable print settings to survive a PSD or PSB exchange with Photoshop CS6 and Photoshop CC.

## What Changes

- Add application-managed printing with a printer ICC profile, rendering intent, and black-point compensation. Keep Printer Manages Colors as the default.
- Add Match Print Colors, Gamut Warning, and Show Paper White preview aids, plus Hard Proofing using the active View proof setup or a print-only override.
- Render crop and registration marks, calibration bars, filename labels, File Info description, background, border, and bleed using the same page layout in preview and output.
- Persist machine-local defaults across launches and portable print settings in standard PSD/PSB resources. Done and successful Print accept the settings, and Print One Copy uses the resolved settings with a one-copy override.
- Verify that Photoshop CS6 and a recorded Photoshop CC build restore the agreed portable fields in File > Print in both directions of a file exchange.

## Capabilities

### New Capabilities

### Modified Capabilities

- `interop/print` gains print colour conversion, preview and hard proofing, output marks, persistent defaults, and Photoshop print-setting interoperability.

## Impact

- The Qt print dialog and both File menu commands share job preparation, page placement, and settings resolution.
- `pictura-color` supplies Little CMS transforms. `pictura-codec` supplies ICC-aware PDF primitives and reads and writes print image resources.
- Existing page-size, paper-source, orientation, and copy-count behaviour remains part of the print contract. Physical CUPS output and Adobe interoperability require manual acceptance checks.
