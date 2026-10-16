## ADDED Requirements

### Requirement: Dab scatter and count

A Paint stroke SHALL lay `count` dabs (1–16) at each spacing step, each offset
by up to `scatter` percent of the diameter in both axes. With a count of 1 and no
scatter or jitter the stroke SHALL paint exactly as without dynamics. The random
offsets SHALL come from the stroke's fixed seed, so the same stroke replays
exactly.

#### Scenario: A scattered cluster

- **WHEN** one step is painted with scatter 300 % and count 12
- **THEN** the ink spreads more than twice as wide as a plain dab and covers more than three times the pixels

#### Scenario: Dynamics replay

- **WHEN** the same jittered, scattered stroke is painted twice
- **THEN** both strokes cover the same pixels

### Requirement: Shape jitter

Each dab SHALL be shrunk by up to `size_jitter` percent (never enlarged),
turned by up to plus or minus `angle_jitter` degrees, and flattened by up to
`roundness_jitter` percent.

#### Scenario: Size jitter only shrinks

- **WHEN** a dab is painted with 90 % size jitter
- **THEN** it is no wider than the same dab without jitter
