# Test Plan

One row per scenario in `specs/ui/color-picker/spec.md`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `cp_field_cursor` | Construct a `ColorPickerDialog` and read its colour field's cursor | The field's cursor is a custom bitmap cursor (the loaded asset), not the crosshair | `tst_color_picker::fieldCursorIsACircle` |

## Manual checks

- Open the picker from the toolbox foreground swatch and hover the colour
  field: the pointer is a ring, and it stays legible over light and dark field
  colours. Visual cursor appearance cannot be asserted offscreen; the automated
  check covers that the circular asset is the one loaded.
