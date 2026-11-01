## ADDED Requirements

### Requirement: Art History Brush paints while held

The Art History Brush SHALL keep laying dabs at the pointer at a bounded time-driven
rate while a stroke is open, even when the pointer is still, with no option to turn
this off. Held dabs SHALL stop when the stroke ends and SHALL belong to the stroke's
one history state.

#### Scenario: A held button keeps scattering strokes [art_history_hold]

- **WHEN** an Art History Brush stroke is held still over an area that differs from the source state
- **THEN** more pixels change over time and release records one history state
