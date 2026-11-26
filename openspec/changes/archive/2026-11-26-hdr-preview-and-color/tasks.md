# Tasks

## Coverage

- [x] Add Qt cases for preview and cancel, PSD/PSB persistence, status reset, HDR picker and color-state mutations, lossy swatches, and native Info values, including layered above-white samples with partial alpha.
- [x] Add a codec test for TIFF resource transport and preservation of unrelated resources.

## Implementation

- [x] Expose unclipped native scene region compositing, map the HDR canvas, and bind the Preview Options and status controls.
- [x] Save and restore document-level options through PSD/PSB resources and transport them in TIFF metadata; do not claim native HDR TIFF pixel round trips.
- [x] Add HDR picker controls and connect the main foreground and background entry points. Synchronize float state across all existing color mutations and confirm lossy swatch conversion.
- [x] Route 32-bit Info sampling to the native scene data.
- [ ] Record native 32-bit Brush painting, HDR TIFF pixel round trips and Adobe preview-resource interoperability as remaining limitations in the relevant trackers when this ships.

## Deferred by request

- [ ] Run the automated tests, build and repository gates when authorized.
