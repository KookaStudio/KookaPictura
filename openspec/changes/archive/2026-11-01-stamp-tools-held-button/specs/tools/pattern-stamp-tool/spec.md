## ADDED Requirements

### Requirement: Pattern Stamp Airbrush

The Pattern Stamp options bar SHALL carry an Airbrush toggle, off by default and persisted with
the tool's options. While a stroke is open with Airbrush on, the tool SHALL keep
laying dabs at the pointer at a bounded time-driven rate even when the pointer is
still, accumulating toward the Opacity cap. With Airbrush off, a still pointer SHALL
add no paint. Held dabs SHALL stop when the stroke ends, whether by release, tool
switch, or session cancel, and SHALL NOT add a second history state.

#### Scenario: Airbrush off adds nothing while held [pattern_hold_airbrush_off]

- **WHEN** a Pattern Stamp stroke with Airbrush off is held still at low Flow
- **THEN** the pixels under the pointer do not change

#### Scenario: Airbrush on builds while held [pattern_hold_airbrush_on]

- **WHEN** a Pattern Stamp stroke with Airbrush on is held still at low Flow
- **THEN** the pixels under the pointer move toward the source and release records one history state

#### Scenario: Toggle sets and persists the option [pattern_hold_airbrush_toggle]

- **WHEN** the Pattern Stamp Airbrush toggle is clicked
- **THEN** the tool's option is on and survives the options JSON round trip
