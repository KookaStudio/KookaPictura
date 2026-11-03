# tools/smudge-tool Specification

## Purpose
The Smudge tool: drag colour along a brushed stroke.

## Requirements

### Requirement: Smudge tool

A Smudge drag SHALL carry the pixels under the tip along the stroke: each dab
SHALL lay down, at Strength times the tip's coverage and restricted to the part
of the pixel the Mode allows, the pixels picked up at the previous dab, and
then pick up the result, so the first dab changes nothing and the smear fades
with distance. With Finger Painting the finger SHALL start loaded with the
foreground colour; with Sample All Layers it SHALL pick up from the current
composite — the press-time composite with the active layer's live pixels laid
over it — while the result lands on the active layer, so a dab picks up the
smear the stroke has already laid rather than a frozen press-time image. A
stroke SHALL record exactly one "Smudge" history state when pixels changed; a
transparency-locked layer SHALL keep every pixel's alpha, and a layer whose
pixels are locked SHALL be refused.

#### Scenario: Dragging colour out of a bar

- **WHEN** the `tst_retouch_tools` Smudge test drags from a black bar into white at Strength 90 %
- **THEN** one "Smudge" state is recorded and black is carried into the white along the stroke

#### Scenario: Finger Painting

- **WHEN** the same drag runs over white with Finger Painting on and a red foreground
- **THEN** red is dragged into the white

#### Scenario: Sample All Layers picks up the live canvas [smudge_live_sample]

- **WHEN** a Sample All Layers Smudge drags while painting onto a layer whose pixels differ from the press-time composite
- **THEN** each dab picks up the pixels the stroke has laid so far, not the press-time composite, so the carried colour follows the smear
