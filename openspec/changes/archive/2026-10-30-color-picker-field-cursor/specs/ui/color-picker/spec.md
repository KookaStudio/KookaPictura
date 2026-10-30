## ADDED Requirements

### Requirement: The colour field's pointer is a circle

The Color Picker's colour field SHALL show a circular pointer while the pointer
is over it, matching the ring the field paints at the current colour, rather
than the platform crosshair. The field SHALL fall back to the crosshair when
the cursor asset cannot be loaded.

#### Scenario: The field pointer is a ring [cp_field_cursor]

- **WHEN** a `ColorPickerDialog` or the Color panel builds its colour field
- **THEN** the field's cursor is the circular asset, not the crosshair
