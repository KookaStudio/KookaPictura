# Proposal

## Why

File > Automate > Merge To HDR Pro is disabled. Photographers need to combine bracketed captures into an editable image that retains highlight and shadow detail.

## What Changes

- Accept at least three bracketed files at up to 8 bits per channel; explicitly refuse higher-depth inputs until native-depth import is implemented.
- Accept sources from a picker, folder, or clean saved open documents without modifying inputs.
- Resolve exposure metadata with explicit manual fallback, optionally align translations and suppress ghosts using a selectable base, and merge linear floating-point radiance.
- Preview with adjustable brightness, exchange Pictura response curves and presets, and atomically publish an untitled 32-, 16-, or 8-bit document.
- Record as a #558 remaining parity gap that preview white point is not persisted when PSD, PSB, or TIFF is reopened; the existing document display has no persistent white-point field.

## Capabilities

### New Capabilities

- `imaging/hdr-merge` covers exposure-bracket selection, fusion, preview, and output.

### Modified Capabilities

None.

## Impact

Rust HDR fusion, the Qt File menu and dialog, the existing HDR depth-conversion path, and regression tests. No new dependencies.
