## ADDED Requirements

### Requirement: Clone Stamp Airbrush

The Clone Stamp options bar SHALL carry an Airbrush toggle, off by default and persisted with
the tool's options. While a stroke is open with Airbrush on, the tool SHALL keep
laying dabs at the pointer at a bounded time-driven rate even when the pointer is
still, accumulating toward the Opacity cap. With Airbrush off, a still pointer SHALL
add no paint. Held dabs SHALL stop when the stroke ends, whether by release, tool
switch, or session cancel, and SHALL NOT add a second history state.

#### Scenario: Airbrush off adds nothing while held [clone_hold_airbrush_off]

- **WHEN** a Clone Stamp stroke with Airbrush off is held still at low Flow
- **THEN** the pixels under the pointer do not change

#### Scenario: Airbrush on builds while held [clone_hold_airbrush_on]

- **WHEN** a Clone Stamp stroke with Airbrush on is held still at low Flow
- **THEN** the pixels under the pointer move toward the source and release records one history state

#### Scenario: Toggle sets and persists the option [clone_hold_airbrush_toggle]

- **WHEN** the Clone Stamp Airbrush toggle is clicked
- **THEN** the tool's option is on and survives the options JSON round trip

#### Scenario: Held dabs stop when the stroke ends [stamp_hold_stops]

- **WHEN** a held Clone Stamp stroke with Airbrush on is released, or a tool switch or session cancel ends it
- **THEN** the pixels stop changing while the event loop keeps running

#### Scenario: A tool switch commits the stroke [stamp_switch_commits]

- **WHEN** the tool changes during a Clone Stamp stroke
- **THEN** the stroke commits as one history state, as with the Brush
