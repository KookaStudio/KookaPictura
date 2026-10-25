# Proposal

## Why

Convert to Profile corrupts layered 16/32-bit CMYK and Lab documents. The
composite is converted correctly, but the retained native layer store is also
converted as if its channel ids 0, 1 and 2 were RGB. A later recomposite (the
Preview path, Flatten, the eyedropper's layer sampling) then shows the corrupted
layer data, which reads as a brown image (#947).

The Color panel's foreground and background swatches only change the active
swatch; they do not open the Color Picker, so the panel cannot reach the picker's
CMYK fields (#319).

## What Changes

- Convert to Profile converts the 8-bit working channels of a layered document
  and leaves a retained native store in a recorded source mode (CMYK/Lab)
  unchanged. The native store is not RGB; the working channels already carry the
  profile conversion.
- Clicking the Color panel's foreground or background swatch opens the Color
  Picker for that swatch and applies an accepted colour.
- Regression tests cover a 16-bit CMYK layer through Convert and the Color panel
  swatch entry point.

## Capabilities

### Modified Capabilities

- `color/color-profile-assignment`: Convert to Profile distinguishes an RGB
  native store from a source-mode native store.
- `ui/color-swatches-panel`: the Color panel swatches open the Color Picker.

## Impact

- `crates/pictura-codec/src/icc.rs`, `crates/pictura-codec/src/icc_convert.rs`
- `crates/pictura-app/cpp/panels/color_panel.cpp`
- Rust and Qt Test coverage; no dependency or asset changes.
