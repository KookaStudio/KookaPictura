## ADDED Requirements

### Requirement: Brush pressure override toggles

The Brush options bar SHALL carry an **Always use Pressure for Opacity** toggle
immediately after Opacity and an **Always use Pressure for Size** toggle at the
far right after Airbrush, both off by default, both with their own icon. The two
flags SHALL persist with the brush options, and a preset or restore SHALL update
the bar.

#### Scenario: The two buttons flip the model [pressure_buttons_toggle]

- **WHEN** either pressure toggle is clicked, or the brush options are restored
  from JSON
- **THEN** the matching `PaintOptions` flag and the button agree, and both
  survive the options round trip

### Requirement: Pressure scales the brush

With Always use Pressure for Opacity on, a lower sample pressure SHALL paint
lighter, scaling the per-dab flow and the opacity cap. With Always use Pressure
for Size on, a lower sample pressure SHALL shrink the dab diameter. With both
off, pressure SHALL NOT change the stroke. A held dab SHALL keep the pressure of
the sample it repeats.

#### Scenario: Lower pressure paints lighter [pressure_opacity_lightens]

- **WHEN** the same stroke is painted at pressure 1.0 and at pressure 0.5 with
  the opacity override on
- **THEN** the half-pressure dab is lighter

#### Scenario: Lower pressure paints a smaller dab [pressure_size_shrinks]

- **WHEN** the same stroke is painted at pressure 1.0 and at pressure 0.5 with
  the size override on
- **THEN** the half-pressure dab covers less of the canvas

#### Scenario: A held dab keeps its pressure [pressure_hold_keeps]

- **WHEN** an Airbrush stroke is pressed at half pressure with the opacity
  override on and held
- **THEN** the build-up stops at the half-pressure cap instead of the full one

#### Scenario: The pressure helper is testable [pressure_scaled_helper]

- **WHEN** the scaling helper is called with each override on and off
- **THEN** it scales only the fields its override names

### Requirement: Tablet pressure reaches the brush

The canvas SHALL take a tablet event's pressure and pass it to the active paint
stroke, and a non-synthesized mouse press SHALL report full pressure. The reduced
level preview SHALL apply the same per-sample pressure as the exact stroke.

#### Scenario: A tablet press sets the dab pressure [pressure_tablet_routing]

- **WHEN** a tablet press and move arrive over the canvas
- **THEN** the next dab uses the tablet's pressure rather than full pressure
