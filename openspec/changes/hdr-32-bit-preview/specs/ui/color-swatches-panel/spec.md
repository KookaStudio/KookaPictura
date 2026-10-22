# Spec Delta

## ADDED Requirements

### Requirement: HDR swatch values

A swatch added from the HDR color picker SHALL retain the float triple alongside
its `QColor` mirror. Clicking an HDR swatch SHALL restore both the float triple
to the foreground HDR state and the clamped `QColor` to the foreground colour.
The float triple SHALL survive the session store round-trip. An 8-bit swatch
SHALL have no float triple and SHALL behave as before.

#### Scenario: Add an HDR swatch

- **WHEN** the HDR picker adds a colour with values above `1.0` to the Swatches panel
- **THEN** the swatch stores the float triple

#### Scenario: Click an HDR swatch

- **WHEN** an HDR swatch is clicked
- **THEN** the foreground HDR state and its `QColor` mirror both restore from that swatch

#### Scenario: HDR swatches persist

- **WHEN** an HDR swatch is added and a new window is created
- **THEN** the swatch still carries its float triple

#### Scenario: Ordinary swatches are unchanged

- **WHEN** an 8-bit swatch is added and clicked
- **THEN** it behaves exactly as before and carries no float triple
