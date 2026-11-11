# tools/brush-dynamics-option-sets Specification

## Purpose

The Brush panel's Control model and the Shape Dynamics, Scattering, Color
Dynamics, Transfer, and Brush Pose option sets, including tablet inputs and
per-set padlocks.

## ADDED Requirements

### Requirement: Control inputs

A dynamics attribute SHALL take a Control of Off, Fade (1–9999 dabs), Pen
Pressure, Pen Tilt, Stylus Wheel, Rotation, Initial Direction, or Direction.
Initial Direction and Direction SHALL be offered only for Angle. A Fade of zero
SHALL behave as one dab. The control signal SHALL be combined with jitter as
`lerp(minimum, base, signal)` followed by the jitter draw, so Control Off with
zero jitter reproduces the previous value exactly.

#### Scenario: Fade ramps to the minimum

- **WHEN** a stroke uses Fade with 4 steps and 100 % size jitter is off
- **THEN** the first dab is at the base size, the fifth dab is at the minimum diameter, and later dabs stay at the minimum

#### Scenario: Pressure raises the controlled value

- **WHEN** the same stroke is painted at pressure 0.25 and 0.75 with Pen Pressure size control and a 20 % minimum diameter
- **THEN** the higher-pressure dab is larger and neither exceeds the base diameter

#### Scenario: Control Off and zero jitter are unchanged

- **WHEN** a stroke with every Control Off and every jitter at zero is painted
- **THEN** its pixels are identical to the same stroke before this change

### Requirement: Shape Dynamics

Shape Dynamics SHALL evaluate size, angle, and roundness per dab. Size SHALL be
floored by Minimum Diameter, roundness by Minimum Roundness, and Angle Control
SHALL accept Initial Direction and Direction. Tilt Scale SHALL scale the tip
before a tilt-driven rotation, and Brush Projection SHALL map pen tilt onto the
tip angle and roundness.

#### Scenario: Direction follows the stroke tangent

- **WHEN** Angle Control is Direction and the stroke turns from 0° to 90°
- **THEN** the later dab's long axis follows the later tangent instead of staying at the first direction

#### Scenario: Minimum diameter floors size jitter

- **WHEN** Size Jitter is 100 %, Minimum Diameter is 40 %, and the base is 100 px
- **THEN** no dab is smaller than 40 px

#### Scenario: Tilt projects the tip

- **WHEN** Brush Projection is on and the pen reports tilt
- **THEN** the dab's angle and roundness change with the tilt

### Requirement: Scattering controls

Scattering SHALL support Scatter Control, Both Axes, Count Control, and Count
Jitter. With Both Axes on, offsets SHALL be radial; with it off, they SHALL be
perpendicular to the stroke tangent. Count Jitter SHALL vary the number of marks
per step within 1–16.

#### Scenario: Both Axes chooses the distribution

- **WHEN** a horizontal stroke is scattered with Both Axes on and then off
- **THEN** the on stroke spreads in both axes and the off stroke spreads mainly across the path

#### Scenario: Count jitter stays in range

- **WHEN** Count Jitter is 100 % with a base count of 1
- **THEN** every step lays at least one mark and never more than 16

### Requirement: Transfer

Transfer SHALL evaluate an Opacity Jitter and a Flow Jitter per dab, each with a
Control. The evaluated opacity SHALL never exceed the options-bar Opacity, and
the evaluated flow SHALL never exceed the options-bar Flow.

#### Scenario: Pressure lowers opacity

- **WHEN** Opacity Jitter is 100 % with Pen Pressure control and pressure is 0.5
- **THEN** the dab's applied opacity is at most half the options-bar Opacity

#### Scenario: Zero jitter leaves transfer alone

- **WHEN** both Transfer jitters are zero
- **THEN** the stroke paints with the options-bar Opacity and Flow

### Requirement: Color Dynamics

Color Dynamics SHALL support Apply Per Tip, Foreground/Background Jitter with a
Control, Hue/Saturation/Brightness Jitter, and Purity (−100…100). With Apply
Per Tip off, the stroke SHALL choose one color at its start and reuse it for
every dab; with it on, each dab SHALL draw its own color. Purity −100 SHALL
fully desaturate and +100 SHALL fully saturate.

#### Scenario: Per-stroke color is constant

- **WHEN** Apply Per Tip is off and Foreground/Background Jitter is 100 %
- **THEN** every dab in the stroke uses the same color

#### Scenario: Per-tip color varies

- **WHEN** Apply Per Tip is on and Hue Jitter is 100 %
- **THEN** at least two dabs in a long stroke differ in hue

#### Scenario: Purity desaturates

- **WHEN** Purity is −100 %
- **THEN** the painted color has equal red, green, and blue channels

### Requirement: Brush Pose

Brush Pose SHALL carry Tilt X, Tilt Y, Rotation, and Pressure, each with an
Override toggle. With an Override on, that value SHALL replace the device input
for every control that reads it; with it off, the device value SHALL pass
through. The canvas SHALL forward tablet tilt and rotation to the active stroke.

#### Scenario: Override pressure feeds controls

- **WHEN** Pressure Override is on at 50 % and Size Control is Pen Pressure
- **THEN** a mouse stroke behaves as if pressure were 0.5 for every dab

#### Scenario: Tablet tilt reaches the stroke

- **WHEN** a tablet event reports tilt and rotation over the canvas
- **THEN** the next dab's dynamics receive those values

### Requirement: Per-set padlocks

Each option set SHALL have a padlock. When a set is locked, applying a brush
preset SHALL retain that set's stored values; when it is unlocked, the preset
SHALL replace them. Disabling a set SHALL NOT discard its values.

#### Scenario: A locked set survives a preset

- **WHEN** Scattering is locked with a custom scatter and a preset is chosen
- **THEN** the custom scatter remains and the preset's other sets apply

#### Scenario: An unlocked set follows the preset

- **WHEN** Shape Dynamics is unlocked and a preset is chosen
- **THEN** the preset's size, angle, and roundness dynamics replace the old ones

### Requirement: Dynamics persistence

Every dynamics value, Control, Fade step count, padlock, and set enable state
SHALL persist in the tool-options JSON and SHALL restore the panel. Missing
fields SHALL fall back to defaults.

#### Scenario: A dynamics round trip

- **WHEN** dynamics are changed, the session is saved, and a new window starts
- **THEN** every changed value and padlock is restored

#### Scenario: Old settings still load

- **WHEN** a saved session has only the five legacy dynamics fields
- **THEN** the new controls load at their defaults and the legacy values survive
