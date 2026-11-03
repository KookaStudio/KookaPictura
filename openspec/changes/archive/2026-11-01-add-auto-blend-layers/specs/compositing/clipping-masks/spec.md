## ADDED Requirements

### Requirement: Clipped adjustments over a soft base

A clipped non-fill adjustment layer SHALL change colour only: the clipping
group's alpha MUST be unchanged by it, and the adjustment SHALL apply at full
strength wherever the base has any coverage. Unclipped adjustments are unaffected
by this requirement.

#### Scenario: An adjustment clipped to a half-transparent base [cla_soft_base]

- **WHEN** the `clipping` unit tests composite an Invert clipped to a base of alpha 0.5, or a base behind a 50 % layer mask
- **THEN** the composite's alpha equals the base's alone and its colour is the fully inverted base colour

#### Scenario: Region composites agree [cla_region]

- **WHEN** a region of such a group is composited serially and in parallel
- **THEN** both match the same region of the full composite
