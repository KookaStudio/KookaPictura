## ADDED Requirements

### Requirement: Blur tool

A Blur drag SHALL soften the active pixel layer under the brush tip, moving
each covered pixel toward its 3×3 neighbourhood average by Strength times the
tip's coverage, restricted to the part of the pixel the Mode allows, and SHALL
work on what the previous dab left so that going over a spot again softens it
further. With Sample All Layers the neighbourhood SHALL be read from the
composite while the result lands on the active layer. A stroke SHALL record
exactly one "Blur" history state when pixels changed; a transparency-locked
layer SHALL keep every pixel's alpha, and a layer whose pixels are locked
SHALL be refused.

#### Scenario: Softening an edge

- **WHEN** the `tst_retouch_tools` Blur test drags along a black/white edge at Strength 100 %
- **THEN** one "Blur" state is recorded, the pixels either side of the edge move toward each other, and pixels outside the tip are unchanged

#### Scenario: Sample All Layers on an empty layer

- **WHEN** the same drag runs on an empty layer, first without and then with Sample All Layers
- **THEN** the first records no state and the second records one "Blur" state
