# Spec Delta

## ADDED Requirements

### Requirement: Temporary status messages in the status bar

While a temporary status message is displayed the status bar SHALL hide its
readouts and tool-hint field so the message never overprints them, and SHALL
restore them when the message clears, including when a readout refresh runs
while the message is shown. The status bar SHALL paint the temporary message
once, on its own banner, and SHALL NOT also paint the base status-bar message
text.

#### Scenario: A temporary message replaces the readouts [las_status_message_replaces_readouts]

- **WHEN** a temporary status message is shown while a document is open
- **THEN** the magnification, size, backend, view-options, action, and tool-hint readouts are hidden and the message banner intersects no visible readout
- **AND WHEN** the message clears
- **THEN** the readouts return

#### Scenario: A refresh does not bring readouts over a message [las_status_message_survives_refresh]

- **WHEN** a status readout refresh runs (for example a canvas zoom change) while a temporary status message is shown
- **THEN** the readouts stay hidden and the message banner keeps the footer to itself
