# Design

## Context

A CMYK/Lab PSD is normalized to 8-bit RGB for editing but keeps the original
CMYK/Lab planes in `source_planes` and each layer's `source_channels`. Convert
to Profile converts an RGB document's working composite and layer channels
between RGB profiles. It previously also converted every layer's native store.

## Goals / Non-Goals

**Goals:**

- Keep the 8-bit working composite and layers in sync after Convert.
- Leave source-mode native stores untouched until a mode conversion owns them.
- Reach the picker's CMYK fields from the Color panel.

**Non-Goals:**

- Converting CMYK/Lab native stores between profiles. `Image > Mode` owns that
  path.
- Rebuilding the panel's slider model; the existing picker dialog is the entry
  point (the full PAN-010 sliders stay with #319).

## Decisions

### Source-mode native stores are not RGB

`convert_document_with` passes `native = doc.source_mode.is_none()` to
`convert_layers`. The 8-bit layer channels always convert. The native store
converts only for a document whose retained store is the working RGB space.
`convert_native_document` already returns early for a recorded source mode, so
the layer path now matches it.

This keeps the existing 16/32-bit RGB behavior and refuses to reinterpret CMYK
or Lab samples as RGB.

### Color panel entry point

The panel's two `ColorSwatchWidget` buttons call
`ColorPickerDialog::getColor` with the current swatch colour. The picker already
exposes editable CMYK, Lab, HSB, RGB and hex fields, so the panel gains CMYK
entry without duplicating the picker.
